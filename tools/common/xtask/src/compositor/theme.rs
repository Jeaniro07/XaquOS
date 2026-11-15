//! The look a watched desktop is drawn in (`docs/DESKTOP.md` §6): Xaqu,
//! the customer's design and the default since 2026-10-24; Ferrix light
//! (§2); or none, hyprix's own defaults.
//!
//! A theme is a configuration file hyprix ships (`hyprix/themes/`), put in
//! the image at `/usr/share/hyprix/themes/` with what it names -- Inter
//! and the Phosphor icons; Xaqu's wallpaper is a gradient the wallpaper
//! client draws, no file -- and `source`d *first*, so
//! that every line after it wins: the keyboard layout and the network
//! `run-compositor` appends, and a person's own `hyprland.conf`. A desktop
//! with no configuration of its own takes the theme's keys too
//! (`xaqu-keys.conf`); one with its own takes the look alone, so that its
//! own binds are the only ones.
//!
//! Only the desktops a person uses are themed -- `run-compositor`, `flash
//! --compositor` and `release-image` set [`Args::theme`] -- never a judged
//! boot, whose pictures stay what they were.

use crate::args::Args;
use crate::ports::{Content, File};
use crate::{Error, Result};

/// Where the themes go in the image.
const THEMES: &str = "usr/share/hyprix/themes";

/// The faces Xaqu draws in, and where: Inter for its text, Phosphor for
/// its icons, each with its licence (`assets/fonts/README.md`).
const FACES: [(&str, &str); 4] = [
    (
        "assets/fonts/inter/InterVariable.ttf",
        "usr/share/fonts/inter/InterVariable.ttf",
    ),
    (
        "assets/fonts/inter/LICENSE",
        "usr/share/fonts/inter/LICENSE",
    ),
    (
        "assets/fonts/phosphor/Phosphor.ttf",
        "usr/share/fonts/phosphor/Phosphor.ttf",
    ),
    (
        "assets/fonts/phosphor/LICENSE",
        "usr/share/fonts/phosphor/LICENSE",
    ),
];

/// The default applications a Xaqu desktop names: Files, shelf, for a
/// folder.
const MIMEAPPS: &str = "[Default Applications]\ninode/directory=shelf.desktop\n";

/// Nocturne for GTK, from hyprix's `themes/gtk`, and where each goes.
const GTK: [(&str, &str); 4] = [
    (
        "gtk/Nocturne/gtk-3.0/gtk.css",
        "usr/share/themes/Nocturne/gtk-3.0/gtk.css",
    ),
    (
        "gtk/Nocturne/gtk-4.0/gtk.css",
        "usr/share/themes/Nocturne/gtk-4.0/gtk.css",
    ),
    ("gtk/Nocturne/settings.ini", "etc/gtk-3.0/settings.ini"),
    ("gtk/Nocturne/settings.ini", "etc/gtk-4.0/settings.ini"),
];

/// Xaqu's dock, the `xaqu-shell` app, which every desktop carries
/// (`desktop::DESKTOP_APPS`).
pub(super) const DOCK: &str = "bin/xaqu-shell";

/// Whether `config` starts a bar of its own: an `exec-once` of waybar or
/// of Xaqu's shell.
fn starts_a_bar(config: &str) -> bool {
    config.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            matches!(key.trim(), "exec-once" | "exec")
                && (value.contains("waybar") || value.contains("xaqu-shell"))
        })
    })
}

/// A desktop's look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Theme {
    /// The customer's design, on Nocturne.
    Xaqu,
    /// GNOME-like and light.
    FerrixLight,
    /// hyprix's own defaults: nothing sourced.
    None,
}

impl Theme {
    /// What `--theme` says, Xaqu when it says nothing.
    ///
    /// # Errors
    ///
    /// A name that is none of them.
    pub(super) fn of(args: &Args) -> Result<Option<Self>> {
        match args.theme.as_deref() {
            None => Ok(None),
            Some("xaqu") => Ok(Some(Self::Xaqu)),
            Some("ferrix-light") => Ok(Some(Self::FerrixLight)),
            Some("none") => Ok(Some(Self::None)),
            Some(other) => Err(Error::new(format!(
                "--theme {other}: xaqu, ferrix-light or none"
            ))),
        }
    }
}

/// Give a desktop a person uses its theme: `--theme`, or Xaqu.
pub(super) fn default_for_a_person(args: &mut Args) {
    if args.theme.is_none() {
        args.theme = Some("xaqu".to_owned());
    }
}

/// Whether the shipped Ferrix wallpaper should give way to the theme's
/// own: under Xaqu, which has one, unless `--wallpaper` named it.
pub(super) fn has_own_wallpaper(args: &Args) -> bool {
    matches!(Theme::of(args), Ok(Some(Theme::Xaqu)))
        && args
            .wallpaper
            .as_deref()
            .is_none_or(|name| name == crate::wallpaper::BRAND && args.release)
}

