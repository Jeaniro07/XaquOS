//! The labels of Mission Control's spaces (`docs/DESKTOP.md` §6.10).
//!
//! A space is a workspace, and its label says which one: "Desktop 1" for
//! a numbered workspace, its own name for a named one. The compositor
//! draws the label itself, as it draws the title bars: the shell may not
//! draw over the overview -- the compositor takes the pointer and the
//! keys while it is up -- and the layout of the overview must be in one
//! place, not in two.
//!
//! The label is a surface of the compositor's own, like a title bar, and it
//! is drawn under its space with a window id of its own, which no client's
//! window has.

use std::collections::BTreeMap;

use compositor_layout::{Rect, WindowId};
use compositor_text::{Ellipsize, FontDescription, Fonts, Size, Weight};

use crate::overview::Space;
use crate::titlebar::{Rendered, Style, argb};

/// The bit a space's label carries: no client's window has it, and no
/// title bar's either (`titlebar::BAR`).
const SPACE: u64 = 1 << 61;

/// The room round the label's words, in its own pixels.
const MARGIN: f32 = 8.0;

/// The id of the label of the space at `index`.
#[must_use]
pub fn space_id(index: usize) -> WindowId {
    WindowId(SPACE | u64::try_from(index).unwrap_or(u64::MAX))
}

/// The labels drawn this frame, and where they are.
#[derive(Default)]
pub struct Spaces {
    /// Every label drawn, by its id.
    rendered: BTreeMap<WindowId, Rendered>,
    /// Each label's place on the last frame, in layout space.
    placed: Vec<(WindowId, Rect)>,
    /// The fonts the labels are set in, read the first time one is drawn.
    fonts: Option<Fonts>,
}

impl core::fmt::Debug for Spaces {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Spaces")
            .field("rendered", &self.rendered.len())
            .field("placed", &self.placed)
            .finish_non_exhaustive()
    }
}

impl Spaces {
    /// Draw the label of every space: its name, in the compositor's own
    /// title bar face, on the bar's own ground.
    pub fn draw(&mut self, spaces: &[Space], scale: f64, style: &Style) {
        self.rendered.clear();
        self.placed.clear();
        let fonts = self.fonts.get_or_insert_with(Fonts::system);
        for (index, space) in spaces.iter().enumerate() {
            let id = space_id(index);
            let Some(drawn) = label(fonts, &space.label, space.rect.width, scale, style) else {
                continue;
            };
            let rect = place(&drawn, space.rect, scale);
            let _ = self.rendered.insert(id, drawn);
            self.placed.push((id, rect));
        }
    }

    /// Every label drawn, by its id.
    #[must_use]
    pub fn rendered(&self) -> &BTreeMap<WindowId, Rendered> {
        &self.rendered
    }

    /// Where each label is, in layout space.
    #[must_use]
    pub fn placed(&self) -> &[(WindowId, Rect)] {
        &self.placed
    }
}

/// One label: the space's name, in the title bar's face, on the bar's
/// ground, as wide as the words and as high as the face.
fn label(fonts: &mut Fonts, text: &str, room: i64, scale: f64, style: &Style) -> Option<Rendered> {
    if text.is_empty() {
        return None;
    }
    // Ponytail: this clippy does not warn about narrowing a scale; the
    // allow says so, and does not rot when the lint is gone.
    #[allow(
        clippy::cast_precision_loss,
        reason = "a screen's scale is a small number"
    )]
    let s = scale as f32;
    let mut description = FontDescription::pango(&style.font_family);
    description.weight = Weight(style.font_weight);
    description.size = Size::Pixels(style.font_size * s);
    let font = fonts.resolve(&description);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a space's width is a screen's width"
    )]
    let room = (room.max(1) as f32 * s).max(1.0);
    let run = fonts.shape_ellipsized(&font, text, room, Ellipsize::End);
    let margin = MARGIN * s;
    let width = (run.width + margin * 2.0).max(margin * 4.0).ceil().max(1.0);
    let height = (run.ascent + run.descent + margin * 2.0).ceil().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a label is a screen's width"
    )]
    let (width, height) = (width as u32, height as u32);
    let mut pixmap = compositor_text::tiny_skia::Pixmap::new(width, height)?;
    pixmap.fill(style.active.to_skia());
    let baseline = margin + run.ascent;
    #[expect(clippy::cast_precision_loss, reason = "a label's own width")]
    let x = (width as f32 - run.width) / 2.0;
    fonts.draw_run(&run, &mut pixmap.as_mut(), x, baseline, style.text);
    Some(Rendered {
        bytes: argb(&pixmap),
        width,
        height,
    })
}

/// Where a label is drawn: at the top of its space, centred in it.
fn place(drawn: &Rendered, space: Rect, scale: f64) -> Rect {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a label is a screen's width"
    )]
    let (width, height) = (
        (f64::from(drawn.width) / scale).round() as i64,
        (f64::from(drawn.height) / scale).round() as i64,
    );
    Rect::new(
        space
            .x
            .saturating_add(space.width.saturating_sub(width) / 2),
        space.y,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use compositor_layout::{Rect, WindowId, WorkspaceId};

    use super::{SPACE, Spaces, space_id};
    use crate::overview::Space;
    use crate::titlebar::Style;

    /// A label is drawn with the name of its space, and it is wider than
    /// it is high.
    #[test]
    fn a_label_is_drawn_with_the_workspace_name() {
        let space = Space {
            workspace: WorkspaceId(2),
            label: "Desktop 2".to_owned(),
            rect: Rect::new(0, 0, 300, 400),
            cards: Vec::new(),
        };
        let mut spaces = Spaces::default();
        spaces.draw(&[space], 1.0, &Style::default());
        let drawn = spaces.rendered().values().next().expect("a label");
        assert!(!drawn.bytes.is_empty());
        assert!(drawn.width > drawn.height, "{drawn:?}");
    }

    /// A space's label is no client's window, and no title bar's either.
    #[test]
    fn the_space_ids_are_not_a_clients_windows() {
        let id = space_id(0);
        assert_ne!(id.0 & SPACE, 0, "the space's own bit");
        assert_eq!(id.0 & crate::titlebar::BAR, 0, "no title bar's bit");
        for client in [WindowId(1), WindowId(2), WindowId(3)] {
            assert_ne!(id, client, "no client's window");
        }
        assert_ne!(space_id(0), space_id(1));
    }
}
