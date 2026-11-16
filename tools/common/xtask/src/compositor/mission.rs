//! `test-compositor --boot mission`: Mission Control, the overview drawn
//! anew -- a space per workspace, the windows of a space as cards -- over
//! the desktop.
//!
//! A script in the guest opens two windows on two workspaces, waits for
//! the compositor to answer, and brings Mission Control up. The
//! compositor says how big the screen is, how many spaces it shows and
//! where the cards are; the host dumps the screen and looks at the cards:
//! inside a card the window's own surface is drawn, and outside it the
//! wallpaper is left to be seen. Escape closes it, and the desktop comes
//! back -- which the same judge must refuse. Required: the cards on the
//! screen, and the desktop after Escape. Kept as `mission.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, press, with_the_transcript};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// How far a pixel's colour may be from the one around a card and still
/// be the same colour: the card's own surface and the wallpaper beside
/// it are far apart, while the window of the desktop and the window
/// around a card are one colour.
const SAME: u8 = 16;

/// The guest's side: two windows on two workspaces, and Mission Control
/// over them.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
/bin/shelf &
sleep 8
hyprctl dispatch workspace 2
sleep 2
/bin/shelf &
sleep 8
hyprctl dispatch workspace 1
sleep 2
hyprctl dispatch mission toggle
sleep 2
echo "mission-probe: clients $(hyprctl clients | grep -c 'class:')"
hyprctl rollinglog | tail -20
"#;

/// One card's place on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Card {
    /// Its left.
    x: i64,
    /// Its top.
    y: i64,
    /// Its width.
    width: i64,
    /// Its height.
    height: i64,
}

/// What the screen shows of the cards.
#[derive(Debug)]
struct Judged {
    /// The cards that are drawn.
    drawn: usize,
    /// The cards in all.
    cards: usize,
}

/// What the compositor said: `the mission control is up, 1024x768, 2
/// spaces`.
fn said_size(line: &str) -> Option<(u32, u32)> {
    let sizes = line.split("the mission control is up, ").nth(1)?;
    let size = sizes.split(',').next()?.trim();
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// How many spaces the compositor said it shows.
fn said_spaces(line: &str) -> Option<usize> {
    let rest = line.split("the mission control is up, ").nth(1)?;
    let spaces = rest.rsplit(',').next()?.trim();
    spaces.strip_suffix(" spaces")?.trim().parse().ok()
}

/// The cards the compositor said it drew: `a card at 24,64 320x240`.
fn said_cards(lines: &[String]) -> Vec<Card> {
    lines
        .iter()
        .filter_map(|line| {
            let rest = line.split("a card at ").nth(1)?;
            let (at, size) = rest.split_once(' ')?;
            let (x, y) = at.split_once(',')?;
            let (width, height) = size.split_once('x')?;
            Some(Card {
                x: x.trim().parse().ok()?,
                y: y.trim().parse().ok()?,
                width: width.trim().parse().ok()?,
                height: height.trim().parse().ok()?,
            })
        })
        .collect()
}

/// The mean colour of the pixels of `areas`, in the screen's own pixels.
fn mean_colour(screen: &Image, areas: &[(i64, i64, i64, i64)]) -> Option<(f64, f64, f64)> {
    let mut sum = (0.0, 0.0, 0.0);
    let mut count = 0_u64;
    for (left, top, width, height) in areas {
        for y in (*top).max(0)..(*top + *height).min(screen.height as i64) {
            for x in (*left).max(0)..(*left + *width).min(screen.width as i64) {
                let at = ((y as usize * screen.width + x as usize) * 3) as usize;
                let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                    continue;
                };
                sum.0 += f64::from(r);
                sum.1 += f64::from(g);
                sum.2 += f64::from(b);
                count += 1;
            }
        }
    }
    if count == 0 {
        return None;
    }
    Some((
        sum.0 / count as f64,
        sum.1 / count as f64,
        sum.2 / count as f64,
    ))
}

