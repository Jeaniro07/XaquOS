//! The Spotlight card (SUPER Space): a query over the desktop's top, read
//! as arithmetic and matched against the installed applications, the open
//! windows and the names under ~ (`search.rs`), drawn in the glass.
//!
//! Started again while it is up, it closes: the wallpaper switcher's
//! [`switch::once`], on `$XDG_RUNTIME_DIR/xaqu-spotlight.sock`
//! (`/tmp/xaqu-spotlight-UID.sock` with no runtime folder).

use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use compositor_toolkit::Event;
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::hypr::Hypr;
use crate::search::{self, App, Hit};
use crate::switch;
use crate::ui::{HitRow, Spotlight, SpotlightSignal};
use crate::window::{entries, say};

/// The surface's namespace.
pub const NAMESPACE: &str = "xaqu-spotlight";

/// The card's width, in logical pixels.
pub const CARD: u32 = 600;
/// The query line's height.
pub const HEAD: f32 = 56.0;
/// A row's height.
pub const STEP: f32 = 44.0;
/// The card's sides under its rows.
pub const FOAM: f32 = 14.0;

/// What running a row asks of the loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A line to hand hyprix: `exec ...` or `focuswindow address:...`.
    Dispatch(String),
    /// Close without running anything.
    Close,
}

/// The keys the card is edited with, as the tree reports them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// The last character typed is gone.
    Backspace,
    /// Return: run the chosen row.
    Enter,
    /// A row up.
    Up,
    /// A row down.
    Down,
    /// Escape: close.
    Escape,
}

/// What the card holds: the sources and the query over them.
#[derive(Clone, Debug)]
pub struct State {
    apps: Vec<App>,
    windows: Vec<(String, String)>,
    files: Vec<String>,
    query: String,
    chosen: usize,
    hits: Vec<Hit>,
}

impl State {
    /// The card over `apps`, with nothing typed yet.
    #[must_use]
    pub fn new(apps: Vec<App>) -> Self {
        let mut state = State {
            apps,
            windows: Vec::new(),
            files: Vec::new(),
            query: String::new(),
            chosen: 0,
            hits: Vec::new(),
        };
        state.recount();
        state
    }

    /// The windows open, as `(address, title)`.
    pub fn set_windows(&mut self, windows: Vec<(String, String)>) {
        self.windows = windows;
        self.recount();
    }

    /// The names under ~, as full paths.
    pub fn set_files(&mut self, files: Vec<String>) {
        self.files = files;
        self.recount();
    }

    /// The query typed.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The row in hand.
    #[must_use]
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    /// How many rows there are.
    #[must_use]
    pub fn rows_len(&self) -> usize {
        self.hits.len()
    }

    /// Text typed into the line; the hits are worked out again.
    pub fn type_in(&mut self, text: &str) {
        self.query.push_str(text);
        self.chosen = 0;
        self.recount();
    }

