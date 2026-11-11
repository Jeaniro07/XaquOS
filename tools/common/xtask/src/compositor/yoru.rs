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
            drawn && framed && docked
        })?;
        if !ready {
            return Err(with_the_transcript(
                &Error::new(format!(
                    "{arch}: yoru: the wallpaper, the second window or the dock never said it was up"
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

/// Whether the launcher is open on `screen`: its card on Nocturne's
/// surface across the middle of the screen's top half, wider than the
/// windows' title bars leave of it.
fn judge_launcher(screen: &Image) -> core::result::Result<String, String> {
    let row = 175;
    let mut card = 0usize;
    for x in 0..screen.width {
        let at = (row * screen.width + x) * 3;
        if let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) {
            let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
            // 8 either way: over the light scrim the card reads #292b37
            // and #2a2c38, the checkered window behind it showing through.
            if (r - 0x23).abs() <= 8 && (g - 0x25).abs() <= 8 && (b - 0x32).abs() <= 8 {
                card += 1;
            }
        }
    }
    if card < 400 {
        return Err(format!(
            "{card} pixels of the launcher's card across row {row}"
        ));
    }
    Ok(format!("the launcher's card {card} pixels across"))
}

/// Whether `screen` is Yoru's light-glass desktop: the wallpaper's sand in
/// the lower right corner, which no window reaches; title bars on the light
/// surface, `#f3f3f5`; a red close button; the wallpaper's blue at the
/// top left, where the gradient starts; and the dock, still Nocturne's
/// until the dock's own change.
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
        return Err(format!("the corner is {r},{g},{b}, not the wallpaper's sand"));
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
    // The dock's bar is translucent: over the light wallpaper its dark
    // ground reads near #3e4552, still far below the wallpaper's #e0 and up.
    let row = screen.height - 6 - 30;
    let [r, g, b] = pixel(screen.width / 2, row);
    if !(r < 0x60 && g < 0x60 && b < 0x70) {
        return Err(format!("the dock's middle is {r},{g},{b}, not its bar"));
    }
    let lit = (0..200).any(|x| {
        (screen.height - 66..screen.height - 6).any(|y| near(pixel(x, y), [0x96, 0x8a, 0xe0], 20))
    });
    if !lit {
        return Err("no workspace lit in the dock".to_owned());
    }
    Ok(format!(
        "the light-glass wallpaper, {surface} pixels of title bar, {close} of close buttons, the dock"
    ))
}

#[cfg(test)]
mod tests {
    use super::{SCREEN, judge};
    use crate::display::Image;

    /// The wallpaper's corner colour, sand.
    const CORNER: [u8; 3] = [0xf6, 0xe3, 0xc9];

    /// A screen of the wallpaper alone is not Yoru's desktop: no bars.
    #[test]
    fn the_wallpaper_alone_has_no_bars() {
        let screen = Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels: CORNER.repeat(SCREEN.0 * SCREEN.1),
        };
        let said = judge(&screen).expect_err("no bars");
        assert!(said.contains("surface"), "{said}");
    }

    /// A light bar with a red close button, the wallpaper's blue near the
    /// top, and the dock are.
    #[test]
    fn a_light_bar_with_its_close_button_is() {
        let mut pixels = CORNER.repeat(SCREEN.0 * SCREEN.1);
        let mut paint = |xs: core::ops::Range<usize>, ys: core::ops::Range<usize>, colour: [u8; 3]| {
            for y in ys {
                for x in xs.clone() {
                    let at = (y * SCREEN.0 + x) * 3;
                    pixels[at..at + 3].copy_from_slice(&colour);
                }
            }
        };
        paint(100..700, 300..330, [0xf3, 0xf3, 0xf5]);
        paint(110..120, 310..320, [0xff, 0x5f, 0x57]);
        paint(100..101, 100..101, [0xc6, 0xcf, 0xf4]);
        paint(SCREEN.0 / 2..SCREEN.0 / 2 + 1, SCREEN.1 - 36..SCREEN.1 - 35, [0x20, 0x22, 0x30]);
        paint(80..110, SCREEN.1 - 50..SCREEN.1 - 48, [0x96, 0x8a, 0xe0]);
        let screen = Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels,
        };
        assert_eq!(judge(&screen).map(|_| ()), Ok(()));
    }
}
