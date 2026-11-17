//! What `xaqu-open` opens, and in which world.
//!
//! A desktop opens a file by handing it to `xdg-open`, which opens what it
//! knows by: a folder in Files, a page in the browser. What is not a
//! document -- a program, a package, a disc image -- belongs to a world, and
//! each world is run by a runtime the user installs. This crate is the
//! plumbing: it says which world a file belongs to, which runtime that
//! world is run by, and refuses, with the reason, when there is none.
//!
//! The runtimes themselves are not here. Xaqu runs a Linux program itself,
//! and the other worlds need what the user brings: Wine for a Windows
//! program, an Android runtime for an `.apk`, a macOS one for a `.app`. The
//! configuration says where they are, so that installing one is a matter of
//! writing a line, not of rebuilding anything.

#![forbid(unsafe_code)]

use std::path::Path;

/// Where the runtimes are named, in an image.
pub const CONFIG: &str = "/etc/xaqu/app-worlds.conf";

/// The world a file belongs to: what may run it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum World {
    /// A Linux program.
    Linux,
    /// A Windows program.
    Windows,
    /// An Android program.
    Android,
    /// A macOS program.
    MacOs,
    /// None of those: a document, a page, a folder.
    Other,
}

impl World {
    /// Every world, in the order they are listed.
    pub const ALL: [World; 5] = [
        World::Linux,
        World::Windows,
        World::Android,
        World::MacOs,
        World::Other,
    ];

    /// The name the configuration gives this world.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            World::Linux => "linux",
            World::Windows => "windows",
            World::Android => "android",
            World::MacOs => "macos",
            World::Other => "other",
        }
    }
}

/// What a file is, by what it is called and by what it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A Linux program: an ELF image, or one that carries itself.
    Program,
    /// A package of Linux programs in Xaqu's own format.
    Package,
    /// A Debian or RPM package.
    ForeignPackage,
    /// A Windows program.
    Windows,
    /// An Android program.
    Android,
    /// A macOS program.
    MacOs,
    /// A document, a page, a folder: none of the above.
    Other,
}

impl Kind {
    /// The world this kind is run in.
    #[must_use]
    pub const fn world(self) -> World {
        match self {
            Kind::Program | Kind::Package | Kind::ForeignPackage => World::Linux,
            Kind::Windows => World::Windows,
            Kind::Android => World::Android,
            Kind::MacOs => World::MacOs,
            Kind::Other => World::Other,
        }
    }
}

/// The world `path` belongs to: by its name first, and by what it holds
/// when the name says nothing.
#[must_use]
pub fn classify(path: &Path, head: &[u8]) -> Kind {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "exe" | "msi" | "com" => Kind::Windows,
        "apk" => Kind::Android,
        "app" | "dmg" => Kind::MacOs,
        "deb" | "rpm" => Kind::ForeignPackage,
        "fxpkg" => Kind::Package,
        "appimage" => Kind::Program,
        _ => classify_bytes(head),
    }
}

/// What a file holds, by what it starts with.
#[must_use]
pub fn classify_bytes(head: &[u8]) -> Kind {
    if head.starts_with(b"\x7fELF") {
        return Kind::Program;
    }
    if head.starts_with(b"MZ") {
        return Kind::Windows;
    }
    if head.starts_with(&[0xfe, 0xed, 0xfa, 0xce])
        || head.starts_with(&[0xfe, 0xed, 0xfa, 0xcf])
        || head.starts_with(&[0xce, 0xfa, 0xed, 0xfe])
        || head.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
    {
        return Kind::MacOs;
    }
    Kind::Other
}

/// The runtimes the worlds are run by, as the configuration names them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worlds {
    /// The loader a Linux program is run by, when it needs one.
    pub linux: Option<String>,
    /// The runtime a Windows program is run by.
    pub windows: Option<String>,
    /// The runtime an Android program is run by.
    pub android: Option<String>,
    /// The runtime a macOS program is run by.
    pub macos: Option<String>,
}

impl Worlds {
    /// The configuration, read from its text: a world's name, an `=`, and
    /// the program that runs it. A `#` begins a comment, and a line with
    /// nothing to say is no line at all.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut worlds = Self::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or_default().trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match key.trim().to_ascii_lowercase().as_str() {
                "linux" => worlds.linux = Some(value.to_owned()),
                "windows" => worlds.windows = Some(value.to_owned()),
                "android" => worlds.android = Some(value.to_owned()),
                "macos" | "mac" | "osx" => worlds.macos = Some(value.to_owned()),
                _ => {}
            }
        }
        worlds
    }

    /// The runtime `world` is run by, as the configuration names it.
    #[must_use]
    pub fn runtime(&self, world: World) -> Option<&str> {
        match world {
            World::Linux => self.linux.as_deref(),
            World::Windows => self.windows.as_deref(),
            World::Android => self.android.as_deref(),
            World::MacOs => self.macos.as_deref(),
            World::Other => None,
        }
    }
}

