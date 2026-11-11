//! `cargo xtask measure-desktop`: `run-compositor`'s desktop booted and
//! timed, then read after it has idled, from the serial console.
//!
//! Per boot it prints how long from QEMU's start until the dock was drawn
//! and until `graphical.target` was active; the kernel's `root     / ...`
//! line, which tells a boot that installed the system on the btrfs root
//! (the first after an image is rebuilt, about 4 s slower) from one that
//! kept it; and, after [`IDLE`], `MemTotal` - `MemAvailable` and each
//! process's `VmRSS`, asked of the shell on the console. `--boots N` boots it
//! N times in a row, so `--boots 2` after a rebuild gives a kept boot too.
//!
//! The machine is the one `cargo xtask run-compositor --vnc :0 --no-gl
//! --memory 4096` boots, with its btrfs root kept from boot to boot; any of
//! those given on the command line wins.

use std::time::{Duration, Instant};

use crate::args::Args;
use crate::qemu::Watching;
use crate::{Error, Result};

/// How long the desktop idles before it is measured.
const IDLE: Duration = Duration::from_secs(65);

/// How long the dock and `graphical.target` are waited for after the root
/// line, and the shell's answers after they were asked for.
const PATIENCE: Duration = Duration::from_secs(300);

/// The kernel's line naming the root file system, printed before init
/// starts: what the boot is watched until.
const ROOT: &str = "root     / ";

/// What the shell on the console is given: the memory lines, then each
/// process's `VmRSS` and name, between two markers.
const ASK: &[&str] = &[
    "echo ===MEASURE\n",
    "while read k v u; do case $k in MemTotal:|MemAvailable:) echo $k $v $u;; esac; done \
     < /proc/meminfo\n",
    "for f in /proc/[0-9]*/status; do n=; while read k v r; do case $k in Name:) n=$v;; \
     VmRSS:) echo RSS $v $n;; esac; done < $f; done\n",
    "echo ===END\n",
];

/// `cargo xtask measure-desktop [--boots N]`.
pub(super) fn measure_desktop(args: &Args) -> Result<()> {
    let mut args = args.clone();
    // `run-compositor`'s machine: its btrfs root, its single screen.
    args.command = Some("run-compositor".to_owned());
    let _ = args.vnc.get_or_insert_with(|| ":0".to_owned());
    if !args.gl {
        args.no_gl = true;
    }
    if !args.memory_given {
        args.memory = 4096;
    }
    let (arch, image, kernel, args) = super::run::desktop_to_boot(&args)?;
    let args = super::run::screen(arch, &args)?;
    let boots = args.boots.unwrap_or(1);
    let mut reports = Vec::new();
    for boot in 1..=boots {
        println!("  measure-desktop: boot {boot} of {boots}");
        let mut report = String::new();
        let _ = crate::qemu::watch_then(arch, &image, &kernel, &args, ROOT, |watching| {
            report = measure(watching)?;
            watching.stop_when_done();
            Ok(())
        })?;
        reports.push(report);
    }
    for (boot, report) in reports.iter().enumerate() {
        println!("\nboot {} of {boots}\n{report}", boot + 1);
    }
    Ok(())
}

