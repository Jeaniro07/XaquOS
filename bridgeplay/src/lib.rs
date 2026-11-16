//! `bridgeplay`: what to run for Steam and for a Windows program, and what
//! to tell the person when the volume that holds it is not there.
//!
//! Kept to plain data so the host tests it; `main.rs` is the only part that
//! touches the system.

/// Steam's launcher on the desktop image (docs/STEAM.md).
pub const STEAM_SCRIPT: &str = "/steam/desktop.sh";
/// Wine's 64-bit loader on the Wine volume (docs/WINE.md).
pub const WINE_LOADER: &str = "/usr/lib/wine/wine64";
/// Wine's server, beside the loader.
pub const WINE_SERVER: &str = "/usr/lib/wine/wineserver64";
/// Where the prefix lives when the data volume is there: on disk, so it
/// costs no memory.
pub const PREFIX_DISK: &str = "/data/wine/prefix";
/// The prefix without a data volume: tmpfs, as the Wine gate does.
pub const PREFIX_TMP: &str = "/tmp/wine/prefix";

/// What the person asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Open Steam.
    Steam,
    /// Run a Windows program, with its arguments.
    Run(Vec<String>),
    /// Say what is present and how much memory is free.
    Doctor,
    /// Print the version.
    Version,
    /// Print the usage; the text says why when it was not asked for.
    Usage(Option<String>),
}

/// Read the command line, without the program's own name.
#[must_use]
pub fn parse(args: &[String]) -> Cmd {
    let mut it = args.iter();
    match it.next().map(String::as_str) {
        Some("steam") => Cmd::Steam,
        Some("doctor") => Cmd::Doctor,
        Some("--version" | "-V") => Cmd::Version,
        Some("-h" | "--help" | "help") | None => Cmd::Usage(None),
        Some("run") => {
            let rest: Vec<String> = it.cloned().collect();
            if rest.is_empty() {
                Cmd::Usage(Some("run needs a program".into()))
            } else {
                Cmd::Run(rest)
            }
        }
        Some(other) => Cmd::Usage(Some(format!("unknown command: {other}"))),
    }
}

/// A program to start: its path, arguments and environment.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// The program.
    pub program: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// Environment variables added to the caller's.
    pub env: Vec<(String, String)>,
}

/// Steam: the desktop's own script, which waits for the X server itself.
#[must_use]
pub fn steam_plan() -> Plan {
    Plan {
        program: "/bin/busybox".into(),
        args: vec!["sh".into(), STEAM_SCRIPT.into()],
        env: Vec::new(),
    }
}

/// A Windows program under Wine. `has_data` picks the prefix: on disk when
/// the data volume is mounted. Debug output off and no menu builder, so the
/// least memory and no stray process stay resident.
#[must_use]
pub fn wine_plan(program: &[String], has_data: bool) -> Plan {
    let prefix = if has_data { PREFIX_DISK } else { PREFIX_TMP };
    Plan {
        program: WINE_LOADER.into(),
        args: program.to_vec(),
        env: vec![
            ("WINEPREFIX".into(), prefix.into()),
            ("WINESERVER".into(), WINE_SERVER.into()),
            ("WINEDEBUG".into(), "-all".into()),
            ("WINEDLLOVERRIDES".into(), "winemenubuilder.exe=d".into()),
        ],
    }
}

/// The line to show when `path` (what `what` needs) is missing.
#[must_use]
pub fn missing(what: &str, path: &str, how: &str) -> String {
    format!("bridgeplay: {what} is not on this system ({path} not found). {how}")
}

/// `MemAvailable` in MiB, from `/proc/meminfo`'s text.
#[must_use]
pub fn mem_available_mib(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib / 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn parses_commands() {
        assert_eq!(parse(&v(&["steam"])), Cmd::Steam);
        assert_eq!(parse(&v(&["doctor"])), Cmd::Doctor);
        assert_eq!(parse(&v(&["--version"])), Cmd::Version);
        assert_eq!(parse(&[]), Cmd::Usage(None));
        assert_eq!(
            parse(&v(&["run", "a.exe", "-x"])),
            Cmd::Run(v(&["a.exe", "-x"]))
        );
    }

    #[test]
    fn bad_input_gives_usage_not_a_panic() {
        assert!(matches!(parse(&v(&["run"])), Cmd::Usage(Some(_))));
        assert!(matches!(parse(&v(&["nope"])), Cmd::Usage(Some(_))));
    }

    #[test]
    fn prefix_follows_the_data_volume() {
        let on = wine_plan(&v(&["a.exe"]), true);
        let off = wine_plan(&v(&["a.exe"]), false);
        assert!(on.env.contains(&("WINEPREFIX".into(), PREFIX_DISK.into())));
        assert!(off.env.contains(&("WINEPREFIX".into(), PREFIX_TMP.into())));
        assert_eq!(on.args, v(&["a.exe"]));
    }

    #[test]
    fn reads_free_memory() {
        let m = "MemTotal: 4096000 kB\nMemAvailable: 2097152 kB\n";
        assert_eq!(mem_available_mib(m), Some(2048));
        assert_eq!(mem_available_mib("nothing"), None);
    }
}