/// `config` with the theme `args` names sourced before it, and the files
/// it needs added to `files`. A configuration of the person's own
/// (`--config`, or `--everything`'s) takes the theme's look and not its
/// keys.
///
/// # Errors
///
/// An unknown `--theme`, or a theme's file missing from the tree.
pub(super) fn apply(config: String, args: &Args, files: &mut Vec<File>) -> Result<String> {
    let Some(theme) = Theme::of(args)? else {
        return Ok(config);
    };
    let own = args.config.is_some();
    let sourced: &[&str] = match (theme, own) {
        (Theme::None, _) => return Ok(config),
        (Theme::Xaqu, false) => &["xaqu.conf", "xaqu-keys.conf"],
        (Theme::Xaqu, true) => &["xaqu.conf"],
        (Theme::FerrixLight, _) => &["ferrix-light.conf"],
    };
    let root = crate::paths::workspace_root();
    let themes = root.join("src/user/system/linux/compositor/hyprix/themes");
    let mut head = format!(
        "# The desktop's look, `--theme {}` (docs/DESKTOP.md §6): first, so every line \
         after it wins.\n",
        args.theme.as_deref().unwrap_or("xaqu")
    );
    let mut carry = |from: std::path::PathBuf, to: String| -> Result<()> {
        let bytes = std::fs::read(&from)
            .map_err(|error| Error::new(format!("{}: {error}", from.display())))?;
        if !files.iter().any(|file| file.path == to) {
            files.push(File {
                path: to,
                mode: 0o644,
                content: Content::Bytes(bytes),
            });
        }
        Ok(())
    };
    for name in sourced {
        carry(themes.join(name), format!("{THEMES}/{name}"))?;
        head.push_str(&format!("source = /{THEMES}/{name}\n"));
    }
    // Xaqu's dock, which starts its menu bar, unless the configuration
    // starts a bar of its own.
    if theme == Theme::Xaqu && !starts_a_bar(&config) {
        head.push_str(&format!("exec-once = /{DOCK}\n"));
    }
    if theme == Theme::Xaqu {
        // The launcher's look, which Xaqu's keys start fuzzel with
        // (`--config`); SUPER Space opens the shell's Spotlight now.
        carry(
            themes.join("xaqu-fuzzel.ini"),
            format!("{THEMES}/xaqu-fuzzel.ini"),
        )?;
        for (from, to) in FACES {
            carry(root.join(from), to.to_owned())?;
        }
        // GTK in Nocturne: the theme, and GTK's own settings naming it,
        // dark, which portald says too (`docs/DESKTOP.md` §6).
        for (from, to) in GTK {
            carry(themes.join(from), to.to_owned())?;
        }
        // A folder opens in Files, shelf (the customer's choice): GIO's and
        // xdg-open's default for inode/directory.
        files.push(File {
            path: "etc/xdg/mimeapps.list".to_owned(),
            mode: 0o644,
            content: Content::Bytes(MIMEAPPS.as_bytes().to_vec()),
        });
    }
    println!(
        "  theme: {} ({})",
        args.theme.as_deref().unwrap_or("xaqu"),
        if own {
            "its look under this configuration's own lines"
        } else {
            "its look and its keys"
        }
    );
    Ok(format!("{head}{config}"))
}

#[cfg(test)]
mod tests {
    use super::{Theme, apply};
    use crate::args::Args;

    /// Xaqu, on a desktop with no configuration of its own: both files
    /// sourced before everything else, and the faces carried.
    #[test]
    fn xaqu_comes_first_with_its_keys() {
        let args = Args {
            theme: Some("xaqu".to_owned()),
            ..Args::default()
        };
        let mut files = Vec::new();
        let config = apply(
            "bind = SUPER, Q, killactive\n".to_owned(),
            &args,
            &mut files,
        )
        .expect("themed");
        let xaqu = config
            .find("source = /usr/share/hyprix/themes/xaqu.conf")
            .expect("look");
        let keys = config
            .find("source = /usr/share/hyprix/themes/xaqu-keys.conf")
            .expect("keys");
        let own = config.find("bind = SUPER, Q").expect("own line");
        assert!(xaqu < keys && keys < own);
        assert!(config.contains("exec-once = /bin/xaqu-shell"), "the dock");
        // The sidebars left the desktop with A2 (spec §3): their boots start
        // them themselves.
        assert!(!config.contains("--sidebar"), "no sidebars: {config}");
        for path in [
            "usr/share/hyprix/themes/xaqu.conf",
            "usr/share/hyprix/themes/xaqu-fuzzel.ini",
            "usr/share/fonts/phosphor/Phosphor.ttf",
            "usr/share/fonts/phosphor/LICENSE",
            "usr/share/fonts/inter/InterVariable.ttf",
        ] {
            assert!(files.iter().any(|file| file.path == path), "{path}");
        }
    }

    /// A person's own configuration keeps its own keys.
    #[test]
    fn an_own_configuration_takes_the_look_alone() {
        let args = Args {
            theme: Some("xaqu".to_owned()),
            config: Some("/home/u/.config/hypr/hyprland.conf".to_owned()),
            ..Args::default()
        };
        let mut files = Vec::new();
        let config = apply(String::new(), &args, &mut files).expect("themed");
        assert!(config.contains("xaqu.conf"));
        assert!(!config.contains("xaqu-keys.conf"));
        assert!(
            config.contains("xaqu-shell"),
            "no bar of its own: Xaqu's dock"
        );
        let with_waybar =
            apply("exec-once = waybar\n".to_owned(), &args, &mut files).expect("themed");
        assert!(
            !with_waybar.contains("xaqu-shell"),
            "its own bar, not the dock"
        );
    }

    /// No `--theme` is no theme (a judged boot's), `none` the same, and an
    /// unknown one is refused.
    #[test]
    fn unthemed_and_unknown() {
        let mut files = Vec::new();
        let plain = Args::default();
        assert_eq!(
            apply("x\n".to_owned(), &plain, &mut files).expect("same"),
            "x\n"
        );
        let none = Args {
            theme: Some("none".to_owned()),
            ..Args::default()
        };
        assert_eq!(
            apply("x\n".to_owned(), &none, &mut files).expect("same"),
            "x\n"
        );
        assert!(files.is_empty());
        let odd = Args {
            theme: Some("mauve".to_owned()),
            ..Args::default()
        };
        assert!(Theme::of(&odd).is_err());
    }
}
