//! The notices: the desktop's notifications as the shell shows them — the
//! glass banners at the top right and the history behind the clock — read
//! from notifyd's line socket (`docs/DESKTOP.md` §4.2).
//!
//! The parsing is pure and tested here; the surface and the socket are
//! the thread the dock starts, which is also where the menu bar's clock
//! asks for the panel.

use std::io::Write as _;
use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use compositor_toolkit::{Event, Rect, WatchId};
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::ui::{NoticeTile, Notices, NoticesSignal};
use crate::window::say;

/// The surface's namespace: Yoru's theme blurs behind every `yoru-` one.
pub const NAMESPACE: &str = "yoru-notices";

/// The surface's width, in logical pixels.
pub const WIDTH: u32 = 360;
/// The surface's height: the banners and the panel under them.
pub const HEIGHT: u32 = 560;
/// A banner's height.
const BANNER: f32 = 64.0;
/// The gap under a banner.
const GAP: f32 = 8.0;
/// A tile's height in the panel.
const TILE: f32 = 54.0;
/// How many tiles the panel shows.
const TILES: usize = 5;
/// How many banners are up at once; the oldest goes first past this.
const MAX_BANNERS: usize = 3;
/// How many notices the history keeps.
const HISTORY: usize = 50;
/// How long a banner stays up.
const LIFETIME: Duration = Duration::from_secs(5);
/// How often the panel and the banners are looked at.
const TICK: Duration = Duration::from_millis(200);
/// How often notifyd is looked for, when it is not there.
const RETRY: Duration = Duration::from_secs(2);

/// Whether the panel is open; the menu bar's clock asks for it.
static OPEN: AtomicBool = AtomicBool::new(false);

/// One notice, as the socket carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// Its id, which `close` names it by.
    pub id: u32,
    /// The application that sent it.
    pub app: String,
    /// Its summary.
    pub summary: String,
    /// Its body.
    pub body: String,
}

/// The socket's lines, read into notices; a line that is not one is
/// dropped.
#[must_use]
pub fn parse(lines: &[&str]) -> Vec<Notice> {
    lines
        .iter()
        .filter_map(|line| {
            let rest = line.strip_prefix("notice ")?;
            let (id, rest) = rest.split_once(' ')?;
            let id = id.parse().ok()?;
            let (app, rest) = rest.split_once('\t')?;
            let (summary, body) = rest.split_once('\t')?;
            Some(Notice {
                id,
                app: app.to_owned(),
                summary: summary.to_owned(),
                body: body.to_owned(),
            })
        })
        .collect()
}

/// The panel, opened or closed; the clock's doing.
pub fn toggle() {
    let open = !OPEN.load(Ordering::SeqCst);
    OPEN.store(open, Ordering::SeqCst);
    say(&format!(
        "notices: the panel is {}",
        if open { "open" } else { "closed" }
    ));
}

/// What a press in the notices asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// A tile of the history: close that notice.
    Close(u32),
    /// A banner: close that notice.
    Banner(u32),
    /// The Clear button: close them all.
    Clear,
}

/// What the notices show: the history, the banners up, and the panel.
#[derive(Clone, Debug, Default)]
pub struct State {
    history: Vec<Notice>,
    banners: Vec<(Notice, Instant)>,
    open: bool,
    connected: bool,
}

impl State {
    /// Nothing yet.
    #[must_use]
    pub fn new() -> Self {
        State::default()
    }

    /// A notice comes in: into the history and up as a banner.
    pub fn push(&mut self, notice: Notice, now: Instant) {
        self.history.retain(|kept| kept.id != notice.id);
        self.history.insert(0, notice.clone());
        self.history.truncate(HISTORY);
        self.banners.retain(|(_, until)| *until > now);
        self.banners.insert(0, (notice, now + LIFETIME));
        while self.banners.len() > MAX_BANNERS {
            self.banners.pop();
        }
    }

