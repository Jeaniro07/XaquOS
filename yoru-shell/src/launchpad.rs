//! The Launchpad (SUPER L): every installed application as a tile on a
//! grid over the whole screen, eight columns and five rows a page, the
//! pages as dots at the foot, a press on a tile running its application.
//!
//! Started again while it is up, it closes: the wallpaper switcher's
//! [`switch::once`], on `$XDG_RUNTIME_DIR/yoru-launchpad.sock`
//! (`/tmp/yoru-launchpad-UID.sock` with no runtime folder). No keyboard:
//! the pad takes none, and closes on SUPER L again or on a press off a
//! tile.

use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixListener;
use std::rc::Rc;

use compositor_toolkit::Event;
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::hypr::Hypr;
use crate::search::App;
use crate::switch;
use crate::ui::{Launchpad, LaunchpadSignal, PadTile, PageDot};
use crate::window::{entries, say};

/// The surface's namespace.
pub const NAMESPACE: &str = "yoru-launchpad";

// Ponytail: the grid is a fixed eight by five -- no row of leftovers and
// no scrolling; the applications past the fortieth are on the next page.
// Upgrade path: a last page with the room left, when the grid grows.
/// The columns on a page.
pub const COLUMNS: usize = 8;
/// The rows on a page.
pub const ROWS: usize = 5;

/// The applications of one page, sorted, paged and placed.
#[derive(Clone, Debug)]
pub struct Grid {
    apps: Vec<App>,
    columns: usize,
    rows: usize,
    page: usize,
}

impl Grid {
    /// The grid over `apps`, by name, `columns` by `rows` a page.
    #[must_use]
    pub fn new(mut apps: Vec<App>, columns: usize, rows: usize) -> Self {
        apps.sort_by_key(|app| app.title.to_lowercase());
        Grid {
            apps,
            columns,
            rows,
            page: 0,
        }
    }

    /// The tiles a page holds.
    #[must_use]
    pub fn cap(&self) -> usize {
        (self.columns * self.rows).max(1)
    }

    /// How many pages there are.
    #[must_use]
    pub fn pages(&self) -> usize {
        self.apps.len().div_ceil(self.cap())
    }

    /// The page shown.
    #[must_use]
    pub fn page(&self) -> usize {
        self.page
    }

    /// Show page `page`, within the pages there are.
    pub fn set_page(&mut self, page: usize) {
        self.page = page.min(self.pages().saturating_sub(1));
    }

    /// The columns.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// The rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The applications of the page in hand, in tile order.
    pub fn page_apps(&self) -> impl Iterator<Item = &App> {
        self.apps_of(self.page)
    }

    /// The applications of `page`, in tile order.
    fn apps_of(&self, page: usize) -> impl Iterator<Item = &App> {
        self.apps.iter().skip(page * self.cap()).take(self.cap())
    }

    /// The application at tile `at` on the page shown.
    #[must_use]
    pub fn at(&self, at: usize) -> Option<&App> {
        self.apps_of(self.page).nth(at)
    }

    /// The exec line of the application at tile `at` on `page`; none for
    /// a tile past the page.
    #[must_use]
    pub fn run(&self, at: usize, page: usize) -> Option<String> {
        self.apps_of(page).nth(at).map(|app| app.exec.clone())
    }
}

/// Where the tiles go on the page in hand, `(x, y, w, h)`, for a screen
/// `width` by `height`: the grid in the middle, the names under the tiles
/// and the page dots at the foot.
#[must_use]
pub fn place(grid: &Grid, width: f32, height: f32) -> Vec<(f32, f32, f32, f32)> {
    let (cols, rows) = (grid.columns() as f32, grid.rows() as f32);
    let gap = 24.0;
    // The room under a tile for its name, the margin above the grid and
    // the dots' row at the foot.
    let name = 30.0;
    let top_margin = 40.0;
    let foot = 48.0;
    let by_width = (width - gap * (cols - 1.0)) / cols;
    let by_height = (height - top_margin - foot - gap * (rows - 1.0) - rows * name) / rows;
    let tile = by_width.min(by_height).clamp(40.0, 120.0);
    let pitch_x = tile + gap;
    let pitch_y = tile + name + gap;
    let left = ((width - cols * pitch_x + gap) / 2.0).max(0.0);
    let top = ((height - foot - rows * pitch_y + gap) / 2.0).max(top_margin);
    (0..grid.page_apps().count())
        .map(|at| {
            (
                left + (at % grid.columns()) as f32 * pitch_x,
                top + (at / grid.columns()) as f32 * pitch_y,
                tile,
                tile,
            )
        })
        .collect()
}

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// A word's first letter, for the tile where there is no icon.
// Ponytail: an application with no Phosphor name is a letter tile, as the
// dock's is. Upgrade path: the application's own icon, rasterised, once
// tole reads SVG.
fn first(text: &str) -> String {
    text.chars()
        .next()
        .map(|letter| letter.to_uppercase().collect())
        .unwrap_or_default()
}

