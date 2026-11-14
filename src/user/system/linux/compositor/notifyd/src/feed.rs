//! The notifications' history, and the line socket the shell reads it
//! from (`docs/DESKTOP.md` §4.2): `list` answers the history newest
//! first, `subscribe` keeps the connection open with the history and
//! then the new notices as they come, and `close <id>` takes one away.
//!
//! The feed itself is pure: what is kept, and what is closed. The socket
//! is a thread that serves it, one thread per connection.
//!
//! The lines it carries: a notice is `notice <id> <app>\t<summary>\t<body>`,
//! the do-not-disturb is `quiet on` or `quiet off`, and `live` ends the
//! history — what follows it is new, and only that is bannered.

use std::collections::VecDeque;
use std::io::{self, Read as _, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::store::{HISTORY, Note, Urgency};

/// How often a subscriber looks for the new notices.
const POLL: Duration = Duration::from_millis(200);

/// The line that ends the history: what follows it is new.
const LIVE: &str = "live";
/// The line that says the do-not-disturb is on.
// Ponytail: the do-not-disturb travels as a line of its own, so the shell
// lists the notices but does not banner them. Upgrade path: a flag on
// every notice, when a quiet notice and a loud one must be told apart.
const QUIET_ON: &str = "quiet on";
/// The line that says the do-not-disturb is off.
const QUIET_OFF: &str = "quiet off";
/// The line that answers a command that is not one.
const ERROR: &str = "error unknown command";

/// The history: the notes kept, oldest first, each with the number the
/// feed gave it.
#[derive(Debug)]
pub struct Feed {
    cap: usize,
    sequence: u64,
    items: VecDeque<(u64, Note)>,
    quiet: bool,
}

impl Default for Feed {
    fn default() -> Self {
        Feed::new(HISTORY)
    }
}

impl Feed {
    /// A feed that keeps `cap` notes.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Feed {
            cap,
            sequence: 0,
            items: VecDeque::new(),
            quiet: false,
        }
    }

    /// A note of its own, the plainest there is.
    pub fn push(&mut self, id: u32, app: &str, summary: &str, body: &str) {
        self.put(Note {
            id,
            app: app.to_owned(),
            summary: summary.to_owned(),
            body: body.to_owned(),
            actions: Vec::new(),
            urgency: Urgency::Normal,
            until_ms: None,
            key: None,
            count: 1,
            on_bus: false,
        });
    }

    /// A note as it is: one under the same id replaces the old.
    pub fn put(&mut self, note: Note) {
        self.sequence = self.sequence.wrapping_add(1);
        let sequence = self.sequence;
        self.items.retain(|(_, kept)| kept.id != note.id);
        self.items.push_back((sequence, note));
        while self.items.len() > self.cap {
            let _ = self.items.pop_front();
        }
    }

    /// The history, newest first, at most `cap` of it.
    #[must_use]
    pub fn list(&self, cap: usize) -> Vec<Note> {
        self.items
            .iter()
            .rev()
            .take(cap)
            .map(|(_, note)| note.clone())
            .collect()
    }

    /// The notes after the number `sequence`, newest first, with their
    /// own numbers.
    // Ponytail: the feed's own numbers, not the notices' ids. Upgrade
    // path: none needed — a notice may take an id that is older than the
    // one before it, and the numbers keep the order right.
    #[must_use]
    pub fn after(&self, sequence: u64) -> Vec<(u64, Note)> {
        self.items
            .iter()
            .rev()
            .take_while(|(at, _)| *at > sequence)
            .cloned()
            .collect()
    }

    /// The number of the last note put in.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Take a note away; whether it was there.
    pub fn close(&mut self, id: u32) -> bool {
        let before = self.items.len();
        self.items.retain(|(_, kept)| kept.id != id);
        self.items.len() != before
    }

    /// Forget them all.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Whether the do-not-disturb is on: the shell is told, so that it
    /// lists the notices but does not banner them.
    #[must_use]
    pub fn quiet(&self) -> bool {
        self.quiet
    }

    /// The do-not-disturb, on or off.
    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }
}

/// The feed, as it is shared with the socket's threads.
pub type Shared = Arc<Mutex<Feed>>;