    /// The banners whose time is up; whether any went.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.banners.len();
        self.banners.retain(|(_, until)| *until > now);
        self.banners.len() != before
    }

    /// The panel, open or closed; whether it changed.
    pub fn set_open(&mut self, open: bool) -> bool {
        let changed = self.open != open;
        self.open = open;
        changed
    }

    /// Whether the panel is open.
    #[must_use]
    pub fn open(&self) -> bool {
        self.open
    }

    /// Whether notifyd was there.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// Notifyd, there or not; whether it changed.
    pub fn set_connected(&mut self, connected: bool) -> bool {
        let changed = self.connected != connected;
        self.connected = connected;
        changed
    }

    /// Take a notice out of the history and down; whether it was there.
    pub fn close(&mut self, id: u32) -> bool {
        let before = self.history.len();
        self.history.retain(|kept| kept.id != id);
        self.banners.retain(|(notice, _)| notice.id != id);
        self.history.len() != before
    }

    /// Forget them all.
    pub fn clear(&mut self) {
        self.history.clear();
        self.banners.clear();
    }

    /// The ids of the history, newest first.
    #[must_use]
    pub fn ids(&self) -> Vec<u32> {
        self.history.iter().map(|notice| notice.id).collect()
    }

    /// The notice of a tile of the history.
    #[must_use]
    pub fn tile(&self, at: usize) -> Option<&Notice> {
        self.history.get(at)
    }

    /// The notice of a banner.
    #[must_use]
    pub fn banner(&self, at: usize) -> Option<&Notice> {
        self.banners.get(at).map(|(notice, _)| notice)
    }
}

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// The panel's place under the banners.
fn panel_y() -> f32 {
    MAX_BANNERS as f32 * (BANNER + GAP)
}

/// The panel's height.
fn panel_h() -> f32 {
    44.0 + TILES as f32 * TILE + 8.0
}

/// The panel's width.
fn panel_w() -> f32 {
    f32::from(u16::try_from(WIDTH).unwrap_or(u16::MAX)) - 16.0
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The notices' surface.
    pub notices: Notices,
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
        let notices = Notices::new(&mut ui, None)?;
        Some(View { ui, notices })
    }

    /// Draw in `tokens`.
    pub fn wear(&mut self, tokens: &crate::theme::Tokens) {
        tokens.apply(&mut self.ui);
    }

    /// Its size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.notices.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree the notices, the banners and the panel.
    pub fn show(&mut self, state: &State) {
        let ui = &mut self.ui;
        let tile = |notice: &Notice| NoticeTile {
            app: s(&notice.app),
            summary: s(&notice.summary),
            body: s(&notice.body),
        };
        let banners = state
            .banners
            .iter()
            .map(|(notice, _)| tile(notice))
            .collect::<Vec<_>>();
        let tiles = state
            .history
            .iter()
            .take(TILES)
            .map(tile)
            .collect::<Vec<_>>();
        self.notices.banners.set(ui, banners);
        self.notices.tiles.set(ui, tiles);
        self.notices.open.set(ui, state.open());
        self.notices.status.set(
            ui,
            s(if state.connected() {
                ""
            } else {
                "notifyd is not running"
            }),
        );
        self.notices.banner_h.set(ui, BANNER);
        self.notices.tile_h.set(ui, TILE);
        self.notices.panel_y.set(ui, panel_y());
        self.notices.panel_h.set(ui, panel_h());
        self.notices.panel_w.set(ui, panel_w());
    }

    /// What the tree asked since the last call.
    pub fn take(&mut self, state: &State) -> Vec<Asked> {
        let emitted = self.ui.take_signals();
        let mut asked = Vec::new();
        for each in &emitted {
            let Some(signal) = self.notices.signal(each) else {
                continue;
            };
            match signal {
                NoticesSignal::Picked(index) => {
                    if let Some(notice) = state.tile(usize::try_from(index).unwrap_or(usize::MAX)) {
                        asked.push(Asked::Close(notice.id));
                    }
                }
                NoticesSignal::Bannered(index) => {
                    if let Some(notice) = state.banner(usize::try_from(index).unwrap_or(usize::MAX))
                    {
                        asked.push(Asked::Banner(notice.id));
                    }
                }
                NoticesSignal::Cleared => asked.push(Asked::Clear),
            }
        }
        asked
    }
}

/// Where the pointer reaches the surface: the banners and the panel, and
/// nothing else, so the rest of the screen is the windows'.
fn region(state: &State) -> Vec<Rect> {
    let mut rects = Vec::new();
    if !state.banners.is_empty() {
        let height = state.banners.len() as f32 * (BANNER + GAP);
        rects.push(Rect {
            x: 8,
            y: 0,
            width: i32::try_from(WIDTH).unwrap_or(360) - 16,
            height: height as i32,
        });
    }
    if state.open() {
        rects.push(Rect {
            x: 8,
            y: panel_y() as i32,
            width: panel_w() as i32,
            height: panel_h() as i32,
        });
    }
    rects
}

