//! `test-compositor --boot notices`: the desktop's notifications, as the
//! shell shows them -- a glass banner at the top right, and the history
//! behind the clock.
//!
//! lode turns a record in the kernel log into an alert, and the alert is
//! a notification, for notifyd reads it; the shell draws it as a banner
//! and keeps it in the history. The guest carries the rule that does it
//! and writes the record it matches; the host waits for the shell to have
//! found notifyd, finds the clock in the menu bar's strip on the screen,
//! presses it, and waits for the panel to open with the notice in it.
//! Required: the panel open with a notice shown, and the panel on the
//! screen. Kept as `notices.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::files::{at, press_here};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The screen, as the Xaqu boot sets it.
const SCREEN: (usize, usize) = (1024, 768);

/// The panel's place on the screen: the notices surface is 360 wide and 8
/// from the screen's right edge, 32 from its top; the panel is 8 further
/// in and under the three banner places (3 x 72).
const PANEL_X: usize = 664;
const PANEL_Y: usize = 248;
const PANEL_W: usize = 344;
const PANEL_H: usize = 322;

/// Light pixels the panel's card needs: half of it. The card is white on
/// the wallpaper's sand, which is not white at all, so half the card is
/// the card and nothing else.
const LIGHT: usize = PANEL_W * PANEL_H / 2;

/// The guest's side: the wait for the compositor, and then the mark in
/// the kernel log that the rule turns into an alert.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
echo "notices-probe: lode.d:"; ls -l /etc/ferrix/lode.d 2>&1
echo "notices-probe: rules:"; cat /etc/ferrix/lode.d/notices.rules 2>&1
echo "notices-probe: reload:"; /sbin/lode reload 2>&1
( /sbin/lode alerts > /tmp/notices-alerts.txt 2>&1 & )
sleep 20
echo "notices-probe: the notices boot" > /dev/kmsg
echo "notices-probe: the mark was written"
sleep 10
echo "notices-probe: alerts-file:"; cat /tmp/notices-alerts.txt 2>&1
echo "notices-probe: levels:"; /sbin/lode levels 2>&1 | grep -i notices
sleep 30
echo "notices-probe: the notices boot" > /dev/kmsg
sleep 10
echo "notices-probe: alerts-file-2:"; cat /tmp/notices-alerts.txt 2>&1
while :; do sleep 2; done
"#;

/// The rule lode reads in the guest, in `/etc/ferrix/lode.d`: a `notify`
/// alert is a line on `/run/lode/alerts`, which notifyd reads and shows
/// as a banner. The record it matches is the mark the script writes.
const RULES: &str =
    "alert notices-boot where message = \"*notices-probe*\" notify \"the notices boot\"\n";

/// The clock's place on the screen: the rightmost dark word in the menu
/// bar's strip at the right end, which is the clock. The bar's words at
/// the left are the menus, and nothing else is in the bar's rows.
fn clock(screen: &Image) -> Option<usize> {
    let right = screen.width.saturating_sub(160);
    (right..screen.width).rev().find(|&x| {
        (4..20).any(|y| {
            let at = (y * screen.width + x) * 3;
            screen
                .pixels
                .get(at..at + 3)
                .is_some_and(|pixel| pixel.iter().map(|&byte| u32::from(byte)).sum::<u32>() < 0x150)
        })
    })
}

/// Counts the panel's card on the screen: the white card the history is in.
fn judge(screen: &Image) -> Result<usize> {
    if (screen.width, screen.height) != SCREEN {
        return Err(Error::new(format!(
            "a {}x{} screen",
            screen.width, screen.height
        )));
    }
    let mut light = 0;
    for y in PANEL_Y..(PANEL_Y + PANEL_H).min(screen.height) {
        for x in PANEL_X..(PANEL_X + PANEL_W).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            if r >= 0xf8 && g >= 0xf8 && b >= 0xf8 {
                light += 1;
            }
        }
    }
    println!("notices: the panel's light {light} (need {LIGHT})");
    if light >= LIGHT {
        Ok(light)
    } else {
        Err(Error::new(format!(
            "no notices panel: {light} light pixels"
        )))
    }
}

/// How many notices the shell's line says are shown.
fn shown(line: &str) -> Option<usize> {
    let (_, rest) = line.split_once("notices: up, ")?;
    let (count, _) = rest.split_once(' ')?;
    count.parse().ok()
}

