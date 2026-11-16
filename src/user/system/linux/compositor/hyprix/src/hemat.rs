//! Hemat: the desktop's memory saving, one cgroup for each app.
//!
//! A desktop running many apps holds the memory of all of them at once,
//! while most of them are looked at now and then. Hemat is the part of the
//! desktop that lets the ones in the background have less: each app gets a
//! cgroup of its own, an app in the background is given less of the
//! processor, and an app that has been in the background for five minutes
//! is put to sleep, which is `memory.reclaim` -- the clean pages it holds
//! are given back.
//!
//! # The tree
//!
//! Hemat's groups live in a tree of their own, `/sys/fs/cgroup/hemat`,
//! rather than below the session's cgroup: the session's cgroup holds the
//! compositor itself, and cgroup v2 does not let a cgroup that holds
//! processes have controllers enabled for its children
//! (`docs/CGROUPS.md` §3.1). The tree is made here, and mounted if it is
//! not mounted yet, because the desktop's boot has no init to mount it
//! (`docs/AUTH.md` §1.2).
//!
//! # Where an app is
//!
//! An app is **Aktif** while one of its windows is on the screen, **Latar**
//! while none of them is, and **Tidur** when it has been in the background
//! for five minutes. A sleeping app is given back its clean pages; a
//! background app is given a tenth of the processor an active one has. An
//! app that is making a sound never sleeps.
//!
//! # Failing
//!
//! Nothing here may take the desktop with it. A mount that is refused, a
//! cgroup that cannot be made, a file that cannot be written: each is
//! reported in a sentence and left behind, and the app it was for keeps
//! running as it was (`docs/DESKTOP.md` §7).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Where the cgroup v2 tree is mounted.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// The tree the hemat's groups live in, of its own.
pub const TREE: &str = "hemat";

/// The controllers the root of the tree has: the three Ferrix has.
pub const CONTROLLERS: [&str; 3] = ["+cpu", "+memory", "+pids"];

/// The controllers the hemat's tree has for its own children.
pub const TREE_CONTROLLERS: [&str; 2] = ["+cpu", "+memory"];

/// How long an app may be in the background before it is put to sleep.
pub const SLEEP_AFTER: Duration = Duration::from_secs(300);

/// What is given back when an app falls asleep: its clean pages, a gigabyte
/// of them at a time.
pub const RECLAIM: &str = "1G";

/// The processor weight of an app that is on the screen.
pub const WEIGHT_ACTIVE: &str = "100";

/// The processor weight of an app in the background.
pub const WEIGHT_BACKGROUND: &str = "10";

/// Where an app is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// One of its windows is on the screen.
    Aktif,
    /// None of its windows is.
    Latar,
    /// It has been in the background for five minutes.
    Tidur,
}

/// An app, as the hemat knows it.
#[derive(Clone, Debug)]
pub struct App {
    /// The class it is known by, as its client named it.
    pub class: String,
    /// The name of its cgroup, made safe.
    pub name: String,
    /// Its number in the hemat's tree.
    pub number: u32,
    /// Its cgroup.
    pub dir: PathBuf,
    /// The process behind its windows, or 0 where the kernel would not say.
    pub pid: u32,
    /// Where it is.
    pub state: State,
    /// Since when it was last seen on the screen: the background is
    /// counted from there.
    pub since_background: Option<Instant>,
    /// Whether it is making a sound.
    pub loud: bool,
    /// The process already moved into its cgroup.
    pub moved: u32,
}

/// A window, as the hemat sees it.
#[derive(Clone, Debug)]
pub struct Window {
    /// Which window.
    pub id: u64,
    /// The class it is, which is the app it belongs to.
    pub class: String,
    /// Its process, or 0 where the kernel would not say.
    pub pid: u32,
    /// Whether the screen shows it.
    pub visible: bool,
}