/// What to do with a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The program to run, and the arguments after the path.
    Run {
        /// The program: a path, or a name found in `PATH`.
        program: String,
        /// The arguments, the file among them.
        args: Vec<String>,
    },
    /// Nothing to run, and why.
    Refuse(String),
}

/// What to do with `path`, a file of the `kind` it is, in the world that
/// kind belongs to. `interp` is the loader the file asks for, when it is a
/// program that asks for one.
#[must_use]
pub fn plan(kind: Kind, path: &Path, worlds: &Worlds, interp: Option<&str>) -> Plan {
    let path = path.display().to_string();
    match kind {
        Kind::Program => match worlds.linux.as_deref() {
            Some(loader) => Plan::Run {
                program: loader.to_owned(),
                args: vec![path],
            },
            None => match interp {
                // A program that asks for a loader this system has not got
                // cannot run: the kernel looks for it and does not find it.
                Some(loader) if !Path::new(loader).exists() => Plan::Refuse(format!(
                    "{path} asks for {loader}, which this system has not got: install a Linux \
                     runtime and name its loader in {CONFIG} (`linux = ...`)"
                )),
                _ => Plan::Run {
                    program: path,
                    args: Vec::new(),
                },
            },
        },
        Kind::Package => Plan::Run {
            program: "/bin/pkg".to_owned(),
            args: vec!["install".to_owned(), path],
        },
        Kind::ForeignPackage => Plan::Refuse(format!(
            "{path} is a Debian or RPM package: Xaqu's package manager installs its own .fxpkg \
             packages, and a program from one of these needs a Linux runtime of its own"
        )),
        Kind::Windows => run_by(worlds.windows.as_deref(), &path, World::Windows),
        Kind::Android => run_by(worlds.android.as_deref(), &path, World::Android),
        Kind::MacOs => run_by(worlds.macos.as_deref(), &path, World::MacOs),
        Kind::Other => Plan::Run {
            program: "xdg-open".to_owned(),
            args: vec![path],
        },
    }
}

