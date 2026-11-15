//! `test-compositor --boot notices`: the desktop's notifications, as the
//! shell shows them -- a glass banner at the top right, and the history
//! behind the clock.
//!
//! notifyd reads the driver events `devmgr` publishes, so a driver that
//! dies is a notification; the shell draws it as a banner and keeps it in
//! the history. The host waits for the shell's banner, finds the clock
//! in the menu bar's strip on the screen, presses it, and waits for the
//! panel to open. Required: the banner, the panel open with the notice
//! in it, and the panel on the screen. Kept as `notices.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::files::{at, press_here};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The panel's place on the screen: the surface's right edge is 8 from
/// the screen's, the panel 8 further in, and it is under the banners.
const PANEL: (usize, usize, usize, usize) = (664, 248, 344, 322);
/// Light pixels the panel needs. Only ever raised; the first boot
/// measured 104204.
const LIGHT: usize = 60_000;

/// The guest's side: the wait for the compositor, and then a driver that
/// dies -- which is a notification, for notifyd reads devmgr's events.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
sleep 5
for p in $(ls /proc | grep '^[0-9][0-9]*$'); do
  name=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null)
  case "$name" in
    *blk*) kill "$p" 2>/dev/null; echo "notices-probe: killed the block driver $p";;
  esac
done
while :; do sleep 2; done
"#;

/// The clock's place on the screen: the rightmost dark word in the menu
/// bar's strip, which is the clock at the bar's right end.
fn clock(screen: &Image) -> Option<usize> {
    let mut right = None;
    for x in 0..screen.width {
        let dark = (4..20).any(|y| {
            let at = (y * screen.width + x) * 3;
            screen
                .pixels
                .get(at..at + 3)
                .is_some_and(|pixel| pixel.iter().map(|&byte| u32::from(byte)).sum::<u32>() < 0x150)
        });
        if dark {
            right = Some(x);
        }
    }
    right.map(|x| x.saturating_sub(4))
}

/// Counts the panel on the screen: the light card the history is in.
fn judge(screen: &Image) -> Result<usize> {
    let (x, y, w, h) = PANEL;
    let mut light = 0;
    for y in y..(y + h).min(screen.height) {
        for x in x..(x + w).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            if r >= 0xf0 && g >= 0xf0 && b >= 0xf0 {
                light += 1;
            }
        }
    }
    println!("notices: panel light {light} (need {LIGHT})");
    if light >= LIGHT {
        Ok(light)
    } else {
        Err(Error::new(format!(
            "no notices panel: {light} light pixels"
        )))
    }
}

/// `test-compositor --boot notices`.
///
/// # Errors
///
/// No notification came, the clock opened nothing, or the panel was not
/// on the screen.
pub(super) fn test_notices(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::yoru::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/notices-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    carried.ports.push(crate::ports::File {
        path: "etc/notices-probe.sh".to_owned(),
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
            with_the_transcript(&Error::new(format!("{arch}: notices: {why}")), watching)
        };
        // The shell is subscribed to notifyd, and a notification came.
        if !watching.read_more(Instant::now() + SETTLE, |lines| {
            lines
                .iter()
                .any(|l| l.contains("notices: notifyd is there"))
        })? {
            return Err(fail("the shell found no notifyd", watching));
        }
        if !watching.read_more(Instant::now() + Duration::from_secs(60), |lines| {
            lines.iter().any(|l| l.contains("notices: banner "))
        })? {
            return Err(fail("no notification came", watching));
        }
        // The clock, on the menu bar at the right end.
        let before = paths::build_dir(arch).join("notices.ppm");
        qmp.screendump(Some(DEVICE_ID), &before)?;
        let bytes = std::fs::read(&before)
            .map_err(|error| Error::new(format!("reading {}: {error}", before.display())))?;
        let screen = parse_ppm(&bytes)?;
        let Some(x) = clock(&screen) else {
            return Err(fail("the menu bar has no clock", watching));
        };
        println!("  {arch}: notices: the clock is at {x}");
        at(&mut qmp, (i64::try_from(x).unwrap_or(0), 12))?;
        std::thread::sleep(Duration::from_millis(500));
        press_here(&mut qmp)?;
        if !watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| l.contains("notices: the panel is open"))
                && lines.iter().any(|l| l.contains("notices: up, "))
        })? {
            return Err(fail("the clock opened no panel", watching));
        }
        std::thread::sleep(Duration::from_secs(2));
        qmp.screendump(Some(DEVICE_ID), &before)?;
        let bytes = std::fs::read(&before)
            .map_err(|error| Error::new(format!("reading {}: {error}", before.display())))?;
        if let Err(error) = judge(&parse_ppm(&bytes)?) {
            return Err(fail(&error.to_string(), watching));
        }
        println!(
            "  {arch}: notices: the banner came and the clock opened the panel (the screen is {})",
            before.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Image, clock, judge};

    /// The screen, as the Yoru boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// A screen with the menu bar's clock at the right end, on the
    /// wallpaper's light.
    fn screen(clock_at: Option<usize>) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0xf0_u8; width * height * 3];
        if let Some(at) = clock_at {
            for y in 8..16 {
                for x in at..at + 40 {
                    let pixel = (y * width + x) * 3;
                    pixels[pixel..pixel + 3].copy_from_slice(&[0x1d, 0x1d, 0x1f]);
                }
            }
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    /// A screen with the notices' panel in it, or without it.
    fn panel(drawn: bool) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0x60_u8; width * height * 3];
        if drawn {
            let (x, y, w, h) = super::PANEL;
            for row in y..y + h {
                for column in x..x + w {
                    let at = (row * width + column) * 3;
                    pixels[at..at + 3].copy_from_slice(&[0xfa, 0xfa, 0xfb]);
                }
            }
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn the_clock_is_the_rightmost_word_in_the_bar() {
        assert_eq!(clock(&screen(Some(960))), Some(995));
        assert_eq!(clock(&screen(None)), None);
    }

    #[test]
    fn a_panel_is_light_where_it_is_drawn() {
        assert!(judge(&panel(true)).is_ok());
        assert!(judge(&panel(false)).is_err());
    }
}