/// One thing to write in a cgroup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// The app it is for.
    pub class: String,
    /// The cgroup it is for.
    pub dir: PathBuf,
    /// The file to write.
    pub file: &'static str,
    /// What to write.
    pub value: String,
}

/// The desktop's memory saving.
#[derive(Debug)]
pub struct Hemat {
    /// The apps, by class.
    pub apps: BTreeMap<String, App>,
    /// Whether the hemat is on at all.
    pub enabled: bool,
    /// The classes that never sleep.
    pub exceptions: Vec<String>,
    /// The tree the groups are made under.
    pub root: PathBuf,
    /// The number the next app is given.
    next: u32,
    /// The apps whose writing has failed, so that it is said once.
    failed: BTreeSet<String>,
}

impl Default for Hemat {
    fn default() -> Self {
        Self::new(true, Vec::new())
    }
}

impl Hemat {
    /// A hemat that is on, with the classes that never sleep.
    #[must_use]
    pub fn new(enabled: bool, exceptions: Vec<String>) -> Self {
        Self {
            apps: BTreeMap::new(),
            enabled,
            exceptions,
            root: PathBuf::from(CGROUP_ROOT),
            next: 0,
            failed: BTreeSet::new(),
        }
    }

    /// A hemat whose groups are made under `root`, which is a directory in
    /// a test and the mount in a desktop.
    #[must_use]
    pub fn at(root: PathBuf, enabled: bool, exceptions: Vec<String>) -> Self {
        Self {
            root,
            ..Self::new(enabled, exceptions)
        }
    }

