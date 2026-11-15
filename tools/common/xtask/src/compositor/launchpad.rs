//! `test-compositor --boot launchpad`: SUPER L opens the Launchpad, the
//! shell's grid of every installed application, and a press on a tile
//! runs the application on it.
//!
//! A script in the guest opens the Launchpad once hyprix answers. The
//! shell says how many applications the grid holds and where each tile
//! sits (`launchpad: tiles x,y,w,h …`), the host dumps the screen and
//! judges the first tile there, presses its middle, and waits for the
//! shell to say what it ran and for that application's window to come
//! up. Required: the tile the shell named on the screen, `launchpad:
//! run /bin/…: ok` and a window that was not up before. Kept as
//! `launchpad.ppm`.

use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::files::{at, press_here};
use super::{MARKER, Programs, SETTLE};
use crate::args::Args;
use crate::display::{DEVICE_ID, Image, Qmp, free_port, parse_ppm};
use crate::paths::{self, Arch};
use crate::qemu::Watching;
use crate::{Error, Result};

/// The tile's place on the screen, as the shell said it: its x, its y,
/// its width and its height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tile {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Tile {
    /// Its middle, where a press on the tile lands.
    fn centre(&self) -> (usize, usize) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// The tiles a page of the grid holds: eight columns and five rows, the
/// shell's grid (`launchpad.rs`'s `COLUMNS` and `ROWS`).
const CAP: usize = 40;

/// The surface the launchpad draws on: not an application, so never the
/// window the boot waits for.
const PAD: &str = "xaqu-launchpad";

/// The colour of a tile: Nocturne's n900 in Xaqu's glass theme, the
/// theme the shell draws with until a person chooses one of their own
/// (`theme.rs`'s `Tokens::GLASS`).
const TILE: [u8; 3] = [0xec, 0xed, 0xf0];

/// How far a pixel may be from the tile's colour and still be it: the
/// compositor's rounding, two at most.
const NEAR: i32 = 4;

/// The guest's side: the Launchpad, once hyprix answers, and the
/// windows every two seconds, so the host can see the application that
/// ran.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
hyprctl dispatch exec '/bin/xaqu-shell --launchpad'
while :; do
  echo "launchpad-probe: windows $(hyprctl clients | grep -E '^[[:space:]]*class:' | tr '\n' ';')"
  sleep 2
done
"#;

/// The applications and the pages a `launchpad is up, N apps, P pages`
/// line says.
fn up(line: &str) -> Option<(usize, usize)> {
    let said = line.split_once("launchpad is up, ")?.1;
    let (apps, said) = said.split_once(" apps, ")?;
    let (pages, _) = said.split_once(" pages")?;
    Some((apps.trim().parse().ok()?, pages.trim().parse().ok()?))
}

/// The tiles a `launchpad: tiles x,y,w,h …` line names, in the grid's
/// order.
fn tiles(line: &str) -> Option<Vec<Tile>> {
    let said = line.split_once("launchpad: tiles ")?.1;
    said.split_whitespace()
        .map(|place| {
            let mut numbers = place.split(',');
            Some(Tile {
                x: numbers.next()?.parse().ok()?,
                y: numbers.next()?.parse().ok()?,
                w: numbers.next()?.parse().ok()?,
                h: numbers.next()?.parse().ok()?,
            })
        })
        .collect()
}

/// The classes of the windows a probe line names; none for another line.
fn classes(line: &str) -> Option<Vec<String>> {
    let said = line.split_once("launchpad-probe: windows")?.1;
    Some(
        said.split(';')
            .filter_map(|entry| entry.trim().strip_prefix("class: "))
            .map(|name| name.trim().to_owned())
            .collect(),
    )
}

/// What the launchpad said and drew.
#[derive(Debug)]
struct Judged {
    /// The applications the grid holds.
    apps: usize,
    /// The pages they need.
    pages: usize,
    /// Where the first tile is.
    tile: Tile,
    /// How much of that tile is the tile's own colour.
    light: usize,
}

impl Judged {
    /// The middle of the first tile, where a press on it lands.
    fn centre(&self) -> (usize, usize) {
        self.tile.centre()
    }
}

