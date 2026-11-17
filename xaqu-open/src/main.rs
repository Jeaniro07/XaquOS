//! `xaqu-open`: opens a file in the world that owns it.
//!
//! The desktop hands a file here -- from Files, from the launcher, from a
//! terminal -- and this program says which world the file belongs to and
//! runs it by the runtime that world is run by. A Linux program runs by
//! itself; a Windows one needs Wine, an Android one an Android runtime, a
//! macOS one a macOS runtime, and each of those is the user's to install
//! and to name in `/etc/xaqu/app-worlds.conf`. A document is left to the
//! desktop's own opener, `xdg-open`.
//!
//! ```text
//! xaqu-open [--worlds] [--config FILE] TARGET
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use xaqu_open::{CONFIG, Kind, Plan, Worlds, classify, plan};

/// How much of a file is read to know what it is.
const HEAD: usize = 64;

/// The directories a program is looked for in, when its name is not a path.
const PATHS: [&str; 4] = ["/bin", "/usr/bin", "/usr/local/bin", "/data/usr/bin"];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut config = None;
    let mut list = false;
    let mut target = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--worlds" => list = true,
            "--config" => config = arguments.next(),
            "--help" | "-h" => {
                println!("usage: xaqu-open [--worlds] [--config FILE] TARGET");
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("xaqu-open: {other}: no such option");
                std::process::exit(2);
            }
            other => target = Some(other.to_owned()),
        }
    }
    let config = config
        .unwrap_or_else(|| std::env::var("XAQU_APP_WORLDS").unwrap_or_else(|_| CONFIG.to_owned()));
    let worlds = Worlds::parse(&std::fs::read_to_string(&config).unwrap_or_default());
    if list {
        for world in xaqu_open::World::ALL
            .iter()
            .copied()
            .filter(|world| *world != xaqu_open::World::Other)
        {
            let runtime = worlds.runtime(world).unwrap_or("none");
            println!("{}: {runtime}", world.name());
        }
        return;
    }
    let Some(target) = target else {
        eprintln!("usage: xaqu-open [--worlds] [--config FILE] TARGET");
        std::process::exit(2);
    };
    match open(&target, &worlds) {
        Ok(status) => std::process::exit(status),
        Err(why) => {
            eprintln!("xaqu-open: {why}");
            std::process::exit(1);
        }
    }
}

/// Open `target`, and say what came of it: the status the program ended
/// with, or the reason it was not run at all.
fn open(target: &str, worlds: &Worlds) -> Result<i32, String> {
    let path = path_of(target);
    let kind = kind_of(&path)?;
    let interp = interpreter_of(&path, kind)?;
    let plan = plan(kind, &path, worlds, interp.as_deref());
    let (program, args) = match plan {
        Plan::Run { program, args } => (program, args),
        Plan::Refuse(why) => return Err(why),
    };
    let program = resolve(&program)?;
    // The program is started and left running: the desktop that asked for
    // it does not wait for an application to end, and a terminal that ran
    // it sees what it prints.
    Command::new(&program)
        .args(&args)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| format!("{}: {error}", program.display()))?;
    Ok(0)
}

/// The path `target` names: a file, or the path a `file://` URL names.
fn path_of(target: &str) -> PathBuf {
    match target.strip_prefix("file://") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(target),
    }
}

/// What `path` is, by what it holds. A page is a document: the browser's.
fn kind_of(path: &Path) -> Result<Kind, String> {
    if path.exists() {
        let head = head_of(path)?;
        return Ok(classify(path, &head));
    }
    if path.to_string_lossy().contains("://") {
        return Ok(Kind::Other);
    }
    Err(format!("{}: there is no such file", path.display()))
}

/// The first [`HEAD`] bytes of `path`.
fn head_of(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut head = vec![0; HEAD];
    let read = file
        .read(&mut head)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    head.truncate(read);
    Ok(head)
}

/// The loader `path` asks for, when it is a program that asks for one.
fn interpreter_of(path: &Path, kind: Kind) -> Result<Option<String>, String> {
    if kind != Kind::Program {
        return Ok(None);
    }
    let image = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let Ok(elf) = xaqu_elf::Elf::parse(&image) else {
        return Ok(None);
    };
    Ok(match elf.interpreter() {
        Some(Ok(interpreter)) => Some(
            String::from_utf8_lossy(interpreter)
                .trim_end_matches('\0')
                .to_owned(),
        ),
        _ => None,
    })
}

/// Where the program is: the path it names, or the first of [`PATHS`] that
/// holds a program by that name.
fn resolve(program: &str) -> Result<PathBuf, String> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return if path.exists() {
            Ok(path.to_owned())
        } else {
            Err(format!("{program}: there is no such program"))
        };
    }
    for directory in PATHS {
        let candidate = Path::new(directory).join(program);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!("{program}: there is no such program"))
}