/// The notices on a thread of the dock's program, with their own tree and
/// their own connection to the compositor.
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("notices".to_owned())
        .spawn(|| {
            let code = run();
            say(&format!("the notices stopped: {code}"));
        });
    if let Err(error) = spawned {
        say(&format!("err: the notices' thread: {error}"));
    }
}

/// Run the notices until the compositor goes away; the exit status.
#[must_use]
pub fn run() -> i32 {
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the notices' tree could not be made");
        return 1;
    };
    view.wear(&crate::theme::Live::new().tokens);
    let layer = LayerOptions {
        output: None,
        layer: Layer::Overlay,
        namespace: NAMESPACE.to_owned(),
        size: (WIDTH, HEIGHT),
        anchor: Anchor::TOP.with(Anchor::RIGHT),
        exclusive_zone: -1,
        margin: Margin {
            top: 32,
            right: 8,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
    };
    let options = WindowOptions {
        title: "Yoru notices".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (WIDTH, HEIGHT),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.notices.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!(
                "err: no compositor to put the notices on: {error}"
            ));
            return 1;
        }
    };
    let tick = window.client().add_timer(TICK, Some(TICK));
    let mut state = State::new();
    let mut socket: Option<UnixStream> = None;
    let mut watch: Option<WatchId> = None;
    let mut pending = String::new();
    let mut again = Instant::now();
    view.show(&state);
    // Nothing is drawn yet: the surface takes no presses, so the windows
    // under it are reached.
    window.set_input_region(Some(&region(&state)));
    loop {
        let happened = match window.dispatch(&mut view.ui, None) {
            Ok(happened) => happened,
            Err(error) => {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        };
        let mut changed = false;
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Timer(timer) if timer == tick => {
                    changed |= state.expire(Instant::now());
                    changed |= state.set_open(OPEN.load(Ordering::SeqCst));
                    // Notifyd, when it is not there yet.
                    changed |=
                        connect_later(&mut window, &mut socket, &mut watch, &mut again, &mut state);
                }
                Event::Readable(id) if Some(id) == watch => {
                    changed |= read_from(
                        &mut window,
                        &mut socket,
                        &mut watch,
                        &mut pending,
                        &mut state,
                    );
                }
                _ => {}
            }
        }
        for asked in view.take(&state) {
            match asked {
                Asked::Close(id) | Asked::Banner(id) => {
                    close(&mut socket, id);
                    changed |= state.close(id);
                }
                Asked::Clear => {
                    clear(&mut socket, &mut state);
                    changed = true;
                }
            }
        }
        if changed {
            view.show(&state);
            window.set_input_region(Some(&region(&state)));
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
    }
}

/// Notifyd, when it is not there yet; whether anything changed.
fn connect_later(
    window: &mut Window,
    socket: &mut Option<UnixStream>,
    watch: &mut Option<WatchId>,
    again: &mut Instant,
    state: &mut State,
) -> bool {
    if socket.is_some() || *again > Instant::now() {
        return false;
    }
    *again = Instant::now() + RETRY;
    let Some((stream, id)) = connect(window) else {
        return false;
    };
    *socket = Some(stream);
    *watch = Some(id);
    let _ = state.set_connected(true);
    say("notices: notifyd is there");
    true
}

/// The socket's lines, read into notices; whether anything came.
fn read_from(
    window: &mut Window,
    socket: &mut Option<UnixStream>,
    watch: &mut Option<WatchId>,
    pending: &mut String,
    state: &mut State,
) -> bool {
    let Some(stream) = socket.as_mut() else {
        return false;
    };
    let mut came = 0;
    if crate::window::drain(stream, pending) {
        while let Some(end) = pending.find('\n') {
            let line: String = pending.drain(..=end).collect();
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            for notice in parse(&[line]) {
                state.push(notice, Instant::now());
                came += 1;
            }
        }
    } else {
        say("notices: notifyd's socket closed");
        if let Some(id) = watch.take() {
            window.client().unwatch_fd(id);
        }
        *socket = None;
        let _ = state.set_connected(false);
    }
    came > 0
}

