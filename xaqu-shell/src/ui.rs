//! The dock's tree, in tole (Xaqu's `docs/UI-FRAMEWORK.md`): the markup
//! under `ui/` draws it; this module hands it the [`State`] after every
//! change and turns what was pressed back into [`Action`]s.

use std::rc::Rc;

use tole::{Element as _, Ui};

use crate::look::icon;
use crate::model::State;

tole::include_ui!();

/// The dock's height, its glass included, in logical pixels.
pub const HEIGHT: u32 = 68;
/// A tile's side.
pub const TILE: u32 = 48;
/// Between tiles.
pub const GAP: u32 = 8;
/// Round the tiles, inside the glass.
pub const PAD: u32 = 10;

/// How wide the dock is for `state`: its tiles and the stash's, the gaps,
/// and the line before the stash (1 wide, a gap each side).
#[must_use]
pub fn width(state: &State) -> u32 {
    let tiles = u32::try_from(state.launchers.len() + state.others.len() + 1).unwrap_or(1);
    2 * PAD + tiles * (TILE + GAP) + 1
}

/// `state` made to fit `room` pixels: the programs without a launcher that
/// do not fit are left off the end, the launchers and the stash kept.
/// Upgrade path: smaller tiles, or an overflow tile, before any is left
/// off.
pub fn fit(state: &mut State, room: u32) {
    while width(state) > room && state.others.pop().is_some() {}
}

/// The focused screen's width in logical pixels from `j/monitors`, or the
/// first screen's where none is focused.
#[must_use]
pub fn screen_width(list: &crate::json::Json) -> Option<u32> {
    let screens = list.items();
    let screen = screens
        .iter()
        .find(|screen| {
            screen
                .get("focused")
                .is_some_and(|focused| matches!(focused, crate::json::Json::Bool(true)))
        })
        .or_else(|| screens.first())?;
    let pixels = screen.get("width").and_then(crate::json::Json::num)?;
    let scale = screen
        .get("scale")
        .and_then(crate::json::Json::num)
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0);
    let logical = (pixels / scale).round();
    (logical >= 0.0 && logical <= f64::from(u32::MAX)).then_some(logical as u32)
}

/// What a press in the dock asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Launcher `n`: its window brought, or the program started.
    Launch(usize),
    /// Other program `n`: its window brought.
    Other(usize),
    /// The stash's panel.
    Stash,
    /// The pointer came over a tile (its title), or left one (empty).
    Hover(String),
}

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// The rows of the quick settings: the sidebar's tab and the Control
/// Center draw the same ones.
pub struct QuickRows {
    /// Brightness, volume, microphone.
    pub sliders: Vec<QuickSlider>,
    /// The six tiles.
    pub tiles: Vec<QuickTile>,
    /// The theme colours.
    pub accents: Vec<QuickAccent>,
    /// The appearances.
    pub looks: Vec<QuickLook>,
}

