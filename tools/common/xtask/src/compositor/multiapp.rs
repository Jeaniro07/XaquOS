//! `test-compositor --boot multiapp`: many apps, and what the hemat does
//! with them.
//!
//! A script in the guest opens two apps of two classes, puts one of them
//! behind, and waits longer than the five minutes an app needs in the
//! background before it sleeps. The compositor says where every app is and
//! how many of them are asleep; the guest says what each of the groups
//! holds. The host reads both: the app that was put behind is asleep, its
//! group holds no more than it did and has the background's weight, and
//! the app on the screen is awake and has the whole processor.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::boot::{build_image, with_the_transcript};
use super::{MARKER, Programs};
use crate::args::Args;
use crate::paths::Arch;
use crate::qemu::Watching;
use crate::{Error, Result};

/// How long the guest waits for the apps to fall asleep: five minutes, and
/// a minute more, so that the tick that puts them to sleep has run.
const WAIT: u64 = 360;

/// The guest's side: two apps of two classes, one of them behind, and the
/// wait.
///
/// The groups are read with the shell's own `read`, so that nothing but the
/// shell is needed in the guest.
const SCRIPT: &str = r#"#!/bin/sh
exec 2>&1
until hyprctl version > /dev/null 2>&1; do sleep 0.2; done
/bin/shelf &
sleep 8
/bin/foot --hold &
sleep 8
# The foot goes behind the shelf.
hyprctl dispatch minimize
sleep 6
for d in /sys/fs/cgroup/hemat/*; do
  [ -d "$d" ] || continue
  mem=
  weight=
  while read v < "$d/memory.current"; do mem=$v; done
  while read v < "$d/cpu.weight"; do weight=$v; done
  echo "the app ${d##*/} holds $mem with weight $weight"
done
sleep 360
hyprctl dispatch hemat state
for d in /sys/fs/cgroup/hemat/*; do
  [ -d "$d" ] || continue
  mem=
  weight=
  while read v < "$d/memory.current"; do mem=$v; done
  while read v < "$d/cpu.weight"; do weight=$v; done
  echo "the app ${d##*/} holds $mem with weight $weight"
done
"#;

/// What the guest said of a group: what it holds, and what weight it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Group {
    /// The bytes it holds.
    bytes: u64,
    /// The processor's weight it has.
    weight: u64,
}

/// What the compositor said of an app: its group and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
struct App {
    /// Its group, `app-foot-1`.
    group: String,
    /// Where it is: `Aktif`, `Latar` or `Tidur`.
    state: String,
}

/// What the boot saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Judged {
    /// The apps that fell asleep.
    asleep: usize,
    /// The apps that are on the screen.
    awake: usize,
}

