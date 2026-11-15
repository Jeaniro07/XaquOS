//! `test-compositor --boot spotlight`: SUPER Space opens the Spotlight
//! card over the desktop, a query read as arithmetic and matched against
//! the installed applications, the open windows and the names under `~`,
//! and Enter runs what it finds.
//!
//! A script in the guest starts the shell's Spotlight once hyprix
//! answers. The shell says the card is up and how big it is; the host
//! dumps the screen and counts the card (light, with dark words in it),
//! types "fi" and presses Return, and waits for the shell to say the row
//! ran and for Files' window to be up. Required: the card on the screen,
//! `spotlight: exec /bin/shelf: ok` and Files' window. Kept as
//! `spotlight.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, press, with_the_transcript};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The card's width, in the shell's own pixels (`src/spotlight.rs`).
const WIDTH: usize = 600;
/// How far down the card sits (`ui/spotlight.tole`).
const TOP: usize = 60;
/// The query line's height (`src/spotlight.rs`'s `HEAD`).
const HEAD: usize = 56;
/// The share of its query line the card's glass must be, in percent. The
/// card's own render measured 96% of it; the wallpaper under it, which
/// is tinted and darker, none of it.
const LIGHT: usize = 75;
/// The share of the same line the words must be, in percent. The card's
/// own render measured 2.5% of it; the wallpaper under it none.
const INK: usize = 1;

/// The guest's side: the Spotlight, once hyprix answers, and the windows
/// every two seconds, so the host can see the app that ran.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
hyprctl dispatch exec '/bin/xaqu-shell --spotlight'
while :; do
  echo "spotlight-probe: windows $(hyprctl clients | grep -E '^[[:space:]]*class:' | tr '\n' ';')"
  sleep 2
done
"#;

/// What the card holds on the screen.
#[derive(Debug)]
struct Judged {
    /// The glass: the light pixels of its query line.
    light: usize,
    /// The words in it.
    ink: usize,
}

/// The size the shell said the card was: `the spotlight is up, 1024x768,
/// 12 apps`.
fn said_size(line: &str) -> Option<(u32, u32)> {
    let sizes = line.split("the spotlight is up, ").nth(1)?;
    let size = sizes.split(',').next()?.trim();
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Counts the card where the shell says it is: the light glass of its
/// query line and the dark words in it.
///
/// Ponytail: only the query line is counted, where the words are and
/// where the card is at its plainest. Upgrade path: the rows under it,
/// when a boot needs to know which row is chosen.
fn judge(screen: &Image, said: (u32, u32)) -> Result<Judged> {
    // The card's place, from the size the shell said and the screen's
    // own pixels: the card is centred, 600 wide, 60 down.
    let said_w = usize::try_from(said.0).unwrap_or(1).max(1);
    let said_h = usize::try_from(said.1).unwrap_or(1).max(1);
    let left = said_w.saturating_sub(WIDTH) / 2 * screen.width / said_w;
    let top = TOP * screen.height / said_h;
    let width = WIDTH * screen.width / said_w;
    let height = HEAD * screen.height / said_h;
    let mut judged = Judged { light: 0, ink: 0 };
    for y in top..(top + height).min(screen.height) {
        for x in left..(left + width).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            let sum = u32::from(r) + u32::from(g) + u32::from(b);
            // The glass, which the wallpaper's own tint never reaches.
            if r >= 0xd0 && g >= 0xd0 && b >= 0xd0 {
                judged.light += 1;
            } else if sum < 0x1f4 {
                judged.ink += 1;
            }
        }
    }
    let area = width * height;
    println!(
        "spotlight: the card's query line: light {} of {area} (need {LIGHT}%), ink {} (need {INK}%)",
        judged.light, judged.ink
    );
    if judged.light * 100 >= area * LIGHT && judged.ink * 100 >= area * INK {
        Ok(judged)
    } else {
        Err(Error::new(format!("no spotlight card: {judged:?}")))
    }
}

