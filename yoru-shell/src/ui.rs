//! The dock's tree, in tole (Ferrix's `docs/UI-FRAMEWORK.md`): the markup
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

    /// What the tree asked since the last call.
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