    /// Turn the hemat off: no group is made and no app is watched.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.apps.clear();
    }

    /// Where the app of the class `class` is, if the hemat knows it.
    #[must_use]
    pub fn state_of(&self, class: &str) -> Option<State> {
        self.apps.get(class).map(|app| app.state)
    }

    /// Whether the class `class` is on the exception list.
    #[must_use]
    pub fn excluded(&self, class: &str) -> bool {
        self.exceptions
            .iter()
            .any(|exception| same_class(exception, class))
    }

    /// The app of the class `class` is making a sound, or is not.
    ///
    /// The name is the one the sound server gives, which is the binary's
    /// name; it is matched against the class loosely, because the class
    /// and the binary do not always have the same name.
    pub fn heard(&mut self, name: &str, loud: bool) {
        let classes: Vec<String> = self.apps.keys().cloned().collect();
        for class in classes {
            if same_class(name, &class)
                && let Some(app) = self.apps.get_mut(&class)
            {
                app.loud = loud;
            }
        }
    }

    /// Forget the app of the class `class`, and its group with it.
    pub fn forget(&mut self, class: &str) {
        if let Some(app) = self.apps.remove(class) {
            let _ = remove_app(&app.dir);
            let _ = self.failed.remove(class);
        }
    }

    /// Look at every window there is, and say where each app is.
    ///
    /// The writes the change asks for are returned rather than done: a
    /// test can see what would be written without a cgroup to write it
    /// in, and the compositor is the one that does them.
    pub fn refresh(&mut self, windows: &[Window], now: Instant) -> Vec<Effect> {
        let mut effects = Vec::new();
        let classes = by_class(windows);
        self.forget_gone(&classes);
        for (class, of_class) in &classes {
            effects.extend(self.refresh_class(class, of_class, now));
        }
        effects
    }

    /// Write what `effects` says, and say once per app what could not be
    /// written.
    pub fn apply(&mut self, effects: &[Effect]) -> Vec<String> {
        let mut said = Vec::new();
        for effect in effects {
            if let Err(why) = write(&effect.dir, effect.file, &effect.value)
                && self.failed.insert(effect.class.clone())
            {
                said.push(why);
            }
        }
        said
    }

    /// The apps that are asleep, and the ones in the background, as the
    /// dispatcher says them.
    #[must_use]
    pub fn describe(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .apps
            .values()
            .map(|app| format!("hemat: {} {} {:?}", app.class, app.name, app.state))
            .collect();
        lines.push(format!(
            "hemat: the apps are asleep: {}",
            self.apps
                .values()
                .filter(|app| app.state == State::Tidur)
                .count()
        ));
        lines
    }

    /// Forget the apps that have no window left.
    fn forget_gone(&mut self, classes: &BTreeMap<String, Vec<&Window>>) {
        let gone: Vec<String> = self
            .apps
            .keys()
            .filter(|class| !classes.contains_key(class.as_str()))
            .cloned()
            .collect();
        for class in gone {
            self.forget(&class);
        }
    }

    /// Look at the windows of one app.
    fn refresh_class(&mut self, class: &str, windows: &[&Window], now: Instant) -> Vec<Effect> {
        let mut effects = Vec::new();
        let visible = windows.iter().any(|window| window.visible);
        let pid = windows
            .iter()
            .find_map(|window| (window.pid != 0).then_some(window.pid))
            .unwrap_or(0);
        if !self.apps.contains_key(class) {
            effects.extend(self.make_app(class, pid));
        }
        if let Some(app) = self.apps.get_mut(class)
            && app.pid == 0
        {
            app.pid = pid;
        }
        effects.extend(self.move_in(class));
        if let Some(state) = self.change_state(class, visible, now) {
            effects.extend(self.effects_for(class, state));
        }
        effects
    }

    /// Make the group of an app that is new.
    fn make_app(&mut self, class: &str, pid: u32) -> Vec<Effect> {
        if !self.enabled || pid == 0 {
            return Vec::new();
        }
        let name = safe(class);
        self.next += 1;
        let number = self.next;
        let dir = self.root.join(TREE).join(format!("app-{name}-{number}"));
        let mut effects = Vec::new();
        if fs::create_dir_all(&dir).is_ok() {
            effects.push(self.effect(class, &dir, "memory.high", "max"));
            effects.push(self.effect(class, &dir, "cpu.weight", WEIGHT_ACTIVE));
        }
        let _ = self.apps.insert(
            class.to_owned(),
            App {
                class: class.to_owned(),
                name,
                number,
                dir,
                pid,
                state: State::Aktif,
                since_background: None,
                loud: false,
                moved: 0,
            },
        );
        effects
    }

    /// Move the app's process into its group, if it is not there yet.
    fn move_in(&mut self, class: &str) -> Vec<Effect> {
        let Some(app) = self.apps.get(class) else {
            return Vec::new();
        };
        if app.pid == 0 || app.moved == app.pid {
            return Vec::new();
        }
        let effect = self.effect(class, &app.dir, "cgroup.procs", &app.pid.to_string());
        if let Some(app) = self.apps.get_mut(class) {
            app.moved = app.pid;
        }
        vec![effect]
    }

    /// Say where the app of the class `class` is now.
    fn change_state(&mut self, class: &str, visible: bool, now: Instant) -> Option<State> {
        let loud = self.apps.get(class).is_some_and(|app| app.loud);
        let excluded = self.excluded(class);
        let app = self.apps.get_mut(class)?;
        let before = app.state;
        if visible {
            app.since_background = Some(now);
            app.state = State::Aktif;
        } else {
            let since = *app.since_background.get_or_insert(now);
            app.state = if loud || excluded {
                State::Latar
            } else if now.duration_since(since) >= SLEEP_AFTER {
                State::Tidur
            } else {
                State::Latar
            };
        }
        (before != app.state).then_some(app.state)
    }

    /// What the app's new state means for its cgroup.
    fn effects_for(&self, class: &str, state: State) -> Vec<Effect> {
        let Some(app) = self.apps.get(class) else {
            return Vec::new();
        };
        let weight = if state == State::Aktif || self.excluded(class) {
            WEIGHT_ACTIVE
        } else {
            WEIGHT_BACKGROUND
        };
        let mut effects = vec![self.effect(class, &app.dir, "cpu.weight", weight)];
        match state {
            State::Aktif => effects.push(self.effect(class, &app.dir, "memory.high", "max")),
            State::Tidur => effects.push(self.effect(class, &app.dir, "memory.reclaim", RECLAIM)),
            State::Latar => {}
        }
        effects
    }

    /// One write to be done in the app's group.
    fn effect(&self, class: &str, dir: &Path, file: &'static str, value: &str) -> Effect {
        Effect {
            class: class.to_owned(),
            dir: dir.to_path_buf(),
            file,
            value: value.to_owned(),
        }
    }
}