/// Whether the card at `card` holds something of its own: its own
/// surface is drawn there, and the colour around it is another one.
///
/// Ponytail: the card is judged by its own colour against the colour
/// around it, so a wallpaper of another picture does not break the boot.
/// Upgrade path: the card's own words, when the guest's windows have
/// text in them.
fn card_is_drawn(screen: &Image, card: &Card) -> bool {
    let inside = mean_colour(
        screen,
        &[(
            card.x + 2,
            card.y + 2,
            card.width.saturating_sub(4),
            card.height.saturating_sub(4),
        )],
    );
    // The ring around the card, the card itself left out of it.
    let around = mean_colour(
        screen,
        &[
            (
                card.x.saturating_sub(4),
                card.y.saturating_sub(4),
                card.width.saturating_add(8),
                4,
            ),
            (
                card.x.saturating_sub(4),
                card.y + card.height,
                card.width.saturating_add(8),
                4,
            ),
            (card.x.saturating_sub(4), card.y, 4, card.height),
            (card.x + card.width, card.y, 4, card.height),
        ],
    );
    let (Some(inside), Some(around)) = (inside, around) else {
        return false;
    };
    let apart = (inside.0 - around.0)
        .abs()
        .max((inside.1 - around.1).abs())
        .max((inside.2 - around.2).abs());
    apart > f64::from(SAME)
}

/// Counts the cards that are really drawn on the screen.
///
/// # Errors
///
/// No card is drawn: the screen is the desktop, not Mission Control.
fn judge(screen: &Image, cards: &[Card]) -> Result<Judged> {
    let drawn = cards
        .iter()
        .filter(|card| card_is_drawn(screen, card))
        .count();
    let judged = Judged {
        drawn,
        cards: cards.len(),
    };
    println!("mission: the cards drawn: {drawn} of {}", judged.cards);
    if drawn == judged.cards && drawn > 0 {
        Ok(judged)
    } else {
        Err(Error::new(format!("no mission control cards: {judged:?}")))
    }
}