/// The rows `quick` shows.
#[must_use]
pub fn quick_rows(quick: &crate::quick::Quick) -> QuickRows {
    let slider = |name: &str, label: &str, value: u32, shown: String, enabled: bool| QuickSlider {
        icon: s(icon(name)),
        label: s(label),
        value: value as f32,
        shown: s(shown),
        enabled,
    };
    let brightness = quick
        .backlight
        .as_ref()
        .map(crate::quick::Backlight::percent);
    let sound_slider =
        |name: &str, label: &str, device: Option<&crate::sound::Device>, none: &str| QuickSlider {
            icon: s(icon(if device.is_some_and(|d| d.muted) {
                if name == "microphone" {
                    "microphone-slash"
                } else {
                    "speaker-slash"
                }
            } else {
                name
            })),
            label: s(label),
            value: device.map_or(0.0, |d| d.percent.min(100) as f32),
            shown: s(device.map_or_else(
                || none.to_owned(),
                |d| {
                    if d.muted {
                        "Muted".to_owned()
                    } else {
                        format!("{}%", d.percent)
                    }
                },
            )),
            enabled: device.is_some(),
        };
    let sliders = vec![
        slider(
            "sun",
            "Brightness",
            brightness.unwrap_or(0),
            brightness.map_or_else(|| "No backlight".to_owned(), |p| format!("{p}%")),
            brightness.is_some(),
        ),
        sound_slider(
            "speaker-high",
            "Volume",
            quick.sink.as_ref(),
            quick
                .sound
                .as_ref()
                .map_or("No sound card", |_| "No sound server"),
        ),
        sound_slider(
            "microphone",
            "Microphone",
            quick.source.as_ref(),
            quick
                .sound
                .as_ref()
                .map_or("No microphone", |_| "No sound server"),
        ),
    ];
    let on_off = |on: bool| if on { "On" } else { "Off" };
    let tile = |name: &str, label: &str, sub: &str, on: bool, enabled: bool| QuickTile {
        icon: s(icon(name)),
        label: s(label),
        sub: s(sub),
        on,
        enabled,
    };
    let tiles = vec![
        tile(
            "wifi-high",
            "Wi-Fi",
            if quick.wireless {
                "Not yet"
            } else {
                "None found"
            },
            false,
            false,
        ),
        tile("bluetooth", "Bluetooth", "Not yet", false, false),
        tile(
            "bell-slash",
            "Do not disturb",
            on_off(quick.quiet),
            quick.quiet,
            true,
        ),
        tile("moon", "Night light", "Not yet", false, false),
        tile(
            "crosshair-simple",
            "Focus mode",
            on_off(quick.focus),
            quick.focus,
            true,
        ),
        tile(
            "squares-four",
            "Auto-tile",
            if quick.tiled { "Tiling" } else { "Off" },
            quick.tiled,
            true,
        ),
    ];
    let theme = &quick.theme;
    let accents = crate::theme::Mode::all()
        .into_iter()
        .map(|mode| {
            let hue = match mode {
                crate::theme::Mode::Named(n) => {
                    crate::theme::TRIADS.get(n).map_or(289.0, |t| t.1[0])
                }
                crate::theme::Mode::Wallpaper => 289.0,
            };
            let [r, g, b] = crate::quick::oklch(0.72, (0.14 * theme.vibrance).min(0.26), hue);
            QuickAccent {
                name: s(mode.name()),
                colour: tole::Color::rgba(r, g, b, 0xff),
                icon: s(if mode == crate::theme::Mode::Wallpaper {
                    icon("image")
                } else {
                    String::new()
                }),
                on: theme.mode == mode,
            }
        })
        .collect();
    let looks = crate::theme::Appearance::ALL
        .into_iter()
        .map(|look| QuickLook {
            name: s(look.name()),
            on: theme.appearance == look,
        })
        .collect();
    QuickRows {
        sliders,
        tiles,
        accents,
        looks,
    }
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The dock.
    pub dock: Dock,
    /// The state last shown.
    shown: Option<State>,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl View {
    /// The tree, drawn with `fonts`; `None` when the markup's items could
    /// not be made, a fault of this program.
    #[must_use]
    pub fn new(fonts: tole::Fonts) -> Option<Self> {
        let mut ui = Ui::new(fonts);
        let dock = Dock::new(&mut ui, None)?;
        dock.stash_icon.set(&mut ui, s(icon("archive")));
        Some(View {
            ui,
            dock,
            shown: None,
        })
    }

    /// The dock's size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.dock.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree `state`.
    pub fn show(&mut self, state: &State) {
        if self.shown.as_ref() == Some(state) {
            return;
        }
        let ui = &mut self.ui;
        let apps = state
            .launchers
            .iter()
            .map(|launcher| AppRow {
                glyph: s(icon(launcher.icon)),
                letter: s(""),
                tile: launcher.tile as i32,
                title: s(&launcher.title),
                running: launcher.running,
            })
            .chain(state.others.iter().map(|other| {
                AppRow {
                    glyph: s(""),
                    letter: s(other
                        .name
                        .chars()
                        .next()
                        .map(|first| first.to_uppercase().collect::<String>())
                        .unwrap_or_default()),
                    tile: 4,
                    title: s(&other.name),
                    running: true,
                }
            }))
            .collect::<Vec<_>>();
        self.dock.apps.set(ui, apps);
        self.dock
            .stash_badge
            .set(ui, s(crate::stash::badge(state.stashed)));
        self.shown = Some(state.clone());
    }

    /// The state last shown, which the tree's indices are of.
    #[must_use]
    pub fn shown(&self) -> Option<&State> {
        self.shown.as_ref()
    }

    /// What the tree asked since the last call: indices into
    /// [`View::shown`], which a refresh since does not move.
    pub fn actions(&mut self) -> Vec<Action> {
        let emitted = self.ui.take_signals();
        let launchers = self.shown.as_ref().map_or(0, |state| state.launchers.len());
        let mut actions = Vec::new();
        for each in &emitted {
            let Some(signal) = self.dock.signal(each) else {
                continue;
            };
            let action = match signal {
                DockSignal::App(i) => usize::try_from(i).ok().map(|i| {
                    if i < launchers {
                        Action::Launch(i)
                    } else {
                        Action::Other(i - launchers)
                    }
                }),
                DockSignal::Stash => Some(Action::Stash),
                DockSignal::Hover(title) => Some(Action::Hover(title.to_string())),
            };
            actions.extend(action);
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::{View, fit, screen_width, width};
    use crate::json::Json;
    use crate::model::{Other, State, launchers};

    fn other(n: usize) -> Other {
        Other {
            class: format!("org.example.App{n:02}"),
            name: format!("App{n:02}"),
            window: format!("0x{n:x}"),
        }
    }

    /// The dock is never wider than the room it is given: the programs
    /// without a launcher that do not fit are left off, the launchers and
    /// the stash kept.
    #[test]
    fn the_dock_drops_the_tiles_that_do_not_fit() {
        let mut state = State::new(launchers(&[]));
        state.others = (0..40).map(other).collect();
        assert!(width(&state) > 1268);
        fit(&mut state, 1268);
        assert!(width(&state) <= 1268, "{} wide", width(&state));
        assert_eq!(state.launchers.len(), 1, "the terminal stays");
        assert_eq!(state.others.first(), Some(&other(0)));
        let kept = state.others.len();
        fit(&mut state, 4000);
        assert_eq!(state.others.len(), kept, "room to spare drops none");
    }

    /// The focused screen's width in logical pixels, from `j/monitors`.
    #[test]
    fn the_screen_is_read_in_logical_pixels() {
        let list = Json::parse(
            r#"[{"name":"DP-2","width":3840,"scale":2.0,"focused":false},
               {"name":"DP-1","width":2560,"scale":2.0,"focused":true}]"#,
        )
        .expect("json");
        assert_eq!(screen_width(&list), Some(1280));
        let one = Json::parse(r#"[{"name":"Virtual-1","width":1280,"scale":1.0}]"#).expect("json");
        assert_eq!(
            screen_width(&one),
            Some(1280),
            "the only one, focused or not"
        );
    }

    /// A press is read against the dock as it was drawn: a refresh between
    /// the drawing and the press does not move which tile it was.
    #[test]
    fn a_press_is_read_against_the_dock_it_was_drawn_from() {
        let mut view = View::new(crate::look::fonts()).expect("tree");
        let mut drawn = State::new(launchers(&[]));
        drawn.others = vec![other(1), other(2)];
        view.show(&drawn);
        let mut now = drawn.clone();
        now.others.remove(0);
        assert_eq!(view.shown(), Some(&drawn));
        assert_ne!(view.shown(), Some(&now));
    }

    use super::quick_rows;
    use crate::quick::Quick;

    /// With nothing found the rows still stand, the unavailable ones off.
    #[test]
    fn the_quick_rows_stand_with_nothing_found() {
        let rows = quick_rows(&Quick::default());
        assert_eq!(rows.sliders.len(), 3);
        assert!(rows.sliders.iter().all(|r| !r.enabled));
        assert_eq!(rows.tiles.len(), 6);
        let on: Vec<bool> = rows.tiles.iter().map(|t| t.enabled).collect();
        assert_eq!(on, [false, false, true, false, true, true]);
        assert_eq!(rows.looks.len(), crate::theme::Appearance::ALL.len());
        assert_eq!(rows.accents.len(), crate::theme::Mode::all().len());
    }

    /// A tile is on when its setting is, and one look and one accent are marked.
    #[test]
    fn the_chosen_look_is_the_one_marked() {
        let quick = Quick {
            quiet: true,
            ..Quick::default()
        };
        let rows = quick_rows(&quick);
        assert!(rows.tiles[2].on);
        assert_eq!(rows.looks.iter().filter(|l| l.on).count(), 1);
        assert_eq!(rows.accents.iter().filter(|a| a.on).count(), 1);
    }
}