/// The launchpad, as the shell said it and the screen shows it.
///
/// The facts: the line the shell says when the grid is up, the line that
/// says where its tiles are, and the picture of the screen with it up.
///
/// # Errors
///
/// The shell said nothing about the grid, or the tile it named is not
/// drawn where it said.
fn judge(said_up: &str, said_tiles: &str, screen: &Image) -> Result<Judged> {
    let Some((apps, pages)) = up(said_up) else {
        return Err(Error::new(format!(
            "the launchpad did not say how many apps it has: {said_up}"
        )));
    };
    if apps == 0 || pages == 0 {
        return Err(Error::new(format!(
            "the launchpad holds {apps} apps on {pages} pages"
        )));
    }
    let Some(tiles) = tiles(said_tiles) else {
        return Err(Error::new(format!(
            "the launchpad did not say where its tiles are: {said_tiles}"
        )));
    };
    // One rectangle for every application of the page in hand.
    if tiles.len() != apps.min(CAP) {
        return Err(Error::new(format!(
            "the launchpad holds {apps} apps and named {} tiles",
            tiles.len()
        )));
    }
    let Some(tile) = tiles.first().copied() else {
        return Err(Error::new("the launchpad named no tile"));
    };
    if tile.x + tile.w > screen.width || tile.y + tile.h > screen.height {
        return Err(Error::new(format!(
            "the first tile at {tile:?} is off the screen"
        )));
    }
    let light = tile_light(screen, tile);
    // The tile is a rounded square of its own colour with a glyph in
    // it: the corners (radius 20) take a twentieth of an 87 pixel tile
    // and the glyph a fifth, so the colour covers over seven tenths of
    // it. Three fifths is the floor that keeps a margin.
    let area = tile.w * tile.h;
    if light * 5 < area * 3 {
        return Err(Error::new(format!(
            "no tile at {tile:?}: {light} of {area} pixels are its own"
        )));
    }
    Ok(Judged {
        apps,
        pages,
        tile,
        light,
    })
}

/// The pixels of `tile` that are the tile's own colour.
///
/// Ponytail: the tile is judged by the colour it is drawn in and not by
/// the picture's layout, so a wallpaper of another picture does not
/// break the boot. Upgrade path: the tiles' icons, when the shell says
/// which icon each tile holds.
fn tile_light(screen: &Image, tile: Tile) -> usize {
    let mut light = 0;
    for y in tile.y..(tile.y + tile.h).min(screen.height) {
        for x in tile.x..(tile.x + tile.w).min(screen.width) {
            let at = (y * screen.width + x) * 3;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3) else {
                continue;
            };
            if (i32::from(r) - i32::from(TILE[0])).abs() <= NEAR
                && (i32::from(g) - i32::from(TILE[1])).abs() <= NEAR
                && (i32::from(b) - i32::from(TILE[2])).abs() <= NEAR
            {
                light += 1;
            }
        }
    }
    light
}