/// The plan for a world whose program is run by a runtime the user
/// installed: the runtime, with the file; or the refusal, with the way out.
/// A runtime named with words (`/bin/bridgeplay run`) runs them before the
/// file; they are split at spaces, so a path with a space in it cannot be
/// named (upgrade path: shell quoting, when a runtime needs it).
fn run_by(runtime: Option<&str>, path: &str, world: World) -> Plan {
    let mut words = runtime.unwrap_or_default().split_whitespace();
    match words.next() {
        Some(program) => Plan::Run {
            program: program.to_owned(),
            args: words.map(str::to_owned).chain([path.to_owned()]).collect(),
        },
        None => Plan::Refuse(format!(
            "there is no {} runtime on this system: install one and name it in {CONFIG} \
             (`{} = ...`)",
            world.name(),
            world.name()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{CONFIG, Kind, Plan, Worlds, classify, classify_bytes, plan};
    use std::path::Path;

    fn kind_of(name: &str, head: &[u8]) -> Kind {
        classify(Path::new(name), head)
    }

    #[test]
    fn a_name_says_which_world_a_file_is_in() {
        assert_eq!(kind_of("game.exe", b""), Kind::Windows);
        assert_eq!(kind_of("tool.msi", b""), Kind::Windows);
        assert_eq!(kind_of("app.apk", b""), Kind::Android);
        assert_eq!(kind_of("Photos.app", b""), Kind::MacOs);
        assert_eq!(kind_of("disc.dmg", b""), Kind::MacOs);
        assert_eq!(kind_of("thing.deb", b""), Kind::ForeignPackage);
        assert_eq!(kind_of("thing.rpm", b""), Kind::ForeignPackage);
        assert_eq!(kind_of("thing.fxpkg", b""), Kind::Package);
        assert_eq!(kind_of("thing.AppImage", b""), Kind::Program);
        assert_eq!(kind_of("notes.txt", b"hello"), Kind::Other);
    }

    #[test]
    fn a_name_that_says_nothing_is_told_by_the_file() {
        assert_eq!(classify_bytes(b"\x7fELFxxxx"), Kind::Program);
        assert_eq!(classify_bytes(b"MZ\x90\x00"), Kind::Windows);
        assert_eq!(classify_bytes(&[0xfe, 0xed, 0xfa, 0xcf]), Kind::MacOs);
        assert_eq!(classify_bytes(&[0xcf, 0xfa, 0xed, 0xfe]), Kind::MacOs);
        assert_eq!(classify_bytes(b"%PDF-1.7"), Kind::Other);
    }

    #[test]
    fn the_configuration_names_a_runtime_for_a_world() {
        let worlds = Worlds::parse(
            "# the runtimes\n\
             windows = /data/proton/files/bin/wine\n\
             android=/data/usr/bin/runner  \n\
             macos = /data/usr/bin/darling\n\
             linux = /data/linux/lib64/ld.so\n\
             \n\
             unknown = /bin/true\n",
        );
        assert_eq!(
            worlds.windows.as_deref(),
            Some("/data/proton/files/bin/wine")
        );
        assert_eq!(worlds.android.as_deref(), Some("/data/usr/bin/runner"));
        assert_eq!(worlds.macos.as_deref(), Some("/data/usr/bin/darling"));
        assert_eq!(worlds.linux.as_deref(), Some("/data/linux/lib64/ld.so"));
    }

    #[test]
    fn a_configuration_that_says_nothing_names_no_runtime() {
        let worlds = Worlds::parse("# nothing here\nwindows =\n");
        assert_eq!(worlds, Worlds::default());
        assert_eq!(worlds.runtime(super::World::Windows), None);
    }

    #[test]
    fn a_linux_program_runs_by_itself() {
        let plan = plan(
            Kind::Program,
            Path::new("/home/a/tool"),
            &Worlds::default(),
            None,
        );
        assert_eq!(
            plan,
            Plan::Run {
                program: "/home/a/tool".to_owned(),
                args: Vec::new()
            }
        );
    }

    #[test]
    fn a_program_that_asks_for_a_loader_this_system_has_not_got_is_refused() {
        let plan = plan(
            Kind::Program,
            Path::new("/home/a/tool"),
            &Worlds::default(),
            Some("/lib64/no-such-loader.so"),
        );
        let Plan::Refuse(why) = plan else {
            panic!("a missing loader is a refusal, not a run");
        };
        assert!(why.contains("/lib64/no-such-loader.so"), "{why}");
        assert!(why.contains(CONFIG), "{why}");
    }

    #[test]
    fn a_linux_runtime_runs_the_program_by_its_loader() {
        let worlds = Worlds::parse("linux = /data/linux/lib64/ld.so\n");
        let plan = plan(Kind::Program, Path::new("/home/a/tool"), &worlds, None);
        assert_eq!(
            plan,
            Plan::Run {
                program: "/data/linux/lib64/ld.so".to_owned(),
                args: vec!["/home/a/tool".to_owned()]
            }
        );
    }

    #[test]
    fn a_windows_program_is_run_by_the_runtime_that_is_installed() {
        let worlds = Worlds::parse("windows = /data/proton/files/bin/wine\n");
        let plan = plan(Kind::Windows, Path::new("/home/a/game.exe"), &worlds, None);
        assert_eq!(
            plan,
            Plan::Run {
                program: "/data/proton/files/bin/wine".to_owned(),
                args: vec!["/home/a/game.exe".to_owned()]
            }
        );
    }

    #[test]
    fn a_runtime_named_with_words_runs_them_before_the_file() {
        let worlds = Worlds::parse("windows = /bin/bridgeplay run\n");
        let plan = plan(Kind::Windows, Path::new("/home/a/game.exe"), &worlds, None);
        assert_eq!(
            plan,
            Plan::Run {
                program: "/bin/bridgeplay".to_owned(),
                args: vec!["run".to_owned(), "/home/a/game.exe".to_owned()]
            }
        );
    }

    #[test]
    fn a_world_without_a_runtime_says_so() {
        for (kind, name) in [
            (Kind::Windows, "windows"),
            (Kind::Android, "android"),
            (Kind::MacOs, "macos"),
        ] {
            let Plan::Refuse(why) =
                plan(kind, Path::new("/home/a/thing"), &Worlds::default(), None)
            else {
                panic!("{name} without a runtime is a refusal");
            };
            assert!(why.contains(name), "{why}");
            assert!(why.contains(CONFIG), "{why}");
        }
    }

    #[test]
    fn a_document_is_left_to_the_desktops_own_opener() {
        let plan = plan(
            Kind::Other,
            Path::new("/home/a/notes.txt"),
            &Worlds::default(),
            None,
        );
        assert_eq!(
            plan,
            Plan::Run {
                program: "xdg-open".to_owned(),
                args: vec!["/home/a/notes.txt".to_owned()]
            }
        );
    }

    #[test]
    fn a_package_is_given_to_the_package_manager() {
        let plan = plan(
            Kind::Package,
            Path::new("/home/a/thing.fxpkg"),
            &Worlds::default(),
            None,
        );
        assert_eq!(
            plan,
            Plan::Run {
                program: "/bin/pkg".to_owned(),
                args: vec!["install".to_owned(), "/home/a/thing.fxpkg".to_owned()]
            }
        );
    }
}
