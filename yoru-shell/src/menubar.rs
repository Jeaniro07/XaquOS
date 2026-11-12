//! The menu bar (spec §5): a strip of glass across the top of the screen,
//! drawn by `ui/menubar.tole`. It follows the focus from hyprix's events,
//! shows the standard menus for the focused program (`menus.rs`), the
//! search and the time, and does what is picked. It runs on a thread of
//! the dock's program (spec §2), or alone with `--menubar`.

use std::os::fd::AsRawFd as _;
use std::rc::Rc;
use std::time::Duration;

use compositor_toolkit::{Event, Rect};
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::hypr::{self, Hypr};
use crate::json::Json;
use crate::menus::{self, Does, Entry, Focused, Menu};
use crate::ui::{MenuBar, MenuBarSignal, MenuLine, TopMenu};
use crate::window::say;

/// The surface's namespace: Yoru's theme blurs behind every `yoru-` one.
pub const NAMESPACE: &str = "yoru-menubar";
/// The strip's height, which windows keep clear of.
pub const HEIGHT: u32 = 24;
/// The words' fonts, as `Nocturne.sans` has them.
const SANS: &str = "Inter Variable, Inter, sans-serif";

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// An open menu's height below the strip, its lines and its edges.
#[must_use]
pub fn drop_height(entries: &[Entry]) -> u32 {
    10 + entries
        .iter()
        .map(|entry| if matches!(entry, Entry::Line) { 9 } else { 24 })
        .sum::<u32>()
}

/// The surface's height with `entries` dropped.
#[must_use]
pub fn open_height(entries: &[Entry]) -> u32 {
    26 + drop_height(entries) + 4
}

