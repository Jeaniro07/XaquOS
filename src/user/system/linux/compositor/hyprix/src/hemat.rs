//! Hemat: the desktop's memory saving, one cgroup for each app.
//!
//! A desktop running many apps holds the memory of all of them at once,
//! while most of them are looked at now and then. Hemat is the part of the
//! desktop that lets the ones in the background have less: each app gets a
//! cgroup of its own, an app in the background is given less of the
//! processor, and an app that has been in the background for five minutes
//! is put to sleep, which is `memory.reclaim` -- the clean pages it holds
//! are given back.
//!
//! # The tree
//!
//! Hemat's groups live in a tree of their own, `/sys/fs/cgroup/hemat`,
//! rather than below the session's cgroup: the session's cgroup holds the
//! compositor itself, and cgroup v2 does not let a cgroup that holds
//! processes have controllers enabled for its children
//! (`docs/CGROUPS.md` §3.1). The tree is made here, and mounted if it is
//! not mounted yet, because the desktop's boot has no init to mount it
//! (`docs/AUTH.md` §1.2).
//!
//! # Failing
//!
//! Nothing here may take the desktop with it. A mount that is refused, a
//! cgroup that cannot be made, a file that cannot be written: each is
//! reported in a sentence and left behind, and the app it was for keeps
//! running as it was (`docs/DESKTOP.md` §7).

use std::ffi::CString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// Where the cgroup v2 tree is mounted.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// The tree the hemat's groups live in, of its own.
pub const TREE: &str = "hemat";

/// The controllers the root of the tree has: the three Ferrix has.
pub const CONTROLLERS: [&str; 3] = ["+cpu", "+memory", "+pids"];

/// The controllers the hemat's tree has for its own children.
pub const TREE_CONTROLLERS: [&str; 2] = ["+cpu", "+memory"];

/// The hemat's tree.
#[must_use]
pub fn tree() -> PathBuf {
    Path::new(CGROUP_ROOT).join(TREE)
}

/// The cgroup of the app called `name`, the `number`th one of its class.
#[must_use]
pub fn app_dir(name: &str, number: u32) -> PathBuf {
    tree().join(format!("app-{name}-{number}"))
}

/// A class name made safe for a cgroup's name.
///
/// A cgroup's name takes letters, digits and `:_.-`; anything else is
/// `_`, as in `crate::scope::name`. An empty name is `program`, so that
/// there is always something to make a directory of.
#[must_use]
pub fn safe(class: &str) -> String {
    let base: String = class
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    if base.is_empty() {
        "program".to_owned()
    } else {
        base
    }
}

/// Write `value` to the file `file` of the cgroup `dir`.
///
/// # Errors
///
/// When the file cannot be written, with a sentence saying which file.
pub fn write(dir: &Path, file: &str, value: &str) -> Result<(), String> {
    write_with(dir, file, value, |path, text| {
        let mut handle = fs::OpenOptions::new().write(true).open(path)?;
        handle.write_all(text.as_bytes())
    })
}

/// Write `value` to the file `file` of the cgroup `dir`, with `put`.
///
/// The write itself is `put`, so that a test can hand in one that fails and
/// see that the failure is a sentence and not a panic.
///
/// # Errors
///
/// When `put` fails, with a sentence saying which file.
pub fn write_with<F>(dir: &Path, file: &str, value: &str, put: F) -> Result<(), String>
where
    F: FnOnce(&Path, &str) -> io::Result<()>,
{
    let at = dir.join(file);
    put(&at, value).map_err(|error| format!("{}: {error}", at.display()))
}

/// Mount the cgroup v2 tree if it is not mounted, and make the hemat's.
///
/// # Errors
///
/// When the tree cannot be mounted, or its controllers cannot be enabled.
pub fn prepare() -> Result<(), String> {
    let root = Path::new(CGROUP_ROOT);
    if !root.join("cgroup.controllers").exists() {
        mount(root)?;
    }
    write(root, "cgroup.subtree_control", &CONTROLLERS.join(" "))?;
    let tree = tree();
    fs::create_dir_all(&tree).map_err(|error| format!("{}: {error}", tree.display()))?;
    write(
        &tree,
        "cgroup.subtree_control",
        &TREE_CONTROLLERS.join(" "),
    )?;
    Ok(())
}