/// `test-compositor --boot spotlight`.
///
/// # Errors
///
/// The card never came, or the query it was given ran nothing.
pub(super) fn test_spotlight(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/spotlight-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    // shelf installed, its desktop entry with it: what the query finds.
    carried.ports.extend(crate::apps::taken(arch, &["shelf"])?);
    carried.ports.push(crate::ports::File {
        path: "etc/spotlight-probe.sh".to_owned(),
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
            with_the_transcript(&Error::new(format!("{arch}: spotlight: {why}")), watching)
        };
        if !watching.read_more(Instant::now() + SETTLE, |lines| {
            lines.iter().any(|l| l.contains("the spotlight is up"))
        })? {
            return Err(fail("the spotlight never came up", watching));
        }
        let Some(said) = watching.after().iter().rev().find_map(|l| said_size(l)) else {
            return Err(fail(
                "the spotlight never said how big its card is",
                watching,
            ));
        };
        std::thread::sleep(Duration::from_secs(2));
        let dump = paths::build_dir(arch).join("spotlight.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        if let Err(error) = judge(&parse_ppm(&bytes)?, said) {
            return Err(fail(&error.to_string(), watching));
        }
        // The query: "fi", the letters of Files, and Return to run it.
        press(&mut qmp, &["f"])?;
        std::thread::sleep(Duration::from_millis(120));
        press(&mut qmp, &["i"])?;
        std::thread::sleep(Duration::from_millis(500));
        press(&mut qmp, &["ret"])?;
        let ran = watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| l.contains("spotlight: exec /bin/shelf: ok"))
        })?;
        if !ran {
            return Err(fail("the query ran nothing", watching));
        }
        // The app the row named is up.
        let opened = watching.read_more(Instant::now() + Duration::from_secs(60), |lines| {
            lines
                .iter()
                .rev()
                .find(|l| l.contains("spotlight-probe: windows"))
                .is_some_and(|l| l.contains("rocks.magical.shelf"))
        })?;
        if !opened {
            return Err(fail("Files' window did not come up", watching));
        }
        println!(
            "  {arch}: spotlight: the card is up and \"fi\" ran Files (the screen is {})",
            dump.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HEAD, Image, TOP, WIDTH, judge, said_size};

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// The wallpaper's own colour where the card sits: the gradient's
    /// first stop, a little way into the second.
    const WALLPAPER: [u8; 3] = [0xb9, 0xca, 0xf3];

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

    /// The screen with the card on it, at `place`: its left, its top,
    /// its width and its height.
    fn card_at(place: (usize, usize, usize, usize)) -> Image {
        let mut image = wallpaper();
        let (left, top, width, height) = place;
        let screen_width = image.width;
        for row in top..top + height {
            for column in left..left + width {
                let at = (row * screen_width + column) * 3;
                image.pixels[at..at + 3].copy_from_slice(&[0xf0, 0xf0, 0xf2]);
            }
        }
        // The words: the query line's own ink.
        for row in top + 20..(top + 34).min(top + height) {
            for column in left + 20..left + 260 {
                if (column - left) % 3 == 0 {
                    let at = (row * screen_width + column) * 3;
                    image.pixels[at..at + 3].copy_from_slice(&[0x1d, 0x1d, 0x1f]);
                }
            }
        }
        image
    }

    #[test]
    fn the_size_line_says_how_big_the_card_is() {
        let line = "xaqu-shell: the spotlight is up, 1024x768, 12 apps";
        assert_eq!(said_size(line), Some((1024, 768)));
        assert_eq!(said_size("xaqu-shell: the spotlight closed"), None);
    }

    #[test]
    fn a_card_is_light_with_dark_words_in_it() {
        let judged = judge(&card_at((212, TOP, WIDTH, HEAD)), (1024, 768)).expect("a card");
        assert!(judged.light > 0 && judged.ink > 0, "{judged:?}");
    }

    #[test]
    fn the_wallpaper_alone_is_not_a_card() {
        assert!(judge(&wallpaper(), (1024, 768)).is_err());
    }

    #[test]
    fn the_card_is_where_the_size_the_shell_said_puts_it() {
        // The shell said 1280x800: the surface is scaled into the
        // screen's own 1024x768.
        let scaled = card_at((272, 57, 480, 53));
        assert!(judge(&scaled, (1280, 800)).is_ok(), "the scaled card");
        // A card anywhere else is not this card.
        assert!(judge(&card_at((0, 0, WIDTH, HEAD)), (1024, 768)).is_err());
    }
}