/// The layer `height` tall; windows keep clear of the strip alone.
fn layer(height: u32) -> LayerOptions {
    LayerOptions {
        output: None,
        layer: Layer::Top,
        namespace: NAMESPACE.to_owned(),
        size: (0, height),
        anchor: Anchor::TOP.with(Anchor::LEFT).with(Anchor::RIGHT),
        exclusive_zone: i32::try_from(HEIGHT).unwrap_or(24),
        margin: Margin {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
    }
}

/// What a press in the bar asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Menu `n` dropped.
    Open(usize),
    /// The open menu closed.
    Close,
    /// An entry's deed.
    Do(Does),
    /// The search.
    Search,
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The bar.
    pub bar: MenuBar,
    menus: Vec<Menu>,
    open: Option<usize>,
    places: Vec<(f32, f32)>,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl View {
    /// The tree, drawn with `fonts`; `None` when the markup's items could
    /// not be made.
    #[must_use]
    pub fn new(fonts: tole::Fonts) -> Option<Self> {
        let mut ui = Ui::new(fonts);
        let bar = MenuBar::new(&mut ui, None)?;
        bar.search_icon
            .set(&mut ui, s(crate::look::icon("magnifying-glass")));
        Some(View {
            ui,
            bar,
            menus: Vec::new(),
            open: None,
            places: Vec::new(),
        })
    }

    /// Where the surface, `width` by `height`, takes presses: with a
    /// menu open, the strip with the two rows under it and the dropdown
    /// (`ui/menubar.tole` puts it at its title's x, 26 down, 260 wide), so
    /// the clear band beside it lets presses through to the windows; with
    /// none, the whole surface.
    ///
    /// A press outside the menu goes to another client, and the compositor
    /// tells this one nothing of it (the pointer's leave is the tree's), so
    /// the menu closes on a pick, on its title, on the focus moving -- a
    /// press on another window -- but not on a press on the wallpaper or
    /// the focused window. Upgrade path: an xdg_popup with a grab, or a
    /// leave the program sees.
    #[must_use]
    pub fn input_region(&self, width: u32, height: u32) -> Option<Vec<Rect>> {
        let x = self.places.get(self.open?)?.0.round() as i32;
        let width = i32::try_from(width).unwrap_or(i32::MAX);
        let height = i32::try_from(height).unwrap_or(i32::MAX);
        Some(vec![
            Rect {
                x: 0,
                y: 0,
                width,
                height: 26,
            },
            Rect {
                x,
                y: 26,
                width: 260,
                height: height.saturating_sub(30),
            },
        ])
    }

    /// The bar's size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.bar.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Show `menus`, `open` the one dropped, and `clock`.
    pub fn show(&mut self, menus: Vec<Menu>, open: Option<usize>, clock: &str) {
        let ui = &mut self.ui;
        let regular = tole::Font::new(SANS, 13.0, 500);
        let bold = tole::Font::new(SANS, 13.0, 700);
        self.places = menus::place(&menus, |label, heavy| {
            ui.text_width(if heavy { &bold } else { &regular }, label)
        });
        let tops = menus
            .iter()
            .zip(&self.places)
            .enumerate()
            .map(|(i, (menu, &(x, w)))| TopMenu {
                label: s(&menu.label),
                bold: menu.bold,
                on: open == Some(i),
                x,
                w,
            })
            .collect::<Vec<_>>();
        self.bar.menus.set(ui, tops);
        let lines = open
            .and_then(|i| menus.get(i))
            .map(|menu| {
                menu.entries
                    .iter()
                    .map(|entry| match entry {
                        Entry::Line => MenuLine {
                            label: s(""),
                            keys: s(""),
                            line: true,
                            enabled: false,
                        },
                        Entry::Item { label, keys, does } => MenuLine {
                            label: s(label),
                            keys: s(keys),
                            line: false,
                            enabled: does.is_some(),
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.bar.lines.set(ui, lines);
        self.bar.open.set(ui, open.is_some());
        let drop_x = open
            .and_then(|i| self.places.get(i))
            .map_or(0.0, |place| place.0);
        self.bar.drop_x.set(ui, drop_x);
        self.bar.clock.set(ui, s(clock));
        self.menus = menus;
        self.open = open;
    }

    /// What the tree asked since the last call.
    pub fn asked(&mut self) -> Vec<Asked> {
        let emitted = self.ui.take_signals();
        let mut out = Vec::new();
        for each in &emitted {
            let Some(signal) = self.bar.signal(each) else {
                continue;
            };
            match signal {
                MenuBarSignal::Pick(i) => out.push(match usize::try_from(i) {
                    Ok(i) if Some(i) != self.open => Asked::Open(i),
                    _ => Asked::Close,
                }),
                MenuBarSignal::Choose(i) => {
                    let does = usize::try_from(i).ok().and_then(|i| {
                        let menu = self.open.and_then(|m| self.menus.get(m))?;
                        match menu.entries.get(i)? {
                            Entry::Item { does, .. } => does.clone(),
                            Entry::Line => None,
                        }
                    });
                    out.extend(does.map(Asked::Do));
                }
                MenuBarSignal::Dismiss => out.push(Asked::Close),
                MenuBarSignal::Search => out.push(Asked::Search),
            }
        }
        out
    }
}

/// The window with the focus, asked of hyprix.
fn ask_focused(hypr: Option<&Hypr>) -> Option<Focused> {
    hypr?
        .ask("j/activewindow")
        .and_then(|text| Json::parse(&text))
        .and_then(|json| menus::focused(&json))
}

/// The time as the bar shows it.
fn clock_now() -> String {
    let clock = crate::model::Clock::at(crate::window::now());
    format!("{}  {}", clock.date, clock.time)
}

/// Dispatch each of `lines`, said.
fn dispatch_all(hypr: &Hypr, lines: &[String]) {
    for what in lines {
        let done = hypr.dispatch(what);
        say(&format!(
            "menu: dispatch {what}: {}",
            if done { "ok" } else { "refused" }
        ));
    }
}

/// Do an entry's deed. A press on the bar may have moved the focus, so the
/// focused window is focused again first.
fn done(hypr: Option<&Hypr>, focused: Option<&Focused>, does: Does) {
    let Some(hypr) = hypr else { return };
    let again = focused.map(|f| format!("focuswindow address:{}", f.address));
    let lines: Vec<String> = match does {
        Does::Dispatch(what) => again.into_iter().chain(std::iter::once(what)).collect(),
        Does::Hide => {
            let desktop = hypr
                .ask("j/activeworkspace")
                .and_then(|text| Json::parse(&text))
                .and_then(|json| json.get("id").and_then(Json::int))
                .unwrap_or(1);
            let clients = hypr
                .ask("j/clients")
                .and_then(|text| Json::parse(&text))
                .unwrap_or(Json::Null);
            focused.map_or_else(Vec::new, |f| menus::hide(&clients, &f.class, desktop))
        }
    };
    dispatch_all(hypr, &lines);
}

/// The search: fuzzel in Yoru's look, as SUPER Space starts it.
fn search(hypr: Option<&Hypr>) {
    let Some(hypr) = hypr else { return };
    let what = if std::path::Path::new(crate::window::PALETTE).exists() {
        format!("exec /bin/fuzzel --config {}", crate::window::PALETTE)
    } else {
        "exec /bin/fuzzel".to_owned()
    };
    dispatch_all(hypr, &[what]);
}

/// Run the menu bar until the compositor goes away; the exit status.
#[must_use]
pub fn run() -> i32 {
    let hypr = Hypr::from_env();
    let launchers = crate::model::launchers(&crate::window::installed());
    let own = crate::window::own_path();
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the menu bar's tree could not be made");
        return 1;
    };
    let mut theme = crate::theme::Live::new();
    theme.tokens.apply(&mut view.ui);
    let standard = |focused: Option<&Focused>| {
        let name = focused.map(|f| menus::app_name(&f.class, &launchers));
        menus::standard(name.as_deref(), &own)
    };
    let mut focused = ask_focused(hypr.as_ref());
    let mut clock = clock_now();
    let mut open: Option<usize> = None;
    let mut height = HEIGHT;
    view.show(standard(focused.as_ref()), open, &clock);
    let options = WindowOptions {
        title: "Yoru menu bar".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (1280, HEIGHT),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer(HEIGHT)),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.bar.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!(
                "err: no compositor to put the menu bar on: {error}"
            ));
            return 1;
        }
    };
    let mut events = hypr.as_ref().and_then(Hypr::events);
    let watch = events
        .as_ref()
        .map(|stream| window.client().watch_fd(stream.as_raw_fd()));
    let tick = window
        .client()
        .add_timer(Duration::from_secs(1), Some(Duration::from_secs(1)));
    let mut pending = String::new();
    let mut up = false;
    loop {
        let happened = match window.dispatch(&mut view.ui, None) {
            Ok(happened) => happened,
            Err(error) => {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        };
        if !up && let Some((w, h)) = window.size() {
            up = true;
            say(&format!("the menu bar is up, {w}x{h}"));
        }
        let mut changed = false;
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Timer(timer) if timer == tick => {
                    let now = clock_now();
                    if now != clock {
                        clock = now;
                        changed = true;
                    }
                    if let Some(tokens) = theme.poll() {
                        tokens.apply(&mut view.ui);
                        changed = true;
                    }
                }
                Event::Readable(id) if Some(id) == watch => {
                    if let Some(stream) = events.as_mut()
                        && !crate::window::drain(stream, &mut pending)
                    {
                        say("menu bar: hyprix's event socket closed");
                        watch
                            .into_iter()
                            .for_each(|w| window.client().unwatch_fd(w));
                        events = None;
                    }
                    let mut moved = false;
                    while let Some(end) = pending.find('\n') {
                        let line: String = pending.drain(..=end).collect();
                        moved |= hypr::wanted(line.trim_end());
                    }
                    let now = if moved {
                        ask_focused(hypr.as_ref())
                    } else {
                        focused.clone()
                    };
                    if now != focused {
                        focused = now;
                        open = None;
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        for asked in view.asked() {
            match asked {
                Asked::Open(i) => open = Some(i),
                Asked::Close => open = None,
                Asked::Search => {
                    open = None;
                    search(hypr.as_ref());
                }
                Asked::Do(does) => {
                    open = None;
                    done(hypr.as_ref(), focused.as_ref(), does);
                }
            }
            changed = true;
        }
        if changed {
            let menus = standard(focused.as_ref());
            let wanted = open
                .and_then(|i| menus.get(i))
                .map_or(HEIGHT, |menu| open_height(&menu.entries));
            if wanted != height {
                height = wanted;
                window.set_layer(layer(height));
            }
            view.show(menus, open, &clock);
            let width = window.size().map_or(1280, |(w, _)| w);
            window.set_input_region(view.input_region(width, height).as_deref());
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
    }
}

/// The menu bar on a thread of the dock's program (spec §2: one program,
/// several surfaces), with its own tree and its own connection to the
/// compositor. Upgrade path: one Wayland connection for both surfaces,
/// should tole's `Window` learn to share a client.
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("menubar".to_owned())
        .spawn(|| {
            let code = run();
            say(&format!("the menu bar stopped: {code}"));
        });
    if let Err(error) = spawned {
        say(&format!("err: the menu bar's thread: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// While a menu is open the layer is the screen's width and the
    /// menu's height, but takes presses on the strip (and the two rows
    /// under it) and on the dropdown alone, so the clear band beside the
    /// dropdown lets them through to the windows; closed, the whole
    /// surface, which is the strip.
    #[test]
    fn an_open_menu_takes_presses_on_the_strip_and_the_dropdown_alone() {
        let menus = menus::standard(Some("Terminal"), "/bin/yoru-shell");
        let mut view = View::new(crate::look::fonts()).expect("tree");
        view.resize((1024, HEIGHT));
        view.show(menus.clone(), None, "Thu 12 Nov  21:40");
        assert_eq!(view.input_region(1024, HEIGHT), None);
        let height = open_height(&menus[2].entries);
        view.show(menus.clone(), Some(2), "Thu 12 Nov  21:40");
        let x = view.places[2].0.round() as i32;
        let region = view.input_region(1024, height).expect("a region");
        assert_eq!(
            region,
            vec![
                Rect {
                    x: 0,
                    y: 0,
                    width: 1024,
                    height: 26
                },
                Rect {
                    x,
                    y: 26,
                    width: 260,
                    height: i32::try_from(height).unwrap() - 30
                },
            ]
        );
    }

    #[test]
    fn the_bar_draws_glass_and_words() {
        let frame = crate::cli::render_menubar(1024, None, 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (1024, HEIGHT));
        let at = |x: u32, y: u32| {
            let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
            (pixel.red(), pixel.green(), pixel.blue())
        };
        let (r, g, b) = at(512, 3);
        let (lo, hi) = (r.min(g).min(b), r.max(g).max(b));
        assert!(lo >= 0xd8 && hi - lo <= 0x18, "the glass: {r} {g} {b}");
        let dark = |xs: std::ops::Range<u32>| {
            xs.flat_map(|x| (4..20).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    let (r, g, b) = at(x, y);
                    u32::from(r) + u32::from(g) + u32::from(b) < 0x150
                })
                .count()
        };
        assert!(dark(4..160) >= 30, "the menus' words at the left");
        assert!(dark(1024 - 160..1024 - 4) >= 30, "the clock at the right");
    }

    #[test]
    fn an_open_menu_drops_white_below_its_title() {
        let menus = menus::standard(Some("Terminal"), "/bin/yoru-shell");
        let frame = crate::cli::render_menubar(1024, Some(2), 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        assert_eq!(pixmap.height(), open_height(&menus[2].entries));
        let white = (0..1024)
            .flat_map(|x| (40..120).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
                pixel.red() >= 0xf5 && pixel.green() >= 0xf5 && pixel.blue() >= 0xf5
            })
            .count();
        assert!(white > 5000, "the dropped menu: {white}");
    }

    #[test]
    fn a_press_opens_a_menu_and_a_pick_does_its_entry() {
        let menus = menus::standard(Some("Terminal"), "/bin/yoru-shell");
        let mut view = View::new(crate::look::fonts()).expect("tree");
        view.resize((1024, HEIGHT));
        view.show(menus.clone(), None, "Thu 12 Nov  21:40");
        view.ui.update();
        let (x, w) = view.places[2];
        view.ui.pointer_move(x + w / 2.0, 12.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        assert_eq!(view.asked(), vec![Asked::Open(2)]);
        view.resize((1024, open_height(&menus[2].entries)));
        view.show(menus.clone(), Some(2), "Thu 12 Nov  21:40");
        view.ui.update();
        // Minimize, the first line: 26 down, 5 in, half a line.
        view.ui.pointer_move(x + 40.0, 26.0 + 5.0 + 12.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        assert_eq!(
            view.asked(),
            vec![Asked::Do(Does::Dispatch("minimize".to_owned()))]
        );
    }
}
