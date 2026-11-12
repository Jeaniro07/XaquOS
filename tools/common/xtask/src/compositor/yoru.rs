//! The Yoru boots (`docs/DESKTOP.md` §6): the desktop in the customer's
//! design, as `run-compositor` now draws it by default, screenshotted.

use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::{Carried, MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The screen the Yoru boots are drawn on.
const SCREEN: (usize, usize) = (1024, 768);

/// The theme's look on two floating windows that asked for the
/// compositor's frame: Nocturne's wallpaper, title bars on the surface with
/// the focused one's accent line, the focused window's glow.
const CONFIG: &str = "\
# Written into the initramfs by `cargo xtask test-compositor`'s yoru boot.
exec-once = /bin/pattern gradient one
exec-once = /bin/pattern checkerboard two --after one
# No grain in the blur, so the picture is the same each run.
decoration:blur:noise = 0
";

/// A boot of Yoru's look (steps Y1 and Y2): the theme sourced as
/// `run-compositor` sources it, the wallpaper drawn, two pattern windows
/// with Yoru's title bars, the dock at the foot, and the picture judged by
/// its colours -- Nocturne's ground, the bars' surface, the accent's line,
/// the dock's bar with the shown workspace lit.
pub(super) fn test_yoru(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    boot(arch, programs, args, false)
}

/// The same desktop with the launcher open over it (step Y3): fuzzel in
/// Yoru's look, as Ctrl K starts it, listing the image's applications --
/// the palette's card on the surface, high on the screen.
pub(super) fn test_yoru_launcher(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    boot(arch, programs, args, true)
}

/// A Yoru boot, with the launcher open or not.
fn boot(arch: Arch, programs: &Programs, args: &Args, launcher: bool) -> Result<()> {
    let (config, carried) = desktop(arch, args, launcher)?;
    let (image, kernel) = build_image(arch, programs, &config, carried, args)?;
    let port = free_port()?;
    let mut qemu_args = args.clone();
    qemu_args.display = true;
    qemu_args.qmp_port = Some(port);
    let dump = paths::build_dir(arch).join(if launcher {
        "yoru-launcher.ppm"
    } else {
        "yoru.ppm"
    });
    let hook = |watching: &mut Watching<'_>| -> Result<()> {
        watching.stop_when_done();
        let mut qmp = Qmp::connect(port, Instant::now() + Duration::from_secs(10))?;
        let ready = watching.read_more(Instant::now() + SETTLE, |lines| {
            let drawn = lines
                .iter()
                .any(|line| line.contains("wallpaper: ") && line.contains(" drawn at "));
            let framed = lines
                .iter()
                .any(|line| line.contains("pattern: two decorations server"));
            let docked = lines
                .iter()
                .any(|line| line.contains("yoru-shell: the dock is up"));
            let barred = lines
                .iter()
                .any(|line| line.contains("yoru-shell: the menu bar is up"));
            drawn && framed && docked && barred
        })?;
        if !ready {
            return Err(with_the_transcript(
                &Error::new(format!(
                    "{arch}: yoru: the wallpaper, the second window, the dock or the menu bar never said it was up"
                )),
                watching,
            ));
        }
        // Judged once the screen has stopped changing: two dumps a second
        // apart, the same and both Yoru's, so the picture kept is not one
        // caught while the second window was being placed.
        let deadline = Instant::now() + SETTLE;
        let mut last: Option<Image> = None;
        loop {
            qmp.screendump(Some(DEVICE_ID), &dump)?;
            let bytes = std::fs::read(&dump)
                .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
            let screen = parse_ppm(&bytes)?;
            let verdict = judge(&screen).and_then(|said| {
                if launcher {
                    judge_launcher(&screen).map(|more| format!("{said}, {more}"))
                } else {
                    Ok(said)
                }
            });
            let still = last.as_ref() == Some(&screen);
            match verdict {
                Ok(said) if still => {
                    println!("  {arch}: yoru: {said}; the picture is {}", dump.display());
                    return Ok(());
                }
                Err(why) if Instant::now() >= deadline => {
                    return Err(with_the_transcript(
                        &Error::new(format!(
                            "{arch}: yoru: {why}; the screen is {}",
                            dump.display()
                        )),
                        watching,
                    ));
                }
                Ok(_) if Instant::now() >= deadline => {
                    return Err(with_the_transcript(
                        &Error::new(format!(
                            "{arch}: yoru: the screen never stopped changing; it is {}",
                            dump.display()
                        )),
                        watching,
                    ));
                }
                _ => {
                    last = Some(screen);
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

/// A program built for `arch`, carried at `/<path>`.
fn program(arch: Arch, app: &str, path: &str) -> Result<crate::ports::File> {
    let built = crate::apps::program(arch, app, app)?;
    Ok(crate::ports::File {
        path: path.to_owned(),
        mode: 0o755,
        content: crate::ports::Content::Bytes(
            std::fs::read(&built)
                .map_err(|error| Error::new(format!("{}: {error}", built.display())))?,
        ),
    })
}

/// A Yoru boot's configuration and what its image carries: the theme as
/// `run-compositor` sources it, the dock, and with `launcher` fuzzel and
/// the applications it lists.
pub(super) fn desktop(arch: Arch, args: &Args, launcher: bool) -> Result<(String, Carried)> {
    let themed = Args {
        theme: Some("yoru".to_owned()),
        config: None,
        ..args.clone()
    };
    let mut carried = Carried::none();
    // The dock, as every desktop carries it (step Y2).
    carried
        .ports
        .push(program(arch, "yoru-shell", super::theme::DOCK)?);
    // The file chooser, which portald opens for GTK's and Chrome's dialogs.
    carried.ports.push(program(arch, "chooser", "bin/chooser")?);
    let mut config = format!(
        "monitor = , {}x{}@60, auto, 1\n{CONFIG}",
        SCREEN.0, SCREEN.1
    );
    if launcher {
        carried.ports.push(program(arch, "fuzzel", "bin/fuzzel")?);
        carried.ports.extend(crate::fuzzel::files(None, false)?);
        config.push_str(
            "exec-once = /bin/fuzzel --config /usr/share/hyprix/themes/yoru-fuzzel.ini \
             --log-level=info\n",
        );
    }
    let config = super::theme::apply(config, &themed, &mut carried.ports)?;
    Ok((config, carried))
}

/// Whether the launcher is open on `screen`: its white card, a run of
/// `#ffffff` 400 to 760 pixels long centred on the screen, somewhere in
/// the rows fuzzel is put at (its top margin below the menu bar).
fn judge_launcher(screen: &Image) -> core::result::Result<String, String> {
    let white = |x: usize, y: usize| {
        let at = (y * screen.width + x) * 3;
        matches!(screen.pixels.get(at..at + 3), Some(&[r, g, b]) if r >= 0xfc && g >= 0xfc && b >= 0xfc)
    };
    let mut best = (0usize, 0usize, 0usize);
    for y in 120..260 {
        let mut x = 0;
        while x < screen.width {
            if !white(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < screen.width && white(x, y) {
                x += 1;
            }
            if x - start > best.0 {
                best = (x - start, start, y);
            }
        }
    }
    let (long, start, y) = best;
    let middle = start + long / 2;
    if !(400..=760).contains(&long) || middle.abs_diff(screen.width / 2) > 16 {
        return Err(format!(
            "the launcher's card: the longest white run is {long} at {start},{y}"
        ));
    }
    Ok(format!(
        "the launcher's card {long} pixels across at row {y}"
    ))
}

/// Whether `screen` is Yoru's light-glass desktop: the wallpaper's sand in
/// the lower right corner, which no window reaches; title bars on the light
/// surface, `#f3f3f5`; a red close button; the wallpaper's blue at the
/// top left, where the gradient starts; the menu bar's glass with words at
/// both ends; and the dock, a centred bar of glass with the terminal's
/// dark tile and a running dot.
#[expect(
    clippy::too_many_lines,
    reason = "one desktop read top to bottom: each part and what it must show"
)]
fn judge(screen: &Image) -> core::result::Result<String, String> {
    if (screen.width, screen.height) != SCREEN {
        return Err(format!("a {}x{} screen", screen.width, screen.height));
    }
    let pixel = |x: usize, y: usize| -> [i32; 3] {
        let at = (y * screen.width + x) * 3;
        match screen.pixels.get(at..at + 3) {
            Some(&[r, g, b]) => [i32::from(r), i32::from(g), i32::from(b)],
            _ => [0, 0, 0],
        }
    };
    let [r, g, b] = pixel(screen.width - 3, screen.height - 3);
    if !(r >= 0xe8 && b < r && b >= 0xb8) {
        return Err(format!(
            "the corner is {r},{g},{b}, not the wallpaper's sand"
        ));
    }
    let near = |[r, g, b]: [i32; 3], [r0, g0, b0]: [i32; 3], by: i32| {
        (r - r0).abs() <= by && (g - g0).abs() <= by && (b - b0).abs() <= by
    };
    let mut surface = 0usize;
    let mut close = 0usize;
    for y in 0..screen.height {
        for x in 0..screen.width {
            let seen = pixel(x, y);
            if near(seen, [0xf3, 0xf3, 0xf5], 2) {
                surface += 1;
            }
            if near(seen, [0xff, 0x5f, 0x57], 24) {
                close += 1;
            }
        }
    }
    // A bar's worth at least 200 pixels wide and 30 high; a close button.
    if surface < 200 * 30 {
        return Err(format!("{surface} pixels of the bars' surface"));
    }
    if close < 40 {
        return Err(format!("{close} pixels of a close button"));
    }
    let [r, g, b] = pixel(100, 100);
    if !(b >= 0xe0 && b > r + 0x10) {
        return Err(format!("no wallpaper's blue at 100,100: {r},{g},{b}"));
    }
    // The menu bar: glass at the top middle, where no menu reaches, and
    // words at both ends (the menus at the left, the clock at the right).
    let glass = |[r, g, b]: [i32; 3], floor: i32, spread: i32| {
        r.min(g).min(b) >= floor && r.max(g).max(b) - r.min(g).min(b) <= spread
    };
    let [r, g, b] = pixel(screen.width / 2, 3);
    if !glass([r, g, b], 0xd8, 0x18) {
        return Err(format!("the menu bar's glass at the top is {r},{g},{b}"));
    }
    let words = |xs: core::ops::Range<usize>| {
        xs.flat_map(|x| (4..20).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b] = pixel(x, y);
                r + g + b < 0x150
            })
            .count()
    };
    let (left, right) = (words(4..160), words(screen.width - 160..screen.width - 4));
    if left < 260 || right < 200 {
        return Err(format!(
            "the menu bar's words: {left} at the left, {right} at the right"
        ));
    }
    // The dock: a bar of glass above its tiles, centred, far narrower than
    // the screen; the terminal's dark tile; a running dot below a tile.
    let row = screen.height - 70;
    let run: Vec<usize> = (0..screen.width)
        .filter(|&x| glass(pixel(x, row), 0xe0, 0x10))
        .collect();
    let (Some(&from), Some(&to)) = (run.first(), run.last()) else {
        return Err(format!("no dock's glass across row {row}"));
    };
    let wide = to - from + 1;
    if !(120..=screen.width - 200).contains(&wide) {
        return Err(format!("the dock's glass is {wide} wide, from {from}"));
    }
    // Centred: the gaps to the screen's sides differ by a pixel when the
    // width's parity differs from the screen's, so 4 is room for that and
    // for the glass's soft edge.
    let middle = (from + to) / 2;
    let (gap_left, gap_right) = (from, screen.width - 1 - to);
    if gap_left.abs_diff(gap_right) > 4 {
        return Err(format!(
            "the dock's gaps to the sides are {gap_left} and {gap_right}, not even"
        ));
    }
    // Its corners are cut round and show the wallpaper: the box round the
    // glass, at its top left, is not glass and matches the wallpaper just
    // outside it, where a scrim or a blur laid over the layer's whole
    // rectangle showed a light square.
    let waist = screen.height - 40;
    let Some(edge) = (0..screen.width).find(|&x| glass(pixel(x, waist), 0xe0, 0x10)) else {
        return Err(format!("no dock's glass across row {waist}"));
    };
    let Some(top) =
        (screen.height - 120..waist).find(|&y| glass(pixel(screen.width / 2, y), 0xe0, 0x10))
    else {
        return Err(format!("no dock's glass above row {waist}"));
    };
    let (corner, outside) = (pixel(edge, top), pixel(edge.saturating_sub(8), top));
    if glass(corner, 0xe0, 0x10) || !near(corner, outside, 6) {
        return Err(format!(
            "the dock's corner at {edge},{top} is {corner:?}, not the wallpaper's {outside:?}"
        ));
    }
    let count = |ys: core::ops::Range<usize>, want: [i32; 3], by: i32| {
        (from..=to)
            .flat_map(|x| ys.clone().map(move |y| (x, y)))
            .filter(|&(x, y)| near(pixel(x, y), want, by))
            .count()
    };
    let tile = count(
        screen.height - 62..screen.height - 18,
        [0x2b, 0x2b, 0x30],
        6,
    );
    if tile < 1460 {
        return Err(format!("{tile} pixels of the terminal's tile in the dock"));
    }
    let dots = count(
        screen.height - 14..screen.height - 8,
        [0x4a, 0x4d, 0x55],
        24,
    );
    if dots < 9 {
        return Err(format!("{dots} pixels of a running dot in the dock"));
    }
    Ok(format!(
        "the light-glass wallpaper, {surface} pixels of title bar, {close} of close buttons, \
         the menu bar ({left}/{right} of words), the dock {wide} wide at {middle} (gaps {gap_left}/{gap_right}, \
         its corner at {edge},{top} the wallpaper), \
         {tile} of the terminal's tile, {dots} of dots"
    ))
}

#[cfg(test)]
mod tests {
    use super::{SCREEN, judge, judge_launcher};
    use crate::display::Image;

    /// The wallpaper's corner colour, sand.
    const CORNER: [u8; 3] = [0xf6, 0xe3, 0xc9];

    fn paint(
        pixels: &mut [u8],
        xs: core::ops::Range<usize>,
        ys: core::ops::Range<usize>,
        colour: [u8; 3],
    ) {
        for y in ys {
            for x in xs.clone() {
                let at = (y * SCREEN.0 + x) * 3;
                pixels[at..at + 3].copy_from_slice(&colour);
            }
        }
    }

    /// Yoru's desktop as A2 draws it: a light bar with its close button,
    /// the wallpaper's blue near the top, the menu bar with words at both
    /// ends, and the dock, `dock_w` wide and centred, the terminal's tile
    /// first with its dot.
    fn desktop(dock_w: usize, menu_bar: bool) -> Vec<u8> {
        let (w, h) = SCREEN;
        let mut pixels = CORNER.repeat(w * h);
        paint(&mut pixels, 100..700, 300..330, [0xf3, 0xf3, 0xf5]);
        paint(&mut pixels, 110..120, 310..320, [0xff, 0x5f, 0x57]);
        paint(&mut pixels, 100..101, 100..101, [0xc6, 0xcf, 0xf4]);
        if menu_bar {
            paint(&mut pixels, 0..w, 0..24, [0xf0, 0xf0, 0xf2]);
            paint(&mut pixels, 10..50, 8..16, [0x1d, 0x1d, 0x1f]);
            paint(&mut pixels, w - 60..w - 20, 8..16, [0x1d, 0x1d, 0x1f]);
        }
        dock(&mut pixels, (w - dock_w) / 2, dock_w);
        pixels
    }

    /// The dock, `wide` across from `left`: its glass, the terminal's tile
    /// first, the running dot under it.
    fn dock(pixels: &mut [u8], left: usize, wide: usize) {
        let h = SCREEN.1;
        paint(pixels, left..left + wide, h - 74..h - 6, [0xf4, 0xf4, 0xf6]);
        // Its corners cut round: the wallpaper shows in each.
        for xs in [left..left + 6, left + wide - 6..left + wide] {
            for ys in [h - 74..h - 68, h - 12..h - 6] {
                paint(pixels, xs.clone(), ys, CORNER);
            }
        }
        paint(
            pixels,
            left + 10..left + 58,
            h - 64..h - 16,
            [0x2b, 0x2b, 0x30],
        );
        paint(
            pixels,
            left + 32..left + 36,
            h - 13..h - 9,
            [0x4a, 0x4d, 0x55],
        );
    }

    fn screen(pixels: Vec<u8>) -> Image {
        Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels,
        }
    }

    /// A screen of the wallpaper alone is not Yoru's desktop: no bars.
    #[test]
    fn the_wallpaper_alone_has_no_bars() {
        let said = judge(&screen(CORNER.repeat(SCREEN.0 * SCREEN.1))).expect_err("no bars");
        assert!(said.contains("surface"), "{said}");
    }

    #[test]
    fn the_a2_desktop_is() {
        assert_eq!(judge(&screen(desktop(301, true))).map(|_| ()), Ok(()));
    }

    #[test]
    fn no_menu_bar_is_not() {
        let said = judge(&screen(desktop(301, false))).expect_err("no menu bar");
        assert!(said.contains("menu bar"), "{said}");
    }

    #[test]
    fn a_dock_across_the_screen_is_not() {
        let said = judge(&screen(desktop(SCREEN.0 - 24, true))).expect_err("too wide");
        assert!(said.contains("dock"), "{said}");
    }

    /// A dock whose glass is a square box -- a scrim or a blur laid over
    /// the layer's whole rectangle, round its rounded glass -- is not.
    #[test]
    fn a_dock_with_square_corners_is_not() {
        let mut pixels = desktop(301, true);
        let (w, h) = SCREEN;
        let left = (w - 301) / 2;
        for xs in [left..left + 6, left + 301 - 6..left + 301] {
            for ys in [h - 74..h - 68, h - 12..h - 6] {
                paint(&mut pixels, xs.clone(), ys, [0xf4, 0xf4, 0xf6]);
            }
        }
        let said = judge(&screen(pixels)).expect_err("square corners");
        assert!(said.contains("corner"), "{said}");
    }

    /// A dock 40 pixels right of the middle: its gaps to the screen's
    /// sides differ by 80.
    #[test]
    fn a_dock_off_the_middle_is_not() {
        let mut pixels = desktop(301, true);
        let (w, h) = SCREEN;
        paint(&mut pixels, 0..w, h - 74..h - 6, CORNER);
        dock(&mut pixels, (w - 301) / 2 + 40, 301);
        let said = judge(&screen(pixels)).expect_err("off the middle");
        assert!(said.contains("gaps"), "{said}");
    }

    #[test]
    fn the_old_dark_dock_is_not() {
        let mut pixels = desktop(301, true);
        let (w, h) = SCREEN;
        paint(&mut pixels, 12..w - 12, h - 66..h - 6, [0x3e, 0x45, 0x52]);
        let said = judge(&screen(pixels)).expect_err("dark dock");
        assert!(said.contains("dock"), "{said}");
    }

    #[test]
    fn the_launcher_is_a_white_card_in_the_middle() {
        let mut pixels = desktop(301, true);
        paint(&mut pixels, 212..812, 174..400, [0xff, 0xff, 0xff]);
        assert_eq!(judge_launcher(&screen(pixels)).map(|_| ()), Ok(()));
        let mut dark = desktop(301, true);
        paint(&mut dark, 212..812, 174..400, [0x23, 0x25, 0x32]);
        assert!(judge_launcher(&screen(dark)).is_err(), "the old dark card");
    }
}
