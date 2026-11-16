//! The Control Center (the menu bar's control icon): the quick settings in
//! one glass card at the top right, over a clear full-screen layer that a
//! press or Escape dismisses. The rows are the sidebar's quick tab's
//! (`ui::quick_rows`) and the presses are the same changes
//! (`quick::slide_change` and the others), so the two stay one list.
//!
//! Started again while it is up, it closes: the wallpaper switcher's
//! [`switch::once`], on `$XDG_RUNTIME_DIR/xaqu-control.sock`
//! (`/tmp/xaqu-control-UID.sock` with no runtime folder).

use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixListener;
use std::time::Duration;

use compositor_toolkit::Event;
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::hypr::Hypr;
use crate::quick::{self, Change, Quick};
use crate::switch;
use crate::ui::{Control, ControlSignal, quick_rows};
use crate::window::say;

/// The surface's namespace.
pub const NAMESPACE: &str = "xaqu-control";

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> std::rc::Rc<str> {
    std::rc::Rc::from(value.as_ref())
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The card.
    pub ctl: Control,
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
        let ctl = Control::new(&mut ui, None)?;
        Some(View { ui, ctl })
    }

    /// Draw in `tokens`.
    pub fn wear(&mut self, tokens: &crate::theme::Tokens) {
        tokens.apply(&mut self.ui);
    }

    /// Its size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.ctl.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree the rows of `quick`.
    pub fn show(&mut self, quick: &Quick) {
        let rows = quick_rows(quick);
        let ui = &mut self.ui;
        let ctl = &self.ctl;
        ctl.sliders.set(ui, rows.sliders);
        ctl.tiles.set(ui, rows.tiles);
        ctl.looks.set(ui, rows.looks);
        ctl.accents.set(ui, rows.accents);
        ctl.line.set(ui, s(&quick.said));
    }

    /// What the tree asked since the last call: the changes to make, and
    /// whether the card is to close.
    pub fn take(&mut self) -> (Vec<Change>, bool) {
        let mut changes = Vec::new();
        let mut close = false;
        for each in &self.ui.take_signals() {
            let Some(signal) = self.ctl.signal(each) else {
                continue;
            };
            let index = |i: i32| usize::try_from(i).ok();
            match signal {
                ControlSignal::Slide(i, value) => {
                    changes.extend(index(i).and_then(|n| quick::slide_change(n, value)));
                }
                ControlSignal::Mute(i) => changes.extend(index(i).and_then(quick::mute_change)),
                ControlSignal::Tile(i) => changes.extend(index(i).and_then(quick::tile_change)),
                ControlSignal::Accent(i) => changes.extend(index(i).map(Change::Theme)),
                ControlSignal::Look(i) => changes.extend(index(i).map(Change::Appearance)),
                ControlSignal::Dismissed => close = true,
            }
        }
        (changes, close)
    }
}

/// Run the card until it is closed; the exit status.
#[must_use]
pub fn run() -> i32 {
    switch::once(NAMESPACE, shown).unwrap_or_else(|| {
        say("the control center was up: closed");
        0
    })
}