/// What the shell says the compositor: `sound <nama>`, `quiet <nama>` or
/// `state`.
///
/// The first two say that an app is making a sound, or is not, which keeps
/// it from falling asleep; the third says where every app is, for a
/// diagnosis. The lines are what the compositor should say back.
#[must_use]
pub fn dispatch(argument: &str, hemat: &mut Hemat) -> (bool, Vec<String>) {
    let mut words = argument.split_whitespace();
    let loud = match words.next() {
        Some("sound") => true,
        Some("quiet") => false,
        Some("state") => return (false, hemat.describe()),
        _ => {
            return (
                false,
                vec!["hemat: want sound <name>, quiet <name> or state".to_owned()],
            );
        }
    };
    let Some(name) = words.next() else {
        return (
            false,
            vec!["hemat: want sound <name> or quiet <name>".to_owned()],
        );
    };
    hemat.heard(name, loud);
    (false, Vec::new())
}

/// The classes that never sleep, from a list separated by commas.
#[must_use]
pub fn exceptions(text: &str) -> Vec<String> {
    text.split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_owned())
        .collect()
}

/// Whether two names are the same app's, as far as the hemat is concerned.
///
/// The class is what the client said, and the sound server's name is the
/// binary's; they are the same app when they are equal, or when the class
/// ends with the name after a dot.
#[must_use]
pub fn same_class(one: &str, other: &str) -> bool {
    let one = one.to_lowercase();
    let other = other.to_lowercase();
    if one == other {
        return true;
    }
    other.rsplit('.').next().is_some_and(|last| last == one)
        || one.rsplit('.').next().is_some_and(|last| last == other)
}

/// The windows of each app, by class.
fn by_class(windows: &[Window]) -> BTreeMap<String, Vec<&Window>> {
    let mut classes: BTreeMap<String, Vec<&Window>> = BTreeMap::new();
    for window in windows {
        if window.class.is_empty() {
            continue;
        }
        classes
            .entry(window.class.clone())
            .or_default()
            .push(window);
    }
    classes
}

/// The hemat's tree.
#[must_use]
pub fn tree() -> PathBuf {
    Path::new(CGROUP_ROOT).join(TREE)
}

/// The cgroup of the app called `name`, the `number`th one of its class.
#[must_use]
pub fn app_dir(name: &str, number: u32) -> PathBuf {
    tree().join(format!("app-{name}-{number}"))
}

/// A class name made safe for a cgroup's name.
///
/// A cgroup's name takes letters, digits and `:_.-`; anything else is
/// `_`, as in `crate::scope::name`. An empty name is `program`, so that
/// there is always something to make a directory of.
#[must_use]
pub fn safe(class: &str) -> String {
    let base: String = class
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    if base.is_empty() {
        "program".to_owned()
    } else {
        base
    }
}

/// Write `value` to the file `file` of the cgroup `dir`.
///
/// # Errors
///
/// When the file cannot be written, with a sentence saying which file.
pub fn write(dir: &Path, file: &str, value: &str) -> Result<(), String> {
    write_with(dir, file, value, |path, text| {
        let mut handle = fs::OpenOptions::new().write(true).open(path)?;
        handle.write_all(text.as_bytes())
    })
}

