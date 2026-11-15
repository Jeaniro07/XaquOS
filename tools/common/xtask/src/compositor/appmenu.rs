//! `test-compositor --boot appmenu`: Files sets its own menus over
//! `ferrix_appmenu_v1`, the bar shows them, a press drops File and picking
//! New Tab reaches Files.
//!
//! A script in the guest starts shelf once hyprix answers. The menu bar
//! says its titles and where each sits; the host presses File's, dumps the
//! screen and counts the drop below it (white with dark words), then
//! presses the first row. Required: the bar's `dispatch appmenu <address> 1`
//! answered ok and shelf saying it was picked. Kept as `appmenu.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::files::{at, press_here};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// White pixels a dropped menu needs below its title. Only ever raised.
/// First boot measured 21211.
const WHITE: usize = 18_000;
/// Dark word pixels it needs. Only ever raised; first boot measured 7870,
/// the unit's screen paints far fewer, so the gate stays where the plan
/// set it and the white one carries the regression.
const DARK: usize = 200;

/// The guest's side: shelf, once hyprix answers, focused once its window
/// is up (a window that maps does not take the focus from the patterns).
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
/bin/shelf &
until hyprctl clients | grep -q rocks.magical.shelf; do sleep 0.2; done
hyprctl dispatch focuswindow 'class:^(rocks.magical.shelf)$'
exec sleep 2
"#;

/// What the drop below a title holds.
#[derive(Debug)]
struct Judged {
    /// Its white pixels.
    white: usize,
    /// Its dark ones: the words.
    dark: usize,
}

/// Counts the drop at `x`, 260 wide and 120 high below the bar.
fn judge(screen: &Image, x: usize) -> Result<Judged> {
    let mut judged = Judged { white: 0, dark: 0 };
    for y in 26..(26 + 120).min(screen.height) {
        for x in x..(x + 260).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            let sum = u32::from(r) + u32::from(g) + u32::from(b);
            if r >= 0xf0 && g >= 0xf0 && b >= 0xf0 {
                judged.white += 1;
            } else if sum < 0x150 {
                judged.dark += 1;
            }
        }
    }
    println!(
        "appmenu: drop white {} (need {WHITE}), dark {} (need {DARK})",
        judged.white, judged.dark
    );
    if judged.white >= WHITE && judged.dark >= DARK {
        Ok(judged)
    } else {
        Err(Error::new(format!("no dropped menu: {judged:?}")))
    }
}

/// Where `label` sits in a `menu bar: titles` line: its x and width.
fn title(line: &str, label: &str) -> Option<(usize, usize)> {
    let titles = line.split("menu bar: titles ").nth(1)?;
    titles.trim_end().split('|').find_map(|title| {
        let (name, place) = title.rsplit_once('@')?;
        let (x, w) = place.split_once('+')?;
        (name == label).then_some((x.parse().ok()?, w.parse().ok()?))
    })
}

/// A place on the screen as the pointer takes it.
fn point(x: usize, y: usize) -> (i64, i64) {
    (i64::try_from(x).unwrap_or(0), i64::try_from(y).unwrap_or(0))
}

/// `test-compositor --boot appmenu`.
///
/// # Errors
///
/// Files' menus did not reach the bar, File did not drop, or New Tab did
/// not reach Files.
pub(super) fn test_appmenu(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/appmenu-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    carried.ports.extend(crate::apps::taken(arch, &["shelf"])?);
    carried.ports.push(crate::ports::File {
        path: "etc/appmenu-probe.sh".to_owned(),
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
            with_the_transcript(&Error::new(format!("{arch}: appmenu: {why}")), watching)
        };
        let titled = |l: &String| l.contains("menu bar: titles") && l.contains("|File@");
        if !watching.read_more(Instant::now() + SETTLE, |lines| lines.iter().any(titled))? {
            return Err(fail("Files did not reach the bar", watching));
        }
        let Some((x, w)) = watching
            .after()
            .iter()
            .rev()
            .find(|l| titled(l))
            .and_then(|l| title(l, "File"))
        else {
            return Err(fail("the titles line has no File", watching));
        };
        println!("  {arch}: appmenu: File is at {x}+{w}");
        std::thread::sleep(Duration::from_secs(2));
        let from = watching.after().len();
        at(&mut qmp, point(x + w / 2, 12))?;
        std::thread::sleep(Duration::from_millis(300));
        press_here(&mut qmp)?;
        let dropped = watching.read_more(Instant::now() + Duration::from_secs(10), |lines| {
            lines
                .iter()
                .skip(from)
                .any(|l| l.trim_end().ends_with("menu bar: open File"))
        })?;
        if !dropped {
            return Err(fail("a press on File dropped nothing", watching));
        }
        std::thread::sleep(Duration::from_secs(1));
        let dump = paths::build_dir(arch).join("appmenu.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        if let Err(error) = judge(&parse_ppm(&bytes)?, x) {
            return Err(fail(&error.to_string(), watching));
        }
        println!(
            "  {arch}: appmenu: File dropped (the screen is {})",
            dump.display()
        );
        // New Tab, the first row: the drop at 26, its column 5 lower, half
        // a 24 row more.
        let from = watching.after().len();
        at(&mut qmp, point(x + 40, 43))?;
        std::thread::sleep(Duration::from_millis(300));
        press_here(&mut qmp)?;
        let picked = watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            let after = || lines.iter().skip(from);
            after().any(|l| {
                l.contains("menu: dispatch appmenu 0x") && l.trim_end().ends_with(" 1: ok")
            }) && after().any(|l| l.contains("shelf: menu: picked 1"))
        })?;
        if !picked {
            return Err(fail("New Tab did not reach Files", watching));
        }
        println!("  {arch}: appmenu: New Tab went from the bar to Files");
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    fn screen(drop: bool) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0x40_u8; width * height * 3];
        let drop_at = (26..26 + 120).flat_map(|y| (100..100 + 260).map(move |x| (y, x)));
        for (y, x) in drop_at.filter(|_| drop) {
            let dark = (y - 26) % 24 == 12 && (x - 100) % 3 == 0 && x < 300;
            let value = if dark { 0x20 } else { 0xfa };
            let at = (y * width + x) * 3;
            pixels[at..at + 3].copy_from_slice(&[value; 3]);
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn a_dropped_menu_is_white_with_dark_words() {
        let judged = judge(&screen(true), 100).expect("a drop");
        assert!(judged.white >= WHITE && judged.dark >= DARK, "{judged:?}");
        assert!(judge(&screen(false), 100).is_err());
    }

    #[test]
    fn the_titles_line_gives_where_file_is() {
        let line = "xaqu-shell: menu bar: titles FX@10+30|Files@58+40|File@116+36|Edit@170+36";
        assert_eq!(title(line, "File"), Some((116, 36)));
        assert_eq!(title(line, "Go"), None);
    }
}