/// The card's loop.
fn shown(listener: Option<&UnixListener>) -> i32 {
    let hypr = Hypr::from_env();
    let mut quick = quick::read(hypr.as_ref());
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the control center's tree could not be made");
        return 1;
    };
    let mut live = crate::theme::Live::new();
    view.wear(&live.tokens);
    view.show(&quick);
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
        title: "Control Center".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (1280, 800),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.ctl.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!(
                "err: no compositor to put the control center on: {error}"
            ));
            return 1;
        }
    };
    let watch = listener.map(|listener| window.client().watch_fd(listener.as_raw_fd()));
    let mut size = None;
    loop {
        // Ponytail: the card looks at the theme files twice a second, so a
        // new colour or look shows at once. Upgrade path: the menubar's
        // `Live` is already told by the same files; share one watcher.
        let wait = Some(Duration::from_millis(500));
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
                say(&format!("the control center is up, {}x{}", now.0, now.1));
            }
            size = Some(now);
            view.resize(now);
            changed = true;
        }
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Readable(id) if Some(id) == watch => {
                    say("the control center was asked again: closed");
                    return 0;
                }
                _ => {}
            }
        }
        if let Some(tokens) = live.poll() {
            view.wear(&tokens);
            changed = true;
        }
        let (changes, close) = view.take();
        for change in &changes {
            let said = quick::change(hypr.as_ref(), &quick, change);
            quick = quick::read(hypr.as_ref());
            say(&format!("control: {said}"));
            quick.said = said;
            changed = true;
        }
        if changed {
            view.show(&quick);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
        if close {
            say("the control center closed");
            return 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::View;
    use crate::quick::{Change, Quick};

    /// A view of `quick` on a 1024 x 768 screen, updated.
    fn view(quick: &Quick) -> View {
        let mut view = View::new(crate::look::fonts()).expect("made");
        view.ui.set_window_active(true);
        view.resize((1024, 768));
        view.show(quick);
        view.ui.update();
        view
    }

    /// A click at `(x, y)`; what it asked.
    fn click(view: &mut View, x: f32, y: f32) -> (Vec<Change>, bool) {
        view.ui.update();
        view.ui.pointer_move(x, y);
        view.ui.pointer_press();
        view.ui.pointer_release();
        view.take()
    }

    /// The card sits at the top right: x 688, y 34; its rows 16 inside.
    #[test]
    fn presses_in_the_card_ask_the_change_they_name() {
        let mut view = view(&Quick::default());
        let (left, right, top) = (
            688.0 + 16.0 + 70.0,
            688.0 + 16.0 + 147.0 + 70.0,
            34.0 + 16.0,
        );
        assert_eq!(
            click(&mut view, left, top + 148.0 + 56.0 + 25.0),
            (vec![Change::Quiet], false)
        );
        assert_eq!(
            click(&mut view, right, top + 148.0 + 112.0 + 25.0),
            (vec![Change::Tile], false)
        );
        assert_eq!(
            click(&mut view, left, top + 148.0 + 25.0),
            (Vec::new(), false),
            "Wi-Fi is drawn disabled"
        );
        assert_eq!(
            click(
                &mut view,
                688.0 + 16.0 + 98.0 * 2.0 + 46.0,
                top + 342.0 + 14.0
            ),
            (vec![Change::Appearance(2)], false)
        );
        assert_eq!(
            click(
                &mut view,
                688.0 + 16.0 + 49.0 * 2.0 + 20.0,
                top + 402.0 + 14.0
            ),
            (vec![Change::Theme(2)], false)
        );
    }

    /// A press off the card, and Escape, ask to close; a press on the
    /// card's bare ground does not.
    #[test]
    fn a_press_off_the_card_and_escape_close_it() {
        let mut view = view(&Quick::default());
        assert_eq!(click(&mut view, 100.0, 400.0), (Vec::new(), true));
        let mut view = self::view(&Quick::default());
        assert_eq!(click(&mut view, 700.0, 530.0), (Vec::new(), false));
        assert!(view.ui.key_press(&tole::KeyEvent::named("Escape")));
        assert_eq!(view.take(), (Vec::new(), true));
    }

    /// The slider's track sets its row's percentage when there is a sink.
    #[test]
    fn a_slider_track_sets_the_volume() {
        let quick = Quick {
            sink: Some(crate::sound::Device {
                index: 0,
                percent: 100,
                muted: false,
                channels: 2,
            }),
            ..Quick::default()
        };
        let mut view = view(&quick);
        let top = 34.0 + 16.0;
        assert_eq!(
            click(&mut view, 688.0 + 16.0 + 144.0, top + 46.0 + 26.0),
            (vec![Change::Volume(50)], false)
        );
        assert_eq!(
            click(&mut view, 688.0 + 16.0 + 40.0, top + 46.0 + 6.0),
            (vec![Change::Mute], false)
        );
    }

    #[test]
    fn the_card_draws() {
        let mut view = view(&Quick::default());
        let mut frame =
            tole::Frame::new((1024, 768), 1.0, tole::Color::rgba(0, 0, 0, 0)).expect("made");
        let _ = view.ui.render_into(&mut frame);
        assert!(!frame.pixmap().data().is_empty());
    }
}