/// The feed's lock, even when a thread panicked holding it.
fn locked(feed: &Shared) -> MutexGuard<'_, Feed> {
    feed.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Where the socket is: the runtime folder, else `/tmp` by the person's
/// own number.
#[must_use]
pub fn socket_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|dir| !dir.is_empty())
    {
        return Some(Path::new(&dir).join("notifyd.sock"));
    }
    use std::os::unix::fs::MetadataExt as _;
    let uid = std::fs::metadata("/proc/self").ok()?.uid();
    Some(PathBuf::from(format!("/tmp/notifyd-{uid}.sock")))
}

/// The socket's listener, at `path`.
pub fn bind(path: &Path) -> io::Result<UnixListener> {
    let _ = std::fs::remove_file(path);
    UnixListener::bind(path)
}

/// The socket's server: a thread per connection, until the listener is
/// gone. `subscribers` counts those that watch for the new notices, so
/// that the daemon knows it is not the only one drawing them.
pub fn serve(path: &Path, feed: Shared, subscribers: Arc<AtomicUsize>) -> io::Result<()> {
    let listener = bind(path)?;
    accept(listener, feed, subscribers)
}

/// The connections, one thread each.
fn accept(listener: UnixListener, feed: Shared, subscribers: Arc<AtomicUsize>) -> io::Result<()> {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let feed = Arc::clone(&feed);
        let subscribers = Arc::clone(&subscribers);
        let spawned = std::thread::Builder::new()
            .name("notifyd-feed".to_owned())
            .spawn(move || {
                let _ = connection(stream, feed, subscribers);
            });
        if spawned.is_err() {
            return Err(io::Error::other(
                "a thread for the socket could not be made",
            ));
        }
    }
    Ok(())
}

/// One connection: the command it carries, and what it is told.
fn connection(
    mut stream: UnixStream,
    feed: Shared,
    subscribers: Arc<AtomicUsize>,
) -> io::Result<()> {
    let said = said(&mut stream)?;
    if said == "subscribe" {
        let _ = subscribers.fetch_add(1, Ordering::SeqCst);
        let result = subscribe(&mut stream, &feed);
        let _ = subscribers.fetch_sub(1, Ordering::SeqCst);
        return result;
    }
    if let Some(rest) = said.strip_prefix("close ") {
        if let Ok(id) = rest.trim().parse() {
            let _ = locked(&feed).close(id);
        }
        return Ok(());
    }
    if said == "list" {
        for note in locked(&feed).list(HISTORY) {
            stream.write_all(line(&note).as_bytes())?;
        }
        return Ok(());
    }
    // A command that is not one is said so, not answered with the
    // history.
    stream.write_all(format!("{ERROR}\n").as_bytes())
}

/// A subscriber: the do-not-disturb, the history, the end of the
/// history, and then every new note as it comes — until the shell goes
/// away. The commands it carries (`close <id>`) are answered as they
/// come.
fn subscribe(stream: &mut UnixStream, feed: &Shared) -> io::Result<()> {
    let mut quiet = locked(feed).quiet();
    stream.write_all(quiet_line(quiet).as_bytes())?;
    for note in locked(feed).list(HISTORY) {
        stream.write_all(line(&note).as_bytes())?;
    }
    // The history ends here: what follows is new.
    stream.write_all(format!("{LIVE}\n").as_bytes())?;
    // A short read is how the shell's going away is seen.
    stream.set_read_timeout(Some(Duration::from_millis(1)))?;
    let mut sequence = locked(feed).sequence();
    let mut said = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(count) => said.extend_from_slice(chunk.get(..count).unwrap_or_default()),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error),
        }
        // The shell's commands, on the connection it watches with.
        while let Some(end) = said.iter().position(|byte| *byte == b'\n') {
            let command: Vec<u8> = said.drain(..=end).collect();
            let command = String::from_utf8_lossy(&command);
            let command = command.trim();
            if let Some(rest) = command.strip_prefix("close ")
                && let Ok(id) = rest.trim().parse()
            {
                let _ = locked(feed).close(id);
            }
        }
        // The new notes.
        for (at, note) in locked(feed).after(sequence) {
            sequence = sequence.max(at);
            stream.write_all(line(&note).as_bytes())?;
        }
        // The do-not-disturb, when it changed.
        let now = locked(feed).quiet();
        if now != quiet {
            quiet = now;
            stream.write_all(quiet_line(quiet).as_bytes())?;
        }
        std::thread::sleep(POLL);
    }
}

