//! `test-compositor --boot control`: the Control Center card, from
//! `xaqu-shell --control`, drawn at the top right over the desktop, and
//! closed by Escape.
//!
//! A script in the guest starts the card once hyprix answers. The shell
//! says it is up and how big the surface is; the host dumps the screen
//! and counts the card (its own glass and its own words), presses Escape
//! and waits for the shell to say it closed.
//! Required: the card on the screen and `the control center closed`.
//! Kept as `control.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, press, with_the_transcript};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The card's width, in the shell's own pixels (`ui/control.tole`).
const WIDTH: usize = 320;
/// The card's height (`ui/control.tole`).
const HEIGHT: usize = 506;
/// How far the card sits from the right edge.
const RIGHT: usize = 16;
/// How far down the card sits.
const TOP: usize = 34;
/// How much a pixel's channels may differ and still be the card's glass:
/// the glass is the theme's ground over the wallpaper, so it is a near
/// white with a little of the wallpaper's tint.
const NEUTRAL: usize = 12;
/// The share of the card that must be its own glass, in percent. The card's
/// own render measured 84% of it; the wallpaper under it, a tinted
/// gradient, none of it.
const GLASS: usize = 60;
/// The share of the card that must be its own words, in percent. The card's
/// own render measured 2.7% of it.
const INK: usize = 1;

/// The guest's side: the card, once hyprix answers.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
hyprctl dispatch exec '/bin/xaqu-shell --control'
"#;

/// What the card holds on the screen.
#[derive(Debug)]
struct Judged {
    /// The glass: the light pixels of its own colour.
    glass: usize,
    /// The words in it.
    ink: usize,
}

/// The size the shell said the surface was: `the control center is up,
/// 1024x768`.
fn said_size(line: &str) -> Option<(u32, u32)> {
    let sizes = line.split("the control center is up, ").nth(1)?;
    let size = sizes.split(',').next()?.trim();
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Counts the card where the shell says it is: the glass of its own
/// colour and the dark words in it.
///
/// Ponytail: the card is judged by its own glass and its own words, not
/// by the picture's layout, so a wallpaper of another picture does not
/// break the boot. Upgrade path: the sliders' rows, when the guest has
/// an audio sink and they are drawn at full strength.
fn judge(screen: &Image, said: (u32, u32)) -> Result<Judged> {
    let said_w = usize::try_from(said.0).unwrap_or(1).max(1);
    let said_h = usize::try_from(said.1).unwrap_or(1).max(1);
    let left = said_w.saturating_sub(WIDTH + RIGHT) * screen.width / said_w;
    let top = TOP * screen.height / said_h;
    let width = WIDTH * screen.width / said_w;
    let height = HEIGHT * screen.height / said_h;
    let mut judged = Judged { glass: 0, ink: 0 };
    for y in top..(top + height).min(screen.height) {
        for x in left..(left + width).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            let sum = u32::from(r) + u32::from(g) + u32::from(b);
            let lightest = r.max(g).max(b);
            let darkest = r.min(g).min(b);
            if r >= 0xd0 && g >= 0xd0 && b >= 0xd0 && usize::from(lightest - darkest) <= NEUTRAL {
                judged.glass += 1;
            } else if sum < 0x1f4 {
                judged.ink += 1;
            }
        }
    }
    let area = width * height;
    println!(
        "control: the card: glass {} of {area} (need {GLASS}%), ink {} (need {INK}%)",
        judged.glass, judged.ink
    );
    if judged.glass * 100 >= area * GLASS && judged.ink * 100 >= area * INK {
        Ok(judged)
    } else {
        Err(Error::new(format!("no control center card: {judged:?}")))
    }
}