/// `test-compositor --boot notices`.
///
/// # Errors
///
/// No notification came, the clock opened nothing, or the panel was not
/// on the screen.
pub(super) fn test_notices(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
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
    // The rule that turns the mark into an alert, read by lode when it
    // starts.
    carried.ports.push(crate::ports::File {
        path: "etc/ferrix/lode.d/notices.rules".to_owned(),
        mode: 0o644,
        content: crate::ports::Content::Bytes(RULES.as_bytes().to_vec()),
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
        // The shell's notices, subscribed to notifyd.
        if !watching.read_more(Instant::now() + SETTLE, |lines| {
            lines
                .iter()
                .any(|l| l.contains("notices: notifyd is there"))
        })? {
            return Err(fail("the shell found no notifyd", watching));
        }
        // The mark in the kernel log, which lode's rule turns into an
        // alert.
        if !watching.read_more(Instant::now() + Duration::from_secs(60), |lines| {
            lines
                .iter()
                .any(|l| l.contains("notices-probe: the mark was written"))
        })? {
            return Err(fail("the mark was not written", watching));
        }
        if !watching.read_more(Instant::now() + Duration::from_secs(60), |lines| {
            lines.iter().any(|l| l.contains("notices: banner "))
        })? {
            return Err(fail("no notification came", watching));
        }
        // The clock, on the menu bar at the right end.
        let dump = paths::build_dir(arch).join("notices.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        let Some(x) = clock(&parse_ppm(&bytes)?) else {
            return Err(fail("the menu bar has no clock", watching));
        };
        println!("  {arch}: notices: the clock is at {x}");
        at(&mut qmp, (i64::try_from(x).unwrap_or(0), 12))?;
        std::thread::sleep(Duration::from_millis(500));
        press_here(&mut qmp)?;
        // The panel, open with the notice in it.
        if !watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| l.contains("notices: the panel is open"))
        })? {
            return Err(fail("the clock opened no panel", watching));
        }
        if !watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| shown(l).is_some_and(|count| count >= 1))
        })? {
            return Err(fail("the panel holds no notice", watching));
        }
        std::thread::sleep(Duration::from_secs(2));
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        if let Err(error) = judge(&parse_ppm(&bytes)?) {
            return Err(fail(&error.to_string(), watching));
        }
        println!(
            "  {arch}: notices: the clock opened the panel (the screen is {})",
            dump.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Image, PANEL_H, PANEL_W, PANEL_X, PANEL_Y, RULES, SCRIPT, clock, judge, shown};

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// The mark the script writes into the kernel log.
    const MARK: &str = "notices-probe";

    /// A screen with the menu bar's glass and, at the right end, the
    /// clock's words.
    fn screen(clock_at: Option<usize>) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0xf3_u8; width * height * 3];
        // The menus' words at the left.
        for x in 20..170 {
            for y in 8..16 {
                let at = (y * width + x) * 3;
                pixels[at..at + 3].copy_from_slice(&[0x1d, 0x1d, 0x1f]);
            }
        }
        if let Some(at) = clock_at {
            for x in at..at + 40 {
                for y in 8..16 {
                    let at = (y * width + x) * 3;
                    pixels[at..at + 3].copy_from_slice(&[0x1d, 0x1d, 0x1f]);
                }
            }
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    /// A screen with the notices' card in it, or without it.
    fn panel(drawn: bool) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0x60_u8; width * height * 3];
        if drawn {
            for row in PANEL_Y..PANEL_Y + PANEL_H {
                for column in PANEL_X..PANEL_X + PANEL_W {
                    let at = (row * width + column) * 3;
                    pixels[at..at + 3].copy_from_slice(&[0xfd, 0xfd, 0xfd]);
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
        assert_eq!(clock(&screen(Some(960))), Some(999));
        assert_eq!(clock(&screen(None)), None);
    }

    #[test]
    fn a_panel_is_light_where_it_is_drawn() {
        assert!(judge(&panel(true)).is_ok());
        assert!(judge(&panel(false)).is_err());
    }

    /// The rule must let through the mark the script writes.
    #[test]
    fn the_rule_matches_the_mark_the_script_writes() {
        assert!(SCRIPT.contains(MARK));
        assert!(RULES.contains(&format!("*{MARK}*")));
    }

    #[test]
    fn the_history_count_is_read_from_the_shells_line() {
        assert_eq!(shown("xaqu-shell: notices: up, 1 shown"), Some(1));
        assert_eq!(shown("xaqu-shell: notices: up, 12 shown"), Some(12));
        assert_eq!(shown("xaqu-shell: notices: the panel is open"), None);
        assert_eq!(shown("xaqu-shell: notices: up, none shown"), None);
    }
}