/// Write `value` to the file `file` of the cgroup `dir`, with `put`.
///
/// The write itself is `put`, so that a test can hand in one that fails and
/// see that the failure is a sentence and not a panic.
///
/// # Errors
///
/// When `put` fails, with a sentence saying which file.
pub fn write_with<F>(dir: &Path, file: &str, value: &str, put: F) -> Result<(), String>
where
    F: FnOnce(&Path, &str) -> io::Result<()>,
{
    let at = dir.join(file);
    put(&at, value).map_err(|error| format!("{}: {error}", at.display()))
}

/// Mount the cgroup v2 tree if it is not mounted, and make the hemat's.
///
/// # Errors
///
/// When the tree cannot be mounted, or its controllers cannot be enabled.
pub fn prepare() -> Result<(), String> {
    let root = Path::new(CGROUP_ROOT);
    if !root.join("cgroup.controllers").exists() {
        mount(root)?;
    }
    write(root, "cgroup.subtree_control", &CONTROLLERS.join(" "))?;
    let tree = tree();
    fs::create_dir_all(&tree).map_err(|error| format!("{}: {error}", tree.display()))?;
    write(&tree, "cgroup.subtree_control", &TREE_CONTROLLERS.join(" "))?;
    Ok(())
}

/// Mount cgroup v2 at the root.
///
/// # Errors
///
/// When the mount is refused for any reason but the tree being there
/// already.
fn mount(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let source = CString::new("cgroup2").map_err(|_| "cgroup2 is not a name".to_owned())?;
    let target =
        CString::new(CGROUP_ROOT).map_err(|_| "the mount point is not a name".to_owned())?;
    // SAFETY: both names are NUL-terminated C strings that live for the
    // call, and the mount takes no data.
    let code = unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            source.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if code == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    // The tree being mounted already is not a failure: the desktop's boot
    // has an init that mounts it (`src/user/system/linux/init`).
    if error.raw_os_error() == Some(libc::EBUSY) {
        return Ok(());
    }
    Err(format!("mounting cgroup2: {error}"))
}

/// Make the cgroup of the app called `name`, the `number`th of its class.
///
/// The group is made under the tree at `root`, which is the mount in a
/// desktop and a directory in a test.
///
/// # Errors
///
/// When the directory cannot be made.
pub fn make_app_in(root: &Path, name: &str, number: u32) -> Result<PathBuf, String> {
    let dir = root.join(TREE).join(format!("app-{name}-{number}"));
    fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    Ok(dir)
}

/// Make the cgroup of the app called `name`, the `number`th of its class.
///
/// # Errors
///
/// When the directory cannot be made.
pub fn make_app(name: &str, number: u32) -> Result<PathBuf, String> {
    make_app_in(Path::new(CGROUP_ROOT), name, number)
}

/// Put the process `pid` in the cgroup `dir`.
///
/// # Errors
///
/// When the process cannot be moved there.
pub fn put_pid(dir: &Path, pid: u32) -> Result<(), String> {
    write(dir, "cgroup.procs", &pid.to_string())
}