/// Where the tiles go on the page in hand, as a boot test reads them:
/// `x,y,w,h` for each tile, in tile order.
fn tiles_line(grid: &Grid, width: f32, height: f32) -> String {
    let tiles: Vec<String> = place(grid, width, height)
        .iter()
        .map(|(x, y, w, h)| format!("{x:.0},{y:.0},{w:.0},{h:.0}"))
        .collect();
    format!("launchpad: tiles {}", tiles.join(" "))
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The pad.
    pub pad: Launchpad,
    /// Its size in logical pixels.
    size: (f32, f32),
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
        let pad = Launchpad::new(&mut ui, None)?;
        Some(View {
            ui,
            pad,
            size: (1280.0, 800.0),
        })
    }

    /// Draw in `tokens`.
    pub fn wear(&mut self, tokens: &crate::theme::Tokens) {
        tokens.apply(&mut self.ui);
    }

    /// Its size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        self.size = (width as f32, height as f32);
        let common = self.pad.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree the tiles of the page in hand, and the dots.
    pub fn show(&mut self, grid: &Grid) {
        let placed = place(grid, self.size.0, self.size.1);
        let tiles = grid
            .page_apps()
            .zip(&placed)
            .map(|(app, (x, y, w, h))| PadTile {
                icon: s(crate::look::icon(app.icon)),
                letter: s(first(&app.title)),
                name: s(&app.title),
                x: *x,
                y: *y,
                w: *w,
                h: *h,
            })
            .collect::<Vec<_>>();
        let dots = (0..grid.pages())
            .map(|page| PageDot {
                on: page == grid.page(),
            })
            .collect::<Vec<_>>();
        let ui = &mut self.ui;
        let pad = &self.pad;
        pad.tiles.set(ui, tiles);
        pad.dots.set(ui, dots);
    }

    /// Do what the tree asked since the last call: the exec line to run,
    /// the pad to close, and whether the tree wants drawing again.
    pub fn take(&mut self, grid: &mut Grid) -> (Option<String>, bool, bool) {
        let emitted = self.ui.take_signals();
        let mut exec = None;
        let mut close = false;
        let mut changed = false;
        for each in &emitted {
            let Some(signal) = self.pad.signal(each) else {
                continue;
            };
            match signal {
                LaunchpadSignal::Run(index) => {
                    if let Ok(at) = usize::try_from(index) {
                        exec = grid.run(at, grid.page());
                    }
                }
                LaunchpadSignal::Page(index) => {
                    if let Ok(page) = usize::try_from(index)
                        && page < grid.pages()
                    {
                        grid.set_page(page);
                        changed = true;
                    }
                }
                LaunchpadSignal::Dismissed => close = true,
            }
        }
        (exec, close, changed)
    }
}

/// Run the launchpad until it is closed; the exit status.
#[must_use]
pub fn run() -> i32 {
    switch::once(NAMESPACE, shown).unwrap_or_else(|| {
        say("the launchpad was up: closed");
        0
    })
}