/// `test-compositor --boot control`.
///
/// # Errors
///
/// The card never came, or Escape did not close it.
pub(super) fn test_control(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/control-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    carried.ports.push(crate::ports::File {
        path: "etc/control-probe.sh".to_owned(),
        mode: 0o755,
        content: crate::ports::Content::Bytes(SCRIPT.as_bytes().to_vec()),
    });
    let (image, kernel) = build_image(arch, programs, &config, carried, args)?;
    let port = free_port()?;
    let mut qemu_args = args.clone();
    qemu_args.display = true;
    qemu_args.qmp_port = Some(port);
    let hook = |watching: &mut Watching<'_>| -> Result<()> {
        watching.stop_when_done();
        let mut qmp = Qmp::connect(port, Instant::now() + Duration::from_secs(10))?;
        let fail = |why: &str, watching: &Watching<'_>| {
            with_the_transcript(&Error::new(format!("{arch}: control: {why}")), watching)
        };
        if !watching.read_more(Instant::now() + SETTLE, |lines| {
            lines.iter().any(|l| l.contains("the control center is up"))
        })? {
            return Err(fail("the control center never came up", watching));
        }
        let Some(said) = watching.after().iter().rev().find_map(|l| said_size(l)) else {
            return Err(fail(
                "the control center never said how big it is",
                watching,
            ));
        };
        std::thread::sleep(Duration::from_secs(2));
        let dump = paths::build_dir(arch).join("control.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        if let Err(error) = judge(&parse_ppm(&bytes)?, said) {
            return Err(fail(&error.to_string(), watching));
        }
        press(&mut qmp, &["esc"])?;
        let closed = watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| l.contains("the control center closed"))
        })?;
        if !closed {
            return Err(fail("Escape did not close the card", watching));
        }
        println!(
            "  {arch}: control: the card is up and Escape closed it (the screen is {})",
            dump.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, Image, RIGHT, TOP, WIDTH, judge, said_size};

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// The wallpaper's own colour where the card sits.
    const WALLPAPER: [u8; 3] = [0xb9, 0xca, 0xf3];

    /// The card's own glass.
    const GLASS: [u8; 3] = [0xf0, 0xf0, 0xf2];

    /// The card's own words.
    const INK: [u8; 3] = [0x1d, 0x1d, 0x1f];

    /// The screen, the wallpaper alone.
    fn wallpaper() -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0_u8; width * height * 3];
        for chunk in pixels.as_chunks_mut::<3>().0 {
            chunk.copy_from_slice(&WALLPAPER);
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    /// The screen with the card on it, at `place`: its left and its top.
    fn card_at(place: (usize, usize)) -> Image {
        let mut image = wallpaper();
        let (left, top) = place;
        let screen_width = image.width;
        for row in top..top + HEIGHT {
            for column in left..left + WIDTH {
                let at = (row * screen_width + column) * 3;
                image.pixels[at..at + 3].copy_from_slice(&GLASS);
            }
        }
        // The words: the tiles' labels, a third of every third column.
        for row in top + 164..top + 214 {
            for column in left + 20..left + 300 {
                if (column - left) % 3 == 0 {
                    let at = (row * screen_width + column) * 3;
                    image.pixels[at..at + 3].copy_from_slice(&INK);
                }
            }
        }
        image
    }

    #[test]
    fn the_size_line_says_how_big_the_surface_is() {
        assert_eq!(
            said_size("xaqu-shell: the control center is up, 1024x768"),
            Some((1024, 768))
        );
        assert_eq!(said_size("xaqu-shell: the control center closed"), None);
    }

    #[test]
    fn a_card_is_its_own_glass_with_dark_words_in_it() {
        let card = card_at((1024 - WIDTH - RIGHT, TOP));
        let judged = judge(&card, (1024, 768)).expect("a card");
        assert!(judged.glass > 0 && judged.ink > 0, "{judged:?}");
    }

    #[test]
    fn the_wallpaper_alone_is_not_a_card() {
        assert!(judge(&wallpaper(), (1024, 768)).is_err());
    }

    #[test]
    fn a_card_at_the_left_is_not_this_card() {
        assert!(judge(&card_at((0, TOP)), (1024, 768)).is_err());
    }

    #[test]
    fn a_card_without_words_is_not_a_card() {
        let mut card = card_at((1024 - WIDTH - RIGHT, TOP));
        let width = card.width;
        for row in TOP + 164..TOP + 214 {
            for column in (1024 - WIDTH - RIGHT + 20)..(1024 - WIDTH - RIGHT + 300) {
                let at = (row * width + column) * 3;
                card.pixels[at..at + 3].copy_from_slice(&GLASS);
            }
        }
        assert!(judge(&card, (1024, 768)).is_err(), "glass without words");
    }
}