/// `test-compositor --boot mission`.
///
/// # Errors
///
/// The cards never came, or Escape did not bring the desktop back.
pub(super) fn test_mission(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/mission-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    carried.ports.extend(crate::apps::taken(arch, &["shelf"])?);
    carried.ports.push(crate::ports::File {
        path: "etc/mission-probe.sh".to_owned(),
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
            with_the_transcript(&Error::new(format!("{arch}: mission: {why}")), watching)
        };
        if !watching.read_more(Instant::now() + Duration::from_secs(180), |lines| {
            lines
                .iter()
                .any(|line| line.contains("the mission control is up"))
        })? {
            return Err(fail("the mission control never came up", watching));
        }
        let said = watching
            .after()
            .iter()
            .rev()
            .find_map(|line| said_size(line));
        let Some(said) = said else {
            return Err(fail(
                "the mission control never said how big the screen is",
                watching,
            ));
        };
        let spaces = watching
            .after()
            .iter()
            .rev()
            .find_map(|line| said_spaces(line))
            .unwrap_or(0);
        let cards = said_cards(&watching.after());
        std::thread::sleep(Duration::from_secs(2));
        let dump = paths::build_dir(arch).join("mission.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        if spaces < 2 {
            return Err(fail(
                "the mission control did not show the two workspaces",
                watching,
            ));
        }
        if cards.is_empty() {
            return Err(fail("the mission control showed no cards", watching));
        }
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        let screen = parse_ppm(&bytes)?;
        // The cards the compositor named are in its own pixels; the dump
        // may be another size.
        let scale = f64::from(screen.width as u32) / f64::from(said.0.max(1));
        let scaled: Vec<Card> = cards
            .iter()
            .map(|card| Card {
                x: (f64::from(card.x as i32) * scale).round() as i64,
                y: (f64::from(card.y as i32) * scale).round() as i64,
                width: (f64::from(card.width as i32) * scale).round() as i64,
                height: (f64::from(card.height as i32) * scale).round() as i64,
            })
            .collect();
        if let Err(error) = judge(&screen, &scaled) {
            return Err(fail(&error.to_string(), watching));
        }
        press(&mut qmp, &["esc"])?;
        let closed = watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|line| line.contains("the mission control closed"))
        })?;
        if !closed {
            return Err(fail("Escape did not close Mission Control", watching));
        }
        std::thread::sleep(Duration::from_secs(2));
        let dump = paths::build_dir(arch).join("mission-desktop.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        let desktop = parse_ppm(&bytes)?;
        if desktop.pixels == screen.pixels {
            return Err(fail("the desktop is the mission control screen", watching));
        }
        println!(
            "  {arch}: mission: the cards are up and Escape brought the desktop back (the screen is {})",
            dump.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Card, Image, judge, said_cards, said_size, said_spaces};

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// The wallpaper's own colour.
    const WALLPAPER: [u8; 3] = [0xa9, 0xc6, 0xf5];

    /// A window's own surface.
    const SURFACE: [u8; 3] = [0xf0, 0xf0, 0xf2];

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

    /// The screen with a rectangle of `colour` at `place`.
    fn with_rectangle(image: &mut Image, place: (usize, usize, usize, usize), colour: [u8; 3]) {
        let (left, top, width, height) = place;
        for row in top..(top + height).min(image.height) {
            for column in left..(left + width).min(image.width) {
                let at = (row * image.width + column) * 3;
                image.pixels[at..at + 3].copy_from_slice(&colour);
            }
        }
    }

    /// The cards of a screen with two small windows at the top.
    fn cards() -> Vec<Card> {
        vec![
            Card {
                x: 100,
                y: 100,
                width: 240,
                height: 180,
            },
            Card {
                x: 380,
                y: 100,
                width: 240,
                height: 180,
            },
        ]
    }

    #[test]
    fn the_size_line_says_how_many_spaces_it_shows() {
        let line = "hyprix: the mission control is up, 1024x768, 2 spaces";
        assert_eq!(said_size(line), Some((1024, 768)));
        assert_eq!(said_spaces(line), Some(2));
        assert_eq!(said_size("hyprix: the mission control is up"), None);
        assert_eq!(said_spaces("hyprix: the mission control is up"), None);
    }

    #[test]
    fn the_card_lines_say_where_the_cards_are() {
        let lines = vec![
            "hyprix: the mission control is up, 1024x768, 2 spaces".to_owned(),
            "hyprix: a card at 24,64 320x240".to_owned(),
            "hyprix: a card at 400,64 320x240".to_owned(),
        ];
        assert_eq!(
            said_cards(&lines),
            [
                Card {
                    x: 24,
                    y: 64,
                    width: 320,
                    height: 240
                },
                Card {
                    x: 400,
                    y: 64,
                    width: 320,
                    height: 240
                },
            ]
        );
    }

    #[test]
    fn the_mission_control_leaves_the_wallpaper_to_be_seen() {
        let mut screen = wallpaper();
        for card in cards() {
            with_rectangle(
                &mut screen,
                (
                    card.x as usize,
                    card.y as usize,
                    card.width as usize,
                    card.height as usize,
                ),
                SURFACE,
            );
        }
        let judged = judge(&screen, &cards()).expect("the cards are drawn");
        assert_eq!(judged.drawn, judged.cards);
    }

    #[test]
    fn the_desktop_is_not_the_mission_control() {
        // The desktop: one big window, where the cards were.
        let mut screen = wallpaper();
        with_rectangle(&mut screen, (60, 60, 640, 320), SURFACE);
        assert!(
            judge(&screen, &cards()).is_err(),
            "the desktop shows no cards of its own"
        );
    }
}