/// How many apps the compositor said are asleep: `the apps are asleep: 2`.
fn said_asleep(lines: &[String]) -> Option<usize> {
    lines.iter().find_map(|line| {
        let rest = line.split("the apps are asleep: ").nth(1)?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// What the compositor said of every app: `hemat: foot app-foot-1 Tidur`.
fn said_apps(lines: &[String]) -> Vec<App> {
    lines
        .iter()
        .filter_map(|line| {
            let rest = line.split("hemat: ").nth(1)?;
            let mut words = rest.split_whitespace();
            let _class = words.next()?;
            let group = words.next()?;
            let state = words.next()?;
            if !matches!(state, "Aktif" | "Latar" | "Tidur") {
                return None;
            }
            Some(App {
                group: group.to_owned(),
                state: state.to_owned(),
            })
        })
        .collect()
}

/// What the guest said of every group: `the app app-foot-1 holds 900 with
/// weight 10`.
///
/// The guest says of every group twice: once before the wait and once
/// after it, and the two readings are what the judge compares.
fn said_groups(lines: &[String]) -> Vec<(String, Group)> {
    lines
        .iter()
        .filter_map(|line| {
            let rest = line.split("the app ").nth(1)?;
            let mut words = rest.split_whitespace();
            let group = words.next()?;
            if words.next()? != "holds" {
                return None;
            }
            let bytes = words.next()?.parse().ok()?;
            if words.next()? != "with" {
                return None;
            }
            if words.next()? != "weight" {
                return None;
            }
            let weight = words.next()?.parse().ok()?;
            Some((group.to_owned(), Group { bytes, weight }))
        })
        .collect()
}

/// The readings of every group, in the order the guest said them.
fn readings(lines: &[String]) -> BTreeMap<String, Vec<Group>> {
    let mut every: BTreeMap<String, Vec<Group>> = BTreeMap::new();
    for (group, said) in said_groups(lines) {
        every.entry(group).or_default().push(said);
    }
    every
}

/// The boot's verdict, from what the console said.
///
/// The app that is asleep must have the background's weight and must hold
/// no more than it did before the wait; the app on the screen must have
/// the whole processor.
fn judge(lines: &[String]) -> Result<Judged> {
    let asleep = said_asleep(lines).unwrap_or(0);
    let apps = said_apps(lines);
    let every = readings(lines);
    let mut judged = Judged {
        asleep: 0,
        awake: 0,
    };
    for app in &apps {
        let Some(said) = every.get(&app.group) else {
            return Err(Error::new(format!(
                "multiapp: nothing was said of the group {}",
                app.group
            )));
        };
        let (Some(first), Some(last)) = (said.first(), said.last()) else {
            return Err(Error::new(format!(
                "multiapp: the group {} was read once, not twice",
                app.group
            )));
        };
        match app.state.as_str() {
            "Tidur" => {
                judged.asleep += 1;
                if last.weight != 10 {
                    return Err(Error::new(format!(
                        "multiapp: the sleeping app {} has the weight {}, not 10",
                        app.group, last.weight
                    )));
                }
                if last.bytes > first.bytes {
                    return Err(Error::new(format!(
                        "multiapp: the sleeping app {} holds more than it did: {} and {}",
                        app.group, first.bytes, last.bytes
                    )));
                }
            }
            "Aktif" => {
                judged.awake += 1;
                if last.weight != 100 {
                    return Err(Error::new(format!(
                        "multiapp: the app on the screen {} has the weight {}, not 100",
                        app.group, last.weight
                    )));
                }
            }
            _ => {}
        }
    }
    if judged.asleep == 0 {
        return Err(Error::new("multiapp: no app fell asleep"));
    }
    if judged.awake == 0 {
        return Err(Error::new("multiapp: no app was on the screen"));
    }
    if asleep != judged.asleep {
        return Err(Error::new(format!(
            "multiapp: the compositor said {asleep} apps were asleep, and {judged:?} say otherwise"
        )));
    }
    Ok(judged)
}

/// `test-compositor --boot multiapp`.
///
/// # Errors
///
/// The apps never fell asleep, or the groups were not what the hemat says
/// they should be.
pub(super) fn test_multiapp(arch: Arch, programs: &Programs, args: &Args) -> Result<()> {
    let (mut config, mut carried) = super::xaqu::desktop(arch, args, false)?;
    config.push_str("exec-once = /bin/sh /etc/multiapp-probe.sh\n");
    carried.busybox = Some(
        crate::busybox::installed_program(arch)
            .ok_or_else(|| Error::new(format!("{arch}: no busybox for the probe")))?,
    );
    carried
        .ports
        .extend(crate::apps::taken(arch, &["shelf", "foot"])?);
    carried.ports.push(crate::ports::File {
        path: "etc/multiapp-probe.sh".to_owned(),
        mode: 0o755,
        content: crate::ports::Content::Bytes(SCRIPT.as_bytes().to_vec()),
    });
    let (image, kernel) = build_image(arch, programs, &config, carried, args)?;
    let mut qemu_args = args.clone();
    qemu_args.display = false;
    let hook = |watching: &mut Watching<'_>| -> Result<()> {
        watching.stop_when_done();
        // The apps fall asleep after five minutes; the guest waits longer.
        let said =
            watching.read_more(Instant::now() + Duration::from_secs(WAIT + 180), |lines| {
                lines
                    .iter()
                    .any(|line| line.contains("the apps are asleep:"))
            })?;
        if !said {
            return Err(with_the_transcript(
                &Error::new(format!("{arch}: multiapp: the apps never fell asleep")),
                watching,
            ));
        }
        let judged = judge(watching.after())?;
        println!(
            "  {arch}: multiapp: {} apps fell asleep, {} are on the screen",
            judged.asleep, judged.awake
        );
        Ok(())
    };
    let _ = crate::qemu::watch_then(arch, &image, &kernel, &qemu_args, MARKER, hook)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{judge, said_apps, said_asleep, said_groups};

    /// What the compositor says of two apps: one asleep, one on the screen.
    fn said() -> Vec<String> {
        [
            "hyprix: hemat: foot app-foot-1 Tidur",
            "hyprix: hemat: shelf app-shelf-2 Aktif",
            "hyprix: hemat: the apps are asleep: 1",
        ]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
    }

    /// What the guest says of the groups, before and after the wait.
    fn held() -> Vec<String> {
        [
            "the app app-foot-1 holds 900 with weight 100",
            "the app app-shelf-2 holds 300 with weight 100",
            "the app app-foot-1 holds 400 with weight 10",
            "the app app-shelf-2 holds 300 with weight 100",
        ]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
    }

    #[test]
    fn the_amount_line_says_what_it_saw() {
        assert_eq!(
            said_asleep(&["hyprix: hemat: the apps are asleep: 2".to_owned()]),
            Some(2)
        );
        assert_eq!(said_asleep(&["nothing here".to_owned()]), None);
    }

    #[test]
    fn the_compositors_lines_say_where_every_app_is() {
        let apps = said_apps(&said());
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].group, "app-foot-1");
        assert_eq!(apps[0].state, "Tidur");
        assert_eq!(apps[1].state, "Aktif");
        // The count of the sleeping ones is not an app.
        assert_eq!(
            said_apps(&["hyprix: hemat: the apps are asleep: 1".to_owned()]).len(),
            0
        );
    }

    #[test]
    fn the_guests_lines_say_what_the_groups_hold() {
        let groups = said_groups(&held());
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0].0, "app-foot-1");
        assert_eq!(groups[0].1.bytes, 900);
        assert_eq!(groups[0].1.weight, 100);
        assert_eq!(groups[2].1.weight, 10);
    }

    #[test]
    fn the_many_apps_sleep_after_five_minutes() {
        let mut lines = said();
        lines.extend(held());
        let judged = judge(&lines).expect("the apps slept");
        assert_eq!(judged.asleep, 1);
        assert_eq!(judged.awake, 1);
    }

    /// The app on the screen never sleeps, and a group is never taken away
    /// while something is in it.
    #[test]
    fn the_app_on_the_screen_never_sleeps() {
        let mut lines = said();
        lines.extend(held());
        // The app on the screen is said to be asleep: the judge refuses.
        let wrong = lines
            .iter()
            .map(|line| line.replace("app-shelf-2 Aktif", "app-shelf-2 Tidur"))
            .collect::<Vec<String>>();
        assert!(judge(&wrong).is_err(), "the awake app cannot sleep");
        // And an app that fell asleep but kept the processor is refused.
        let wrong = lines
            .iter()
            .map(|line| line.replace("holds 400 with weight 10", "holds 400 with weight 100"))
            .collect::<Vec<String>>();
        assert!(judge(&wrong).is_err(), "a sleeping app is quiet");
    }
}