/// Remove the cgroup `dir`.
///
/// A cgroup that still holds a process cannot be removed, and that is not
/// an error: the group stays until the process in it has gone.
///
/// # Errors
///
/// When the directory cannot be removed for any other reason.
pub fn remove_app(dir: &Path) -> Result<(), String> {
    match fs::remove_dir(dir) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(format!("{}: {error}", dir.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: u64, class: &str, pid: u32, visible: bool) -> Window {
        Window {
            id,
            class: class.to_owned(),
            pid,
            visible,
        }
    }

    /// A number that no other test has, so that the tests do not share
    /// a directory and pull each other's groups away.
    fn number() -> u32 {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// A hemat whose groups are made in a directory of its own.
    fn hemat(enabled: bool, exceptions: &[&str]) -> (Hemat, PathBuf) {
        let root = std::env::temp_dir().join(format!("hemat-{}-{}", std::process::id(), number()));
        let _ = fs::remove_dir_all(&root);
        let hemat = Hemat::at(
            root.clone(),
            enabled,
            exceptions.iter().map(|name| (*name).to_owned()).collect(),
        );
        (hemat, root)
    }

    /// The value written in a file of an app's group.
    fn written(effects: &[Effect], file: &str) -> Option<String> {
        effects
            .iter()
            .find(|effect| effect.file == file)
            .map(|effect| effect.value.clone())
    }

    #[test]
    fn the_hemat_tree_is_a_slice_of_its_own() {
        assert_eq!(tree(), Path::new("/sys/fs/cgroup/hemat"));
        assert_eq!(
            app_dir("foot", 3),
            Path::new("/sys/fs/cgroup/hemat/app-foot-3")
        );
    }

    #[test]
    fn a_class_name_is_made_safe_for_a_cgroup() {
        assert_eq!(safe("org.example.Notes"), "org.example.Notes");
        assert_eq!(safe("my prog+x"), "my_prog_x");
        assert_eq!(safe(""), "program");
    }

    #[test]
    fn the_controllers_are_the_three_ferrix_has() {
        assert_eq!(CONTROLLERS, ["+cpu", "+memory", "+pids"]);
    }

    #[test]
    fn a_write_that_fails_leaves_the_app_alone() {
        let failed = write_with(Path::new("/nowhere"), "cpu.weight", "10", |_, _| {
            Err(io::Error::other("no such file"))
        });
        let why = failed.expect_err("a write that fails says so");
        assert!(why.contains("cpu.weight"), "{why}");
        assert!(why.contains("no such file"), "{why}");
    }

    #[test]
    fn an_app_gets_a_group_of_its_own() {
        let root = std::env::temp_dir().join(format!("hemat-group-{}", number()));
        let _ = fs::remove_dir_all(&root);
        let dir = make_app_in(&root, "foot", 1).expect("the group's made");
        assert_eq!(dir, root.join("hemat").join("app-foot-1"));
        assert!(dir.is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_app_is_one_class_and_one_group() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(
            &[
                window(1, "foot", 100, true),
                window(2, "foot", 100, false),
                window(3, "zinc", 200, true),
            ],
            now,
        );
        assert_eq!(hemat.apps.len(), 2, "two classes, two apps");
        assert_eq!(hemat.apps["foot"].dir, root.join("hemat/app-foot-1"));
        assert_eq!(hemat.apps["zinc"].dir, root.join("hemat/app-zinc-2"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_app_of_a_window_is_the_class_it_named() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        assert_eq!(hemat.apps["foot"].class, "foot");
        // A client whose process the kernel would not say has no group.
        let _ = hemat.refresh(&[window(2, "zinc", 0, true)], now);
        assert!(!hemat.apps.contains_key("zinc"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_first_window_of_an_app_makes_its_group() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let effects = hemat.refresh(&[window(1, "foot", 100, true)], now);
        assert!(root.join("hemat/app-foot-1").is_dir());
        assert_eq!(written(&effects, "memory.high"), Some("max".to_owned()));
        assert_eq!(written(&effects, "cpu.weight"), Some("100".to_owned()));
        // A second window of the same app is the same group.
        let _ = hemat.refresh(&[window(2, "foot", 100, true)], now);
        assert_eq!(hemat.apps.len(), 1);
        assert!(!root.join("hemat/app-foot-2").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_window_that_went_away_takes_its_group_with_it() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        let dir = hemat.apps["foot"].dir.clone();
        let _ = hemat.refresh(&[], now);
        assert!(!hemat.apps.contains_key("foot"));
        assert!(!dir.exists(), "the group goes with the app");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_app_with_a_visible_window_is_active() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        assert_eq!(hemat.state_of("foot"), Some(State::Aktif));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_app_with_every_window_hidden_is_in_the_background() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], now);
        assert_eq!(hemat.state_of("foot"), Some(State::Latar));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_background_app_falls_asleep_after_five_minutes() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], now);
        let later = now + Duration::from_secs(299);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], later);
        assert_eq!(hemat.state_of("foot"), Some(State::Latar));
        let later = now + Duration::from_secs(301);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], later);
        assert_eq!(hemat.state_of("foot"), Some(State::Tidur));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_app_never_sleeps_while_it_is_heard() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        hemat.heard("foot", true);
        let later = now + Duration::from_secs(600);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], later);
        assert_eq!(hemat.state_of("foot"), Some(State::Latar));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_sleeping_app_is_woken_when_it_is_shown_again() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        let later = now + Duration::from_secs(600);
        let _ = hemat.refresh(&[window(1, "foot", 100, false)], later);
        assert_eq!(hemat.state_of("foot"), Some(State::Tidur));
        let effects = hemat.refresh(&[window(1, "foot", 100, true)], later);
        assert_eq!(hemat.state_of("foot"), Some(State::Aktif));
        assert_eq!(written(&effects, "memory.high"), Some("max".to_owned()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_asleep_app_gives_its_clean_cache_back() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        let later = now + Duration::from_secs(600);
        let effects = hemat.refresh(&[window(1, "foot", 100, false)], later);
        assert_eq!(written(&effects, "memory.reclaim"), Some("1G".to_owned()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_hemat_dispatcher_takes_a_sound() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(
            &[window(1, "foot", 100, true), window(2, "zinc", 200, true)],
            now,
        );
        let (_, said) = dispatch("sound foot", &mut hemat);
        assert!(said.is_empty(), "a sound is said with no words");
        assert!(hemat.apps["foot"].loud, "foot is making a sound");
        assert!(!hemat.apps["zinc"].loud, "zinc is not");
        let _ = dispatch("quiet foot", &mut hemat);
        assert!(!hemat.apps["foot"].loud, "foot is quiet again");
        // Another name does not touch the first one.
        let _ = dispatch("sound zinc", &mut hemat);
        assert!(hemat.apps["zinc"].loud);
        assert!(!hemat.apps["foot"].loud);
        let (_, said) = dispatch("state", &mut hemat);
        assert_eq!(said.len(), 3, "every app is named, and the count: {said:?}");
        assert!(
            said.last()
                .is_some_and(|line| line.contains("the apps are asleep: 0")),
            "the count of the sleeping ones: {said:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_exception_list_is_split_on_commas() {
        assert_eq!(
            exceptions("steam, wine"),
            vec!["steam".to_owned(), "wine".to_owned()]
        );
        assert!(exceptions("").is_empty());
        assert_eq!(
            exceptions(" steam , wine "),
            vec!["steam".to_owned(), "wine".to_owned()]
        );
    }

    #[test]
    fn an_excluded_app_never_sleeps() {
        let (mut hemat, root) = hemat(true, &["steam"]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "steam", 100, true)], now);
        let later = now + Duration::from_secs(600);
        let effects = hemat.refresh(&[window(1, "steam", 100, false)], later);
        assert_eq!(hemat.state_of("steam"), Some(State::Latar));
        assert_eq!(written(&effects, "cpu.weight"), Some("100".to_owned()));
        assert_eq!(written(&effects, "memory.reclaim"), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_hemat_off_means_no_cgroup_is_made() {
        let (mut hemat, root) = hemat(false, &[]);
        let now = Instant::now();
        let _ = hemat.refresh(&[window(1, "foot", 100, true)], now);
        assert!(hemat.apps.is_empty());
        assert!(!root.join("hemat/app-foot-1").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_background_app_is_quieter_than_the_active_one() {
        let (mut hemat, root) = hemat(true, &[]);
        let now = Instant::now();
        let effects = hemat.refresh(&[window(1, "foot", 100, true)], now);
        assert_eq!(written(&effects, "cpu.weight"), Some("100".to_owned()));
        let effects = hemat.refresh(&[window(1, "foot", 100, false)], now);
        assert_eq!(written(&effects, "cpu.weight"), Some("10".to_owned()));
        let _ = fs::remove_dir_all(&root);
    }
}