    /// A key; whether it asks for the card to close.
    pub fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Backspace => {
                self.query.pop();
                self.chosen = 0;
                self.recount();
                false
            }
            Key::Enter => false,
            Key::Up => {
                self.move_chosen(false);
                false
            }
            Key::Down => {
                self.move_chosen(true);
                false
            }
            Key::Escape => true,
        }
    }

    /// The rows as the tree shows them, best first.
    #[must_use]
    pub fn rows(&self) -> Vec<HitRow> {
        self.hits
            .iter()
            .enumerate()
            .map(|(at, hit)| {
                let (icon, letter, text, detail) = match hit {
                    Hit::App { title, icon, .. } => {
                        (*icon, first(title), title.clone(), "Application".to_owned())
                    }
                    Hit::Window { title, .. } => {
                        ("", first(title), title.clone(), "Window".to_owned())
                    }
                    Hit::File { path } => {
                        let name = path.rsplit('/').next().unwrap_or(path.as_str());
                        let rest = path.strip_suffix(name).unwrap_or(path.as_str());
                        let dir = rest.strip_suffix('/').unwrap_or(rest);
                        ("", first(name), name.to_owned(), dir.to_owned())
                    }
                    Hit::Value(value) => ("", first(value), value.clone(), "Calculator".to_owned()),
                };
                HitRow {
                    icon: s(icon),
                    letter: s(letter),
                    text: s(text),
                    detail: s(detail),
                    on: at == self.chosen,
                }
            })
            .collect()
    }

    /// What running row `at` asks; `None` for a row that is not runnable,
    /// and for a row there is not. A file opens with its default handler.
    // Ponytail: paths come from the scan, never typed, so they are not
    // quoted here. Upgrade path: shell-quoting when a name ever holds a
    // space that matters -- xdg-open takes one argument, and hyprix's exec
    // splits words.
    #[must_use]
    pub fn run(&self, at: usize) -> Option<Outcome> {
        match self.hits.get(at)? {
            Hit::App { exec, .. } => Some(Outcome::Dispatch(format!("exec {exec}"))),
            Hit::Window { address, .. } => {
                Some(Outcome::Dispatch(format!("focuswindow address:{address}")))
            }
            Hit::File { path } => Some(Outcome::Dispatch(format!("exec /bin/xdg-open {path}"))),
            Hit::Value(_) => None,
        }
    }

    /// The hits of the query as it stands.
    fn recount(&mut self) {
        self.hits = search::hits(&self.query, &self.apps, &self.windows, &self.files);
        self.chosen = self.chosen.min(self.hits.len().saturating_sub(1));
    }

    /// A row in hand, moved.
    fn move_chosen(&mut self, down: bool) {
        if self.hits.is_empty() {
            return;
        }
        if down {
            self.chosen = (self.chosen + 1).min(self.hits.len() - 1);
        } else {
            self.chosen = self.chosen.saturating_sub(1);
        }
    }
}

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// A word's first letter, for the tile where there is no icon.
fn first(text: &str) -> String {
    text.chars().next().map(String::from).unwrap_or_default()
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The spotlight card.
    pub spot: Spotlight,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl View {
    /// The tree, drawn with `fonts`.
    #[must_use]
    pub fn new(fonts: tole::Fonts) -> Option<Self> {
        let mut ui = Ui::new(fonts);
        let spot = Spotlight::new(&mut ui, None)?;
        spot.ic_search
            .set(&mut ui, s(crate::look::icon("magnifying-glass")));
        Some(View { ui, spot })
    }

    /// Draw in `tokens`.
    pub fn wear(&mut self, tokens: &crate::theme::Tokens) {
        tokens.apply(&mut self.ui);
    }

    /// Its size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.spot.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree the query, the rows and the pick.
    pub fn show(&mut self, state: &State) {
        let ui = &mut self.ui;
        let spot = &self.spot;
        spot.query.set(ui, s(state.query()));
        spot.card_w.set(ui, CARD as f32);
        spot.card_h
            .set(ui, HEAD + STEP * state.rows_len() as f32 + FOAM);
        spot.rows.set(ui, state.rows());
    }

    /// Do what the tree asked since the last call: the lines to run, the
    /// card to close, and whether the tree wants drawing again.
    pub fn take(&mut self, state: &mut State) -> (Vec<Outcome>, bool, bool) {
        let emitted = self.ui.take_signals();
        let mut outcomes = Vec::new();
        let mut close = false;
        let mut changed = false;
        for each in &emitted {
            let Some(signal) = self.spot.signal(each) else {
                continue;
            };
            match signal {
                SpotlightSignal::Typed(text) => {
                    let text = text.trim();
                    if !text.is_empty() {
                        state.type_in(text);
                        changed = true;
                    }
                }
                SpotlightSignal::Backspaced => {
                    state.key(Key::Backspace);
                    changed = true;
                }
                SpotlightSignal::Stepped(down) => {
                    let key = if down != 0 { Key::Down } else { Key::Up };
                    state.key(key);
                    changed = true;
                }
                SpotlightSignal::Entered => {
                    if let Some(outcome) = state.run(state.chosen()) {
                        outcomes.push(outcome);
                    }
                }
                SpotlightSignal::Run(index) => {
                    if let Some(at) = usize::try_from(index).ok()
                        && let Some(outcome) = state.run(at)
                    {
                        outcomes.push(outcome);
                    }
                }
                SpotlightSignal::Dismissed => close = true,
            }
        }
        (outcomes, close, changed)
    }
}

/// The person's home.
fn home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The windows open, as `(address, title)`, from `j/clients`.
fn windows(hypr: Option<&Hypr>) -> Vec<(String, String)> {
    hypr.and_then(|hypr| hypr.ask("j/clients"))
        .and_then(|text| crate::json::Json::parse(&text))
        .map_or(Vec::new(), |list| {
            list.items()
                .iter()
                .filter_map(|client| {
                    let address = client.get("address").and_then(crate::json::Json::str)?;
                    let title = client.get("title").and_then(crate::json::Json::str)?;
                    (!(address.is_empty() || title.is_empty()))
                        .then(|| (address.to_owned(), title.to_owned()))
                })
                .collect()
        })
}

/// The names under ~, read off the program's loop (a walk of 5000 names on
/// a slow machine takes long enough for a key held to repeat); they come
/// as one lot, full paths.
fn scan_home() -> Option<std::sync::mpsc::Receiver<Vec<String>>> {
    let home = home()?;
    let (send, receive) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("spotlight-files".to_owned())
        .spawn(move || {
            let found = search::scan(&home, 5000, 300_000_000);
            let full: Vec<String> = found
                .iter()
                .map(|path| home.join(path).to_string_lossy().into_owned())
                .collect();
            let _ = send.send(full);
        });
    Some(receive)
}

/// Take the names read so far; whether any came.
fn take_scan(state: &mut State, from: &std::sync::mpsc::Receiver<Vec<String>>) -> bool {
    let mut came = false;
    while let Ok(files) = from.try_recv() {
        state.set_files(files);
        came = true;
    }
    came
}

/// Run the spotlight until it is closed; the exit status.
#[must_use]
pub fn run() -> i32 {
    switch::once(NAMESPACE, shown).unwrap_or_else(|| {
        say("the spotlight was up: closed");
        0
    })
}

/// The spotlight's loop.
fn shown(listener: Option<&UnixListener>) -> i32 {
    let hypr = Hypr::from_env();
    let apps = entries();
    let apps_shown = apps.len();
    let mut state = State::new(apps);
    state.set_windows(windows(hypr.as_ref()));
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the spotlight's tree could not be made");
        return 1;
    };
    view.wear(&crate::theme::Live::new().tokens);
    view.show(&state);
    let layer = LayerOptions {
        output: None,
        layer: Layer::Overlay,
        namespace: NAMESPACE.to_owned(),
        size: (0, 0),
        anchor: Anchor::TOP
            .with(Anchor::BOTTOM)
            .with(Anchor::LEFT)
            .with(Anchor::RIGHT),
        exclusive_zone: -1,
        margin: Margin::default(),
        keyboard: KeyboardInteractivity::Exclusive,
    };
    let options = WindowOptions {
        title: "Spotlight".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (1280, 800),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.spot.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!(
                "err: no compositor to put the spotlight on: {error}"
            ));
            return 1;
        }
    };
    let watch = listener.map(|listener| window.client().watch_fd(listener.as_raw_fd()));
    let mut size = None;
    let mut scanning = scan_home();
    loop {
        let wait = scanning.as_ref().map(|_| Duration::from_millis(120));
        let happened = match window.dispatch(&mut view.ui, wait) {
            Ok(happened) => happened,
            Err(error) => {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        };
        let mut changed = false;
        if let Some(now) = window.size()
            && size != Some(now)
        {
            if size.is_none() {
                say(&format!(
                    "the spotlight is up, {}x{}, {} apps",
                    now.0, now.1, apps_shown
                ));
            }
            size = Some(now);
            view.resize(now);
            changed = true;
        }
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Readable(id) if Some(id) == watch => {
                    say("the spotlight was asked again: closed");
                    return 0;
                }
                _ => {}
            }
        }
        if let Some(from) = &scanning
            && take_scan(&mut state, from)
        {
            scanning = None;
            changed = true;
        }
        let (outcomes, close, asked) = view.take(&mut state);
        changed |= asked;
        if changed {
            view.show(&state);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
        for outcome in outcomes {
            match outcome {
                Outcome::Dispatch(line) => {
                    let done = hypr.as_ref().is_some_and(|hypr| hypr.dispatch(&line));
                    say(&format!(
                        "spotlight: {line}: {}",
                        if done { "ok" } else { "refused" }
                    ));
                    if done {
                        return 0;
                    }
                }
                Outcome::Close => {}
            }
        }
        if close {
            say("the spotlight closed");
            return 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, Outcome, State, View};

    fn apps() -> Vec<crate::search::App> {
        vec![
            crate::search::App {
                exec: "/bin/term /bin/zinc".to_owned(),
                title: "Terminal".to_owned(),
                icon: "terminal-window",
            },
            crate::search::App {
                exec: "/bin/shelf".to_owned(),
                title: "Files".to_owned(),
                icon: "folder-simple",
            },
        ]
    }

    #[test]
    fn a_query_fills_the_rows_and_enter_runs_the_first() {
        let mut state = State::new(apps());
        state.type_in("te");
        assert_eq!(state.rows().len(), 1);
        assert_eq!(state.chosen(), 0);
        assert_eq!(
            state.run(0),
            Some(Outcome::Dispatch("exec /bin/term /bin/zinc".to_owned()))
        );
    }

    #[test]
    fn a_value_runs_nothing_and_a_window_focuses() {
        let mut state = State::new(Vec::new());
        state.type_in("2+2");
        assert_eq!(state.rows().first().map(|row| row.text.as_ref()), Some("4"));
        assert_eq!(state.run(0), None, "a value is not runnable");
        let mut state = State::new(Vec::new());
        state.set_windows(vec![("0x9".to_owned(), "notes.md — Files".to_owned())]);
        state.type_in("notes");
        assert_eq!(
            state.run(0),
            Some(Outcome::Dispatch("focuswindow address:0x9".to_owned()))
        );
    }

    #[test]
    fn typing_edits_and_escape_asks_to_close() {
        let mut state = State::new(Vec::new());
        state.type_in("fi");
        state.key(Key::Backspace);
        assert_eq!(state.query(), "f");
        assert!(state.key(Key::Escape), "escape asks for close");
    }

    #[test]
    fn a_row_down_moves_the_pick() {
        let mut state = State::new(apps());
        state.type_in("e");
        assert_eq!(state.chosen(), 0);
        state.key(Key::Down);
        assert_eq!(state.chosen(), 1);
        state.key(Key::Down);
        assert_eq!(state.chosen(), 1, "the last row holds");
        state.key(Key::Up);
        assert_eq!(state.chosen(), 0);
    }

    #[test]
    fn keys_and_presses_drive_the_tree() {
        let mut state = State::new(apps());
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&state);
        view.ui.update();
        assert!(view.ui.key_press(&tole::KeyEvent::named("t")));
        assert!(view.ui.key_press(&tole::KeyEvent::named("e")));
        let (_, _, changed) = view.take(&mut state);
        assert!(changed);
        assert_eq!(state.query(), "te");
        view.show(&state);
        assert!(view.ui.key_press(&tole::KeyEvent::named("Return")));
        let (outcomes, close, changed) = view.take(&mut state);
        assert!(!close && !changed);
        assert_eq!(
            outcomes,
            vec![Outcome::Dispatch("exec /bin/term /bin/zinc".to_owned())]
        );
        assert!(view.ui.key_press(&tole::KeyEvent::named("Escape")));
        let (_, close, _) = view.take(&mut state);
        assert!(close);
    }

    #[test]
    fn the_card_draws() {
        let mut state = State::new(apps());
        state.type_in("te");
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&state);
        view.ui.update();
        let mut frame =
            tole::Frame::new((1024, 768), 1.0, tole::Color::rgba(0, 0, 0, 0)).expect("made");
        let _ = view.ui.render_into(&mut frame);
        assert!(!frame.pixmap().data().is_empty());
    }
}