/// The launchpad's loop.
fn shown(listener: Option<&UnixListener>) -> i32 {
    let hypr = Hypr::from_env();
    let mut grid = Grid::new(entries(), COLUMNS, ROWS);
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the launchpad's tree could not be made");
        return 1;
    };
    view.wear(&crate::theme::Live::new().tokens);
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
        // Ponytail: the pad takes no keyboard (the plan's decision 5): it
        // closes on SUPER L again or on a press off the grid. Upgrade path:
        // arrow keys and a type-ahead, when the plan asks for them.
        keyboard: KeyboardInteractivity::None,
    };
    let options = WindowOptions {
        title: "Launchpad".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (1280, 800),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.pad.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!(
                "err: no compositor to put the launchpad on: {error}"
            ));
            return 1;
        }
    };
    let watch = listener.map(|listener| window.client().watch_fd(listener.as_raw_fd()));
    let mut size = None;
    loop {
        let happened = match window.dispatch(&mut view.ui, None) {
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
            let up = size.is_none();
            size = Some(now);
            view.resize(now);
            if up {
                say(&format!(
                    "launchpad is up, {} apps, {} pages",
                    grid.apps.len(),
                    grid.pages()
                ));
                // Where the tiles are, so a boot test can click one.
                say(&tiles_line(&grid, now.0 as f32, now.1 as f32));
            }
            changed = true;
        }
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Readable(id) if Some(id) == watch => {
                    say("the launchpad was asked again: closed");
                    return 0;
                }
                _ => {}
            }
        }
        let (exec, close, asked) = view.take(&mut grid);
        changed |= asked;
        if changed {
            view.show(&grid);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
        if let Some(exec) = exec {
            let done = hypr
                .as_ref()
                .is_some_and(|hypr| hypr.dispatch(&format!("exec {exec}")));
            say(&format!(
                "launchpad: run {exec}: {}",
                if done { "ok" } else { "refused" }
            ));
            if done {
                return 0;
            }
        }
        if close {
            say("the launchpad closed");
            return 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Grid, View};

    fn apps() -> Vec<crate::search::App> {
        vec![
            crate::search::App {
                exec: "/bin/term /bin/zinc".into(),
                title: "Terminal".into(),
                icon: "terminal-window",
            },
            crate::search::App {
                exec: "/bin/shelf".into(),
                title: "Files".into(),
                icon: "folder",
            },
        ]
    }

    #[test]
    fn the_grid_is_sorted_and_paginated() {
        let grid = Grid::new(apps(), 8, 5);
        assert_eq!(grid.pages(), 1);
        assert_eq!(
            grid.at(0).map(|app| app.title.as_str()),
            Some("Files"),
            "sorted by title"
        );
        assert_eq!(grid.at(1).map(|app| app.title.as_str()), Some("Terminal"));
        // The title is compared, not the whole application: `search::App`
        // is not `PartialEq`.
        assert_eq!(grid.at(2).map(|app| app.title.as_str()), None);
    }

    #[test]
    fn eighty_apps_fill_two_pages_and_a_click_runs_its_app() {
        // The brief's forty apps are one page: eight columns of five rows
        // is forty tiles, so eighty fill two.
        let many: Vec<crate::search::App> = (0..80)
            .map(|n| crate::search::App {
                exec: format!("/bin/app{n}"),
                title: format!("App {n:02}"),
                icon: "",
            })
            .collect();
        let grid = Grid::new(many, 8, 5);
        assert_eq!(grid.pages(), 2);
        assert_eq!(grid.run(37, 0), Some("/bin/app37".to_owned()));
        assert_eq!(grid.run(37, 1), Some("/bin/app77".to_owned()));
        assert_eq!(grid.run(41, 0), None, "past the page");
    }

    #[test]
    fn the_pad_draws() {
        let grid = Grid::new(apps(), 8, 5);
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&grid);
        view.ui.update();
        let mut frame =
            tole::Frame::new((1024, 768), 1.0, tole::Color::rgba(0, 0, 0, 0)).expect("made");
        let _ = view.ui.render_into(&mut frame);
        assert!(!frame.pixmap().data().is_empty());
    }

    #[test]
    fn a_press_on_a_tile_runs_its_app_and_a_press_off_it_goes_away() {
        let mut grid = Grid::new(apps(), 8, 5);
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&grid);
        view.ui.update();
        let (x, y, w, h) = super::place(&grid, 1024.0, 768.0)[0];
        view.ui.pointer_move(x + w / 2.0, y + h / 2.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        let (exec, close, changed) = view.take(&mut grid);
        assert_eq!(exec.as_deref(), Some("/bin/shelf"), "Files sorts first");
        assert!(!close && !changed);
        // A press off the tiles closes the pad.
        view.ui.pointer_move(4.0, 4.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        let (exec, close, _) = view.take(&mut grid);
        assert!(exec.is_none());
        assert!(close, "a press off the grid closes the pad");
    }

    #[test]
    fn a_dot_turns_the_page() {
        let many: Vec<crate::search::App> = (0..80)
            .map(|n| crate::search::App {
                exec: format!("/bin/app{n}"),
                title: format!("App {n:02}"),
                icon: "",
            })
            .collect();
        let mut grid = Grid::new(many, 8, 5);
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&grid);
        view.ui.update();
        // The second of the two dots: the row is centred at the foot.
        view.ui
            .pointer_move(1024.0 / 2.0 + 10.0, 768.0 - 40.0 + 4.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        let (exec, close, changed) = view.take(&mut grid);
        assert!(exec.is_none());
        assert!(!close && changed, "the dot turned the page");
        assert_eq!(grid.page(), 1);
    }

    #[test]
    fn a_press_in_the_gap_between_the_dots_does_nothing() {
        let many: Vec<crate::search::App> = (0..80)
            .map(|n| crate::search::App {
                exec: format!("/bin/app{n}"),
                title: format!("App {n:02}"),
                icon: "",
            })
            .collect();
        let mut grid = Grid::new(many, 8, 5);
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(&grid);
        view.ui.update();
        // Between the two dots: the pad stays up and the page stays.
        view.ui.pointer_move(1024.0 / 2.0, 768.0 - 40.0 + 4.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        let (exec, close, changed) = view.take(&mut grid);
        assert!(exec.is_none());
        assert!(!close, "the gap between the dots does not close the pad");
        assert!(!changed);
        assert_eq!(grid.page(), 0);
    }

    #[test]
    fn the_tiles_are_announced_where_they_are() {
        let grid = Grid::new(apps(), 8, 5);
        let line = super::tiles_line(&grid, 1024.0, 768.0);
        assert!(line.starts_with("launchpad: tiles "), "{line}");
        let tiles: Vec<&str> = line
            .trim_start_matches("launchpad: tiles ")
            .split(' ')
            .collect();
        assert_eq!(tiles.len(), 2, "one rectangle per application");
        let first: Vec<f32> = tiles[0]
            .split(',')
            .map(|n| n.parse().expect("a number"))
            .collect();
        assert_eq!(first.len(), 4, "x,y,w,h");
        let placed = super::place(&grid, 1024.0, 768.0);
        assert_eq!(first[0], placed[0].0.round());
        assert_eq!(first[1], placed[0].1.round());
    }
}