/// One boot, measured from the root line on.
fn measure(watching: &mut Watching<'_>) -> Result<String> {
    let started = Instant::now()
        .checked_sub(watching.elapsed())
        .unwrap_or_else(Instant::now);
    let root = watching.lines().last().cloned().unwrap_or_default();
    let (mut dock, mut graphical) = (None, None);
    let _ = watching.read_more(Instant::now() + PATIENCE, |lines| {
        let line = lines.last().map(String::as_str).unwrap_or_default();
        if dock.is_none() && dock_drawn(line) {
            dock = Some(started.elapsed());
        }
        if graphical.is_none() && graphical_active(line) {
            graphical = Some(started.elapsed());
        }
        dock.is_some() && graphical.is_some()
    })?;
    // Idle, the lines read as they come rather than left in the pipe.
    let _ = watching.read_more(Instant::now() + IDLE, |_| false)?;
    let mark = watching.after().len();
    for line in ASK {
        watching.type_in(line.as_bytes())?;
    }
    let answered = watching.read_more(Instant::now() + PATIENCE, |lines| {
        lines
            .get(mark..)
            .unwrap_or_default()
            .iter()
            .any(|line| line.trim() == "===END")
    })?;
    if !answered {
        return Err(Error::new("the shell on the console did not answer"));
    }
    let memory = memory(watching.after().get(mark..).unwrap_or_default());
    let mut report = format!(
        "QEMU start -> dock drawn: {}\nQEMU start -> graphical.target: {}\n{}\n",
        seconds(dock),
        seconds(graphical),
        root.trim()
    );
    for (kib, name) in &memory.rss {
        report.push_str(&format!("RSS {kib} {name}\n"));
    }
    report.push_str(&match memory.used_kib {
        Some(used) => format!(
            "used (MemTotal - MemAvailable): {} MiB",
            (used + 512) / 1024
        ),
        None => "used (MemTotal - MemAvailable): not read".to_owned(),
    });
    Ok(report)
}

/// `6.06 s`, or `not seen`.
fn seconds(at: Option<Duration>) -> String {
    at.map_or_else(
        || "not seen".to_owned(),
        |at| format!("{:.2} s", at.as_secs_f64()),
    )
}

/// yoru-shell's line once its dock is drawn.
fn dock_drawn(line: &str) -> bool {
    line.contains("the dock is up")
}

/// init's line once `graphical.target` is reached.
fn graphical_active(line: &str) -> bool {
    line.contains("graphical.target") && (line.contains("active") || line.contains("reached"))
}

/// What the shell answered to [`ASK`].
#[derive(Debug, Default, PartialEq, Eq)]
struct Memory {
    /// `MemTotal` - `MemAvailable`, in KiB.
    used_kib: Option<u64>,
    /// Each process's `VmRSS` in kB, and its name.
    rss: Vec<(u64, String)>,
}

/// [`Memory`] from the console's lines. The echo of what was typed starts
/// with the prompt, so no answer is taken from it.
fn memory(lines: &[String]) -> Memory {
    let (mut total, mut available) = (None, None);
    let mut rss = Vec::new();
    for line in lines {
        let mut words = line.split_whitespace();
        let (Some(first), Some(number)) = (
            words.next(),
            words.next().and_then(|word| word.parse::<u64>().ok()),
        ) else {
            continue;
        };
        match first {
            "MemTotal:" => total = Some(number),
            "MemAvailable:" => available = Some(number),
            "RSS" => rss.push((number, words.collect::<Vec<_>>().join(" "))),
            _ => {}
        }
    }
    Memory {
        used_kib: total.zip(available).map(|(t, a)| t.saturating_sub(a)),
        rss,
    }
}

#[cfg(test)]
mod tests {
    use super::{Memory, dock_drawn, graphical_active, memory};

    #[test]
    fn the_answers_are_read_and_the_echo_is_not() {
        let lines: Vec<String> = [
            "%m%# echo ===MEASURE",
            "===MEASURE",
            "%m%# while read k v u; do case $k in MemTotal:|MemAvailable:) echo $k $v $u;; esac",
            "MemTotal: 4146808 kB",
            "MemAvailable: 4047109 kB\r",
            "%m%# for f in /proc/[0-9]*/status; do echo RSS $v $n; done",
            "RSS 1748 init",
            "RSS 10024 wallpaper",
            "===END",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(
            memory(&lines),
            Memory {
                used_kib: Some(4_146_808 - 4_047_109),
                rss: vec![(1748, "init".to_owned()), (10024, "wallpaper".to_owned())],
            }
        );
        assert_eq!(memory(&lines[..3]).used_kib, None);
    }

    #[test]
    fn the_dock_and_graphical_target_are_told_by_their_lines() {
        assert!(dock_drawn(
            "yoru-shell: the dock is up, 1896x60, 4 workspaces, 3 launchers"
        ));
        assert!(!dock_drawn(
            "yoru-shell: the right sidebar is up, 2 widgets"
        ));
        assert!(graphical_active("  init     graphical.target: active"));
        assert!(!graphical_active("  init     graphical.target: starting"));
    }
}