/// `test-compositor --boot launchpad`.
///
/// # Errors
///
/// The grid never came, the tile the shell named was not on the screen,
/// or the tile that was pressed ran no application.
#[expect(
    clippy::too_many_lines,
    reason = "one boot read top to bottom: the grid, the tile and what the press did"
)]
pub(super) fn test_launchpad(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/launchpad-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    // shelf installed, its desktop entry with it: the grid's tile.
    carried.ports.extend(crate::apps::taken(arch, &["shelf"])?);
    carried.ports.push(crate::ports::File {
        path: "etc/launchpad-probe.sh".to_owned(),
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
            with_the_transcript(&Error::new(format!("{arch}: launchpad: {why}")), watching)
        };
        // The grid is up and says where its tiles are.
        if !watching.read_more(Instant::now() + SETTLE, |lines| {
            lines.iter().any(|l| l.contains("launchpad is up, "))
                && lines.iter().any(|l| l.contains("launchpad: tiles "))
        })? {
            return Err(fail("the launchpad never came up", watching));
        }
        let Some(said_up) = watching
            .after()
            .iter()
            .rev()
            .find(|l| l.contains("launchpad is up, "))
            .cloned()
        else {
            return Err(fail(
                "the launchpad did not say how many apps it has",
                watching,
            ));
        };
        let Some(said_tiles) = watching
            .after()
            .iter()
            .rev()
            .find(|l| l.contains("launchpad: tiles "))
            .cloned()
        else {
            return Err(fail(
                "the launchpad did not say where its tiles are",
                watching,
            ));
        };
        // The picture of the grid, and the tile the shell named in it.
        std::thread::sleep(Duration::from_secs(2));
        let dump = paths::build_dir(arch).join("launchpad.ppm");
        qmp.screendump(Some(DEVICE_ID), &dump)?;
        let bytes = std::fs::read(&dump)
            .map_err(|error| Error::new(format!("reading {}: {error}", dump.display())))?;
        let judged = judge(&said_up, &said_tiles, &parse_ppm(&bytes)?)
            .map_err(|error| fail(&error.to_string(), watching))?;
        println!(
            "  {arch}: launchpad: {} apps on {} pages, the first tile at {:?} ({} of its pixels are its own)",
            judged.apps, judged.pages, judged.tile, judged.light
        );
        // The windows that were up before the press.
        let _ = watching.read_more(Instant::now() + Duration::from_secs(15), |lines| {
            lines.iter().any(|l| l.contains("launchpad-probe: windows"))
        })?;
        let before: Vec<String> = watching
            .after()
            .iter()
            .rev()
            .find_map(|l| classes(l))
            .unwrap_or_default();
        // The tile, in the middle of it.
        let (x, y) = judged.centre();
        at(
            &mut qmp,
            (i64::try_from(x).unwrap_or(0), i64::try_from(y).unwrap_or(0)),
        )?;
        std::thread::sleep(Duration::from_millis(500));
        press_here(&mut qmp)?;
        let ran = watching.read_more(Instant::now() + Duration::from_secs(30), |lines| {
            lines
                .iter()
                .any(|l| l.contains("launchpad: run ") && l.trim_end().ends_with(": ok"))
        })?;
        if !ran {
            return Err(fail("the tile ran nothing", watching));
        }
        // The application's window, which was not up before.
        //
        // Ponytail: any window that was not up before the press counts,
        // so the boot does not name the application the grid's first
        // tile holds. Upgrade path: the tile's application, when the
        // shell says which application each tile holds.
        let opened = watching.read_more(Instant::now() + Duration::from_secs(60), |lines| {
            let after = lines
                .iter()
                .rev()
                .find_map(|l| classes(l))
                .unwrap_or_default();
            after
                .iter()
                .any(|name| name != PAD && !before.contains(name))
        })?;
        if !opened {
            return Err(fail("the application's window did not come up", watching));
        }
        println!(
            "  {arch}: launchpad: the tile ran its application (the screen is {})",
            dump.display()
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Image, Tile, judge, tiles, up};

    /// The screen, as the Xaqu boot sets it.
    const SCREEN: (usize, usize) = (1024, 768);

    /// What the shell says when the grid is up with one application.
    const UP: &str = "xaqu-shell: launchpad is up, 1 apps, 1 pages";
    /// Where its one tile is.
    const TILES: &str = "xaqu-shell: launchpad: tiles 81,40,87,87";

    /// The tile the shell named.
    fn tile() -> Tile {
        Tile {
            x: 81,
            y: 40,
            w: 87,
            h: 87,
        }
    }

    /// A screen with a tile at `tile`, in the tile's own colour.
    fn screen(tile: Option<Tile>) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0x60_u8; width * height * 3];
        if let Some(tile) = tile {
            for y in tile.y..tile.y + tile.h {
                for x in tile.x..tile.x + tile.w {
                    let at = (y * width + x) * 3;
                    pixels[at..at + 3].copy_from_slice(&[0xec, 0xed, 0xf0]);
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
    fn the_up_line_says_how_many_apps_and_pages() {
        assert_eq!(up(UP), Some((1, 1)));
        assert_eq!(
            up("xaqu-shell: launchpad is up, 12 apps, 2 pages"),
            Some((12, 2))
        );
        assert_eq!(up("xaqu-shell: the dock is up"), None);
    }

    #[test]
    fn the_tiles_line_names_where_the_tiles_are() {
        assert_eq!(tiles(TILES), Some(vec![tile()]));
        assert_eq!(
            tiles("xaqu-shell: launchpad: tiles 81,40,87,87 191,40,87,87"),
            Some(vec![
                tile(),
                Tile {
                    x: 191,
                    y: 40,
                    w: 87,
                    h: 87
                }
            ])
        );
        assert_eq!(tiles("xaqu-shell: launchpad is up, 1 apps, 1 pages"), None);
    }

    #[test]
    fn the_first_tile_is_pressed_in_its_middle() {
        assert_eq!(tile().centre(), (124, 83));
    }

    #[test]
    fn the_tile_is_drawn_where_the_shell_said_it_is() {
        let judged = judge(UP, TILES, &screen(Some(tile()))).expect("the tile is there");
        assert_eq!(judged.apps, 1);
        assert_eq!(judged.pages, 1);
        assert_eq!(judged.tile, tile());
        assert!(judged.light > 7_000, "{} pixels", judged.light);
        assert!(
            judge(UP, TILES, &screen(None)).is_err(),
            "no tile on the screen"
        );
    }

    #[test]
    fn a_launchpad_with_no_applications_is_no_launchpad() {
        let said = "xaqu-shell: launchpad is up, 0 apps, 0 pages";
        assert!(judge(said, TILES, &screen(Some(tile()))).is_err());
    }

    #[test]
    fn the_tiles_must_be_as_many_as_the_apps() {
        let said = "xaqu-shell: launchpad: tiles 81,40,87,87 191,40,87,87";
        assert!(judge(UP, said, &screen(Some(tile()))).is_err());
    }
}
