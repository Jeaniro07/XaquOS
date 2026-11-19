//! The dock on the screen: the state and the tree joined to a layer-shell
//! surface by tole's window, and kept up to date from hyprix.
//!
//! One loop. hyprix's event socket is watched, and an event that moves a
//! workspace or a window has the dock ask again (`j/clients`); a timer
//! each second looks at the stash and the theme; a press becomes a
//! dispatcher. What happens is said on
//! standard output, a line each, which is how a boot that drives the dock
//! knows what it did.

use std::io::Read as _;
use std::os::fd::AsRawFd as _;
use std::time::{Duration, SystemTime};

use compositor_toolkit::Event;
use tole::Color;
use tole::Element as _;
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};

use crate::hypr::{self, Hypr};
use crate::json::Json;
use crate::model::{self, State};
use crate::ui::{Action, HEIGHT, View};

/// The surface's namespace, which `layerrule`s match: Xaqu's theme blurs
/// what is behind every `xaqu-` surface.
pub const NAMESPACE: &str = "xaqu-dock";

/// How far above the screen's foot the dock floats.
const MARGIN: i32 = 6;

/// Say one line of what the program did.
pub fn say(text: &str) {
    use std::io::Write as _;
    let mut out = std::io::stdout();
    let _ = writeln!(out, "xaqu-shell: {text}");
    let _ = out.flush();
}

/// Seconds since 1970 in the local zone.
pub(crate) fn now() -> i64 {
    let utc = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
    utc + model::offset(std::env::var("TZ").ok().as_deref())
}

/// The applications installed, as `(id, command, window class)`.
pub(crate) fn installed() -> Vec<(String, String, Option<String>)> {
    let var = |name: &str| std::env::var(name).ok();
    let search = compositor_desktop::entry::Search::listing(&var);
    compositor_desktop::entry::find_programs(&search)
        .into_iter()
        .filter(|app| app.visible)
        .filter_map(|app| {
            let exec = model::command(app.exec.as_deref()?);
            Some((app.id?, exec, app.app_id))
        })
        .collect()
}

/// The applications installed, as Spotlight and the Launchpad show them:
/// exec lines with the codes stripped, the names as they are written, and
/// the icons the dock's mapping widened gives.
pub(crate) fn entries() -> Vec<crate::search::App> {
    let var = |name: &str| std::env::var(name).ok();
    let search = compositor_desktop::entry::Search::listing(&var);
    compositor_desktop::entry::find_programs(&search)
        .into_iter()
        .filter(|app| app.visible)
        .filter_map(|app| {
            let exec = model::command(app.exec.as_deref()?);
            let name = app.title_string();
            let id = app.id?;
            let key = id.strip_suffix(".desktop").unwrap_or(&id);
            Some(crate::search::App {
                exec,
                title: if name.is_empty() { words(key) } else { name },
                icon: model::icon_for(key, app.app_id.as_deref()),
            })
        })
        .collect()
}

/// An id's words, as a name: `google-chrome` gives `Google Chrome`.
fn words(id: &str) -> String {
    let mut name = String::new();
    for word in id.split(['-', '_', '.']).filter(|word| !word.is_empty()) {
        if !name.is_empty() {
            name.push(' ');
        }
        let mut letters = word.chars();
        if let Some(first) = letters.next() {
            name.extend(first.to_uppercase());
            name.push_str(letters.as_str());
        }
    }
    name
}

/// This program, to start its wallpaper switcher with.
pub(crate) fn own_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
        .unwrap_or_else(|| "/bin/xaqu-shell".to_owned())
}

/// Ask hyprix again for the windows.
fn refresh(hypr: Option<&Hypr>, state: &mut State) {
    let Some(hypr) = hypr else {
        return;
    };
    if let Some(list) = hypr.ask("j/clients").and_then(|text| Json::parse(&text)) {
        state.clients(&list);
    }
    // No wider than the screen, a margin each side.
    let screen = hypr
        .ask("j/monitors")
        .and_then(|text| Json::parse(&text))
        .and_then(|list| crate::ui::screen_width(&list));
    if let Some(screen) = screen {
        let margins = 2 * u32::try_from(MARGIN).unwrap_or(0);
        crate::ui::fit(state, screen.saturating_sub(margins));
    }
}

/// Do what a press asked.
fn act(hypr: Option<&Hypr>, state: &State, action: Action) {
    let dispatch = |what: &str| {
        let done = hypr.is_some_and(|hypr| hypr.dispatch(what));
        say(&format!(
            "dispatch {what}: {}",
            if done { "ok" } else { "refused" }
        ));
    };
    match action {
        // A window of it anywhere, minimized too, is brought; focusing a
        // minimized window takes it out of special:minimized (§8.1).
        Action::Launch(at) => {
            if let Some(launcher) = state.launchers.get(at) {
                match &launcher.window {
                    Some(address) => dispatch(&format!("focuswindow address:{address}")),
                    None => dispatch(&format!("exec {}", launcher.exec)),
                }
            }
        }
        Action::Other(at) => {
            if let Some(other) = state.others.get(at) {
                dispatch(&format!("focuswindow address:{}", other.window));
            }
        }
        Action::Stash => dispatch(&format!("exec {} --stash-panel", own_path())),
        // Said for the tooltip's sake, and for a boot that finds a tile by
        // moving the pointer along the dock.
        Action::Hover(title) => {
            if !title.is_empty() {
                say(&format!("over {title}"));
            }
        }
    }
}