/// Every notice closed.
fn clear(socket: &mut Option<UnixStream>, state: &mut State) {
    for id in state.ids() {
        close(socket, id);
    }
    state.clear();
}

/// Notifyd's socket, subscribed to the notices: the history and then
/// every new one.
fn connect(window: &mut Window) -> Option<(UnixStream, WatchId)> {
    let path = crate::switch::socket("notifyd")?;
    let mut stream = UnixStream::connect(&path).ok()?;
    let _ = writeln!(stream, "subscribe");
    let _ = stream.set_nonblocking(true);
    let watch = window.client().watch_fd(stream.as_raw_fd());
    Some((stream, watch))
}

/// Tell notifyd that a notice is closed.
fn close(socket: &mut Option<UnixStream>, id: u32) {
    let Some(stream) = socket.as_mut() else {
        return;
    };
    let _ = writeln!(stream, "close {id}");
}

#[cfg(test)]
mod tests {
    use super::{Notice, State, View, parse};

    fn lines() -> Vec<Notice> {
        parse(&[
            "notice 7 Files\tCopy done\tnotes.md to the stash",
            "notice 6 devmgr\tGPU driver is up\t",
        ])
    }

    #[test]
    fn the_socket_lines_are_read_into_notices() {
        let notices = lines();
        assert_eq!(notices.len(), 2);
        assert_eq!(notices[0].id, 7);
        assert_eq!(notices[0].app, "Files");
        assert_eq!(notices[0].summary, "Copy done");
        assert_eq!(notices[0].body, "notes.md to the stash");
        assert_eq!(notices[1].body, "");
    }

    #[test]
    fn a_bad_line_is_dropped() {
        assert!(parse(&["notice", "notice x", "other 1 a b c"]).is_empty());
    }

    /// A notice comes in: the history keeps it, and a banner goes up for
    /// five seconds, at most three at a time.
    #[test]
    fn a_notice_is_kept_and_shown_as_a_banner() {
        let now = std::time::Instant::now();
        let mut state = State::new();
        for id in 1..=4 {
            let notice = Notice {
                id,
                app: "Files".to_owned(),
                summary: format!("n{id}"),
                body: String::new(),
            };
            state.push(notice, now);
        }
        assert_eq!(state.ids().len(), 4, "the history keeps them all");
        assert_eq!(state.banner(0).map(|notice| notice.id), Some(4));
        assert_eq!(state.banner(2).map(|notice| notice.id), Some(2));
        assert!(state.banner(3).is_none(), "three banners at a time");
        assert!(state.expire(now + std::time::Duration::from_secs(6)));
        assert!(
            state.banner(0).is_none(),
            "the banners go after five seconds"
        );
    }

    /// The surface draws the banners and the panel.
    #[test]
    fn the_notices_draw() {
        let now = std::time::Instant::now();
        let mut state = State::new();
        state.push(
            Notice {
                id: 7,
                app: "Files".to_owned(),
                summary: "Copy done".to_owned(),
                body: "notes.md to the stash".to_owned(),
            },
            now,
        );
        state.set_open(true);
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((360, 560));
        view.show(&state);
        view.ui.update();
        let mut frame =
            tole::Frame::new((360, 560), 1.0, tole::Color::rgba(0, 0, 0, 0)).expect("made");
        let _ = view.ui.render_into(&mut frame);
        assert!(!frame.pixmap().data().is_empty());
        let drawn = |x: u32, y: u32| {
            frame
                .pixmap()
                .pixel(x, y)
                .map(|pixel| pixel.alpha())
                .unwrap_or(0)
        };
        assert!(drawn(20, 40) > 0, "the banner is drawn");
        assert!(drawn(20, 230) > 0, "the panel's card is drawn");
    }

    /// A tile closes its notice, and the Clear button closes them all.
    #[test]
    fn a_notice_is_closed_and_the_rest_kept() {
        let now = std::time::Instant::now();
        let mut state = State::new();
        for id in 1..=3 {
            state.push(
                Notice {
                    id,
                    app: "Files".to_owned(),
                    summary: format!("n{id}"),
                    body: String::new(),
                },
                now,
            );
        }
        assert!(state.close(2));
        assert_eq!(state.ids(), vec![3, 1]);
        state.clear();
        assert!(state.ids().is_empty());
    }
}