/// The line the connection carries, up to its newline.
fn said(stream: &mut UnixStream) -> io::Result<String> {
    let mut got = Vec::new();
    let mut chunk = [0_u8; 128];
    loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        got.extend_from_slice(chunk.get(..count).unwrap_or_default());
        if got.contains(&b'\n') {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&got).trim().to_owned())
}

/// A note as the line carries it: `notice <id> <app>\t<summary>\t<body>`.
fn line(note: &Note) -> String {
    format!(
        "notice {} {}\t{}\t{}\n",
        note.id,
        label(&note.app),
        label(&note.summary),
        label(&note.body)
    )
}

/// The do-not-disturb as the line carries it.
fn quiet_line(quiet: bool) -> String {
    format!("{}\n", if quiet { QUIET_ON } else { QUIET_OFF })
}

/// A label as the line carries it: no tab, no newline.
fn label(text: &str) -> String {
    text.chars()
        .map(|letter| match letter {
            '\t' | '\n' | '\r' => ' ',
            letter => letter,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed() -> Feed {
        let mut feed = Feed::new(50);
        feed.push(1, "Files", "Copy done", "notes.md to the stash");
        feed.push(2, "devmgr", "GPU driver is up", "");
        feed
    }

    /// A socket of its own, with the feed and the thread that serves it.
    fn serving(name: &str) -> (PathBuf, Shared, std::thread::JoinHandle<()>) {
        let feed: Shared = Arc::new(Mutex::new(feed()));
        let subscribers = Arc::new(AtomicUsize::new(0));
        let path =
            std::env::temp_dir().join(format!("notifyd-feed-{}-{name}.sock", std::process::id()));
        let listener = bind(&path).expect("the socket");
        let served = Arc::clone(&feed);
        let thread = std::thread::spawn(move || {
            let _ = accept(listener, served, Arc::clone(&subscribers));
        });
        (path, feed, thread)
    }

    #[test]
    fn the_list_answers_newest_first_up_to_the_cap() {
        assert_eq!(feed().list(50).len(), 2);
        assert_eq!(feed().list(50)[0].summary, "GPU driver is up");
        assert_eq!(feed().list(1).len(), 1, "the cap cuts");
    }

    #[test]
    fn fifty_are_kept_and_the_oldest_fall_off() {
        let mut feed = Feed::new(50);
        for id in 1..=51 {
            feed.push(id, "app", &format!("n{id}"), "");
        }
        assert_eq!(feed.list(50).len(), 50);
        assert_eq!(
            feed.list(50)[49].summary,
            "n2",
            "oldest first at the end: n1 has fallen off"
        );
        assert_eq!(feed.list(50)[0].summary, "n51");
    }

    #[test]
    fn a_close_removes_one() {
        let mut feed = feed();
        assert!(feed.close(2));
        assert!(!feed.close(2));
        assert_eq!(feed.list(50).len(), 1);
    }

    /// A note under the same id takes the old one's place, and the line
    /// carries no tab or newline in its labels.
    #[test]
    fn a_note_replaces_its_own_and_the_line_is_one_line() {
        let mut feed = feed();
        feed.push(1, "Files", "Copy done again", "notes.md to the stash");
        feed.push(2, "devmgr", "GPU\tdriver\tis up", "");
        let notes = feed.list(50);
        assert_eq!(notes.len(), 2, "the same id, once");
        assert_eq!(
            notes.last().map(|note| note.summary.as_str()),
            Some("Copy done again")
        );
        assert_eq!(
            notes.first().map(line),
            Some("notice 2 devmgr\tGPU driver is up\t\n".to_owned())
        );
        assert_eq!(label("a\tb\nc"), "a b c");
    }

    /// The socket answers `list` with the history, one line per notice,
    /// newest first.
    #[test]
    fn the_socket_answers_the_history() {
        let (path, _feed, _thread) = serving("list");
        let mut stream = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(stream, "list");
        let mut said = String::new();
        let _ = stream.read_to_string(&mut said);
        assert_eq!(
            said,
            "notice 2 devmgr\tGPU driver is up\t\n\
             notice 1 Files\tCopy done\tnotes.md to the stash\n"
        );
    }

    /// A command that is not one is said so, and not answered with the
    /// history.
    #[test]
    fn an_unknown_command_is_refused() {
        let (path, _feed, _thread) = serving("unknown");
        let mut stream = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(stream, "nonsense");
        let mut said = String::new();
        let _ = stream.read_to_string(&mut said);
        assert_eq!(said, "error unknown command\n");
        assert!(!said.contains("notice"), "not the history");
    }

    /// A subscriber gets the do-not-disturb, the history, the end of the
    /// history, and then the new notices as they come.
    #[test]
    fn a_subscriber_gets_the_new_notices() {
        let (path, feed, thread) = serving("subscribe");
        let mut stream = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(stream, "subscribe");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut buf = [0_u8; 256];
        let mut said = String::new();
        while !said.contains(LIVE) {
            let count = stream.read(&mut buf).unwrap_or(0);
            if count == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(
                buf.get(..count).unwrap_or_default(),
            ));
        }
        assert!(said.starts_with(QUIET_OFF), "the quiet state first: {said}");
        assert!(said.contains("notice 2 devmgr\tGPU driver is up\t\n"));
        assert!(said.contains("notice 1 Files\tCopy done\tnotes.md to the stash\n"));
        assert!(said.contains(LIVE), "the end of the history");
        locked(&feed).push(3, "Files", "Copy done", "notes.md");
        let mut said = String::new();
        while !said.contains("Copy done") {
            let count = stream.read(&mut buf).unwrap_or(0);
            if count == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(
                buf.get(..count).unwrap_or_default(),
            ));
        }
        assert!(
            said.contains("notice 3 Files\tCopy done\tnotes.md\n"),
            "the new notice: {said}"
        );
        // The thread that serves the socket ends with the program.
        drop(thread);
    }

    /// The do-not-disturb is told to the subscriber, so that it lists the
    /// notices but does not banner them.
    #[test]
    fn the_do_not_disturb_is_told_to_the_subscriber() {
        let (path, feed, thread) = serving("quiet");
        let mut stream = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(stream, "subscribe");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut buf = [0_u8; 256];
        let mut said = String::new();
        while !said.contains(LIVE) {
            let count = stream.read(&mut buf).unwrap_or(0);
            if count == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(
                buf.get(..count).unwrap_or_default(),
            ));
        }
        assert!(said.starts_with(QUIET_OFF), "the quiet state first: {said}");
        locked(&feed).set_quiet(true);
        let mut said = String::new();
        while !said.contains(QUIET_ON) {
            let count = stream.read(&mut buf).unwrap_or(0);
            if count == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(
                buf.get(..count).unwrap_or_default(),
            ));
        }
        assert!(said.contains(QUIET_ON), "the quiet state is told: {said}");
        // The thread that serves the socket ends with the program.
        drop(thread);
    }

    /// A close on the connection the shell watches with reaches the
    /// history: the notice is gone from the next `list`.
    #[test]
    fn a_close_over_the_socket_reaches_the_history() {
        let (path, feed, thread) = serving("close");
        let mut stream = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(stream, "subscribe");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        // The history and the end of it.
        let mut buf = [0_u8; 256];
        let mut said = String::new();
        while !said.contains(LIVE) {
            let count = stream.read(&mut buf).unwrap_or(0);
            if count == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(
                buf.get(..count).unwrap_or_default(),
            ));
        }
        // The close, on the same connection.
        let _ = writeln!(stream, "close 2");
        let mut gone = false;
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(20));
            if !locked(&feed).list(HISTORY).iter().any(|note| note.id == 2) {
                gone = true;
                break;
            }
        }
        assert!(gone, "the notice is out of the history");
        // And the next `list` does not carry it.
        let mut other = UnixStream::connect(&path).expect("the socket");
        let _ = writeln!(other, "list");
        let mut said = String::new();
        let _ = other.read_to_string(&mut said);
        assert!(!said.contains("notice 2"), "not in the list: {said}");
        drop(stream);
        drop(thread);
    }
}