/// Read what the event socket has into `pending`; false when it closed.
pub(crate) fn drain(stream: &mut std::os::unix::net::UnixStream, pending: &mut String) -> bool {
    let mut bytes = [0u8; 4096];
    loop {
        match stream.read(&mut bytes) {
            Ok(0) => return false,
            Ok(n) => pending.push_str(&String::from_utf8_lossy(bytes.get(..n).unwrap_or_default())),
            Err(_) => return true,
        }
    }
}

/// A drag over the dock: the tree follows it, so the Stash button lights
/// under it, and files let go there go into the stash.
fn dragged(view: &mut View, state: &mut State, drag: crate::drop::Drag) {
    use crate::drop::Drag;
    match drag {
        Drag::Over(x, y, _) => {
            view.ui.pointer_move(x, y);
        }
        Drag::Left => view.ui.pointer_leave(),
        Drag::Dropped(x, y, text) => {
            view.ui.pointer_move(x, y);
            view.ui.update();
            let over = view.ui.get(view.dock.over_stash);
            view.ui.pointer_leave();
            let files = crate::stash::uris(&text);
            if !over {
                say(&format!(
                    "drop: {} files let go off the Stash button",
                    files.len()
                ));
                return;
            }
            let Some(dir) = crate::stash::folder() else {
                return;
            };
            match crate::stash::add(&dir, &files) {
                Ok(n) => say(&format!(
                    "stash: {n} dropped, {} in all",
                    crate::stash::count(&dir)
                )),
                Err(error) => say(&format!("stash: {}: {error}", dir.display())),
            }
            state.stashed = crate::stash::count(&dir);
        }
    }
}

/// The window frames in `tokens`, kept by the compositor as Settings keeps
/// its choices; a refusal said.
fn frames(hypr: Option<&Hypr>, tokens: &crate::theme::Tokens) {
    let Some(hypr) = hypr else { return };
    for line in tokens.frame_requests() {
        let said = hypr.ask(&line).unwrap_or_default();
        if said.trim() != "ok" {
            say(&format!("theme: {line}: {}", said.trim()));
        }
    }
}

/// Run the dock until the compositor goes away; the exit status.
#[must_use]
pub fn run() -> i32 {
    let hypr = Hypr::from_env();
    if hypr.is_none() {
        say(
            "no hyprix here (HYPRLAND_INSTANCE_SIGNATURE is not set): the workspaces stay as they are",
        );
    }
    let mut state = State::new(model::launchers(&installed()));
    state.stashed = crate::stash::folder().map_or(0, |dir| crate::stash::count(&dir));
    refresh(hypr.as_ref(), &mut state);
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the dock's tree could not be made");
        return 1;
    };
    let mut theme = crate::theme::Live::new();
    theme.tokens.apply(&mut view.ui);
    view.show(&state);
    // Anchored to the foot alone, so the compositor centres it; as wide as
    // its tiles, and set again when a program comes or goes.
    let layer = |width: u32| LayerOptions {
        output: None,
        layer: Layer::Top,
        namespace: NAMESPACE.to_owned(),
        size: (width, HEIGHT),
        anchor: Anchor::BOTTOM,
        exclusive_zone: i32::try_from(HEIGHT).unwrap_or(68) + MARGIN,
        margin: Margin {
            top: 0,
            right: 0,
            bottom: MARGIN,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
    };
    let mut width = crate::ui::width(&state);
    let options = WindowOptions {
        title: "Xaqu".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (width, HEIGHT),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer(width)),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.dock.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!("err: no compositor to put the dock on: {error}"));
            return 1;
        }
    };
    // Files dropped on the Stash button.
    let mut target = crate::drop::Target::bind(window.client());
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
            say(&format!(
                "the dock is up, {w}x{h}, {} launchers",
                state.launchers.len()
            ));
            // The menu bar after the dock, so the dock is drawn first.
            crate::menubar::start();
            // The notices after the bar: the banners at the top right and
            // the history behind the clock.
            crate::notices::start();
        }
        let mut changed = false;
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Timer(timer) if timer == tick => {
                    let stashed = crate::stash::folder().map_or(0, |dir| crate::stash::count(&dir));
                    if stashed != state.stashed {
                        state.stashed = stashed;
                        changed = true;
                    }
                    if let Some(tokens) = theme.poll() {
                        tokens.apply(&mut view.ui);
                        frames(hypr.as_ref(), &tokens);
                        say(&format!(
                            "dock: {}",
                            crate::theme::said(&theme.theme, &tokens)
                        ));
                        changed = true;
                    }
                }
                Event::Object {
                    object,
                    opcode,
                    ref args,
                    ..
                } if target.is_some() => {
                    let drag = target
                        .as_mut()
                        .and_then(|t| t.event(window.client(), object, opcode, args))
                        .flatten();
                    if let Some(drag) = drag {
                        dragged(&mut view, &mut state, drag);
                        changed = true;
                    }
                }
                Event::Readable(id) if Some(id) == watch => {
                    if let Some(stream) = events.as_mut()
                        && !drain(stream, &mut pending)
                    {
                        say("hyprix's event socket closed");
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
                    if moved {
                        refresh(hypr.as_ref(), &mut state);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        // Read against the dock as drawn: the tiles' indices are of the
        // state last shown, which a refresh above may have changed since.
        for action in view.actions() {
            let hover = matches!(action, Action::Hover(_));
            act(hypr.as_ref(), view.shown().unwrap_or(&state), action);
            // A hover asks nothing of hyprix and changes nothing drawn.
            if !hover {
                refresh(hypr.as_ref(), &mut state);
                changed = true;
            }
        }
        if changed {
            let wanted = crate::ui::width(&state);
            if wanted != width {
                width = wanted;
                window.set_layer(layer(width));
            }
            view.show(&state);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
    }
}