/// Mount cgroup v2 at the root.
///
/// # Errors
///
/// When the mount is refused for any reason but the tree being there
/// already.
fn mount(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let source = CString::new("cgroup2").map_err(|_| "cgroup2 is not a name".to_owned())?;
    let target = CString::new(CGROUP_ROOT).map_err(|_| "the mount point is not a name".to_owned())?;
    // SAFETY: both names are NUL-terminated C strings that live for the
    // call, and the mount takes no data.
    let code = unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            source.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if code == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    // The tree being mounted already is not a failure: the desktop's boot
    // has an init that mounts it (`src/user/system/linux/init`).
    if error.raw_os_error() == Some(libc::EBUSY) {
        return Ok(());
    }
    Err(format!("mounting cgroup2: {error}"))
}

/// Make the cgroup of the app called `name`, the `number`th of its class.
///
/// The group is made under the tree at `root`, which is the mount in a
/// desktop and a directory in a test.
///
/// # Errors
///
/// When the directory cannot be made.
pub fn make_app_in(root: &Path, name: &str, number: u32) -> Result<PathBuf, String> {
    let dir = root.join(TREE).join(format!("app-{name}-{number}"));
    fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    Ok(dir)
}

/// Make the cgroup of the app called `name`, the `number`th of its class.
///
/// # Errors
///
/// When the directory cannot be made.
pub fn make_app(name: &str, number: u32) -> Result<PathBuf, String> {
    make_app_in(Path::new(CGROUP_ROOT), name, number)
}

/// Put the process `pid` in the cgroup `dir`.
///
/// # Errors
///
/// When the process cannot be moved there.
pub fn put_pid(dir: &Path, pid: u32) -> Result<(), String> {
    write(dir, "cgroup.procs", &pid.to_string())
}

/// Remove the cgroup `dir`.
///
/// A cgroup that still holds a process cannot be removed, and that is not
/// an error: the group stays until the process in it has gone.
///
/// # Errors
///
/// When the directory cannot be removed for any other reason.
pub fn remove_app(dir: &Path) -> Result<(), String> {
    match fs::remove_dir(dir) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(format!("{}: {error}", dir.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hemat_tree_is_a_slice_of_its_own() {
        assert_eq!(tree(), Path::new("/sys/fs/cgroup/hemat"));
        assert_eq!(
            app_dir("foot", 3),
            Path::new("/sys/fs/cgroup/hemat/app-foot-3")
        );
    }

    #[test]
    fn a_class_name_is_made_safe_for_a_cgroup() {
        assert_eq!(safe("org.example.Notes"), "org.example.Notes");
        assert_eq!(safe("my prog+x"), "my_prog_x");
        assert_eq!(safe(""), "program");
    }

    #[test]
    fn the_controllers_are_the_three_ferrix_has() {
        assert_eq!(CONTROLLERS, ["+cpu", "+memory", "+pids"]);
    }

    #[test]
    fn a_write_that_fails_leaves_the_app_alone() {
        let failed = write_with(Path::new("/nowhere"), "cpu.weight", "10", |_, _| {
            Err(io::Error::other("no such file"))
        });
        let why = failed.expect_err("a write that fails says so");
        assert!(why.contains("cpu.weight"), "{why}");
        assert!(why.contains("no such file"), "{why}");
    }

    #[test]
    fn an_app_gets_a_group_of_its_own() {
        let root = std::env::temp_dir().join(format!("hemat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = make_app_in(&root, "foot", 1).expect("the group's made");
        assert_eq!(dir, root.join("hemat").join("app-foot-1"));
        assert!(dir.is_dir());
        let _ = fs::remove_dir_all(&root);
    }
}
