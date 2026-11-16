//! `bridgeplay`: open Steam and Windows programs on Xaqu.

use bridgeplay_core::{
    Cmd, PREFIX_DISK, Plan, STEAM_SCRIPT, WINE_LOADER, mem_available_mib, missing, parse,
    steam_plan, wine_plan,
};
use std::path::Path;
use std::process::{Command, ExitCode};

const USAGE: &str = "usage: bridgeplay steam | run PROGRAM.exe [ARGS] | doctor | --version";

fn start(plan: &Plan) -> ExitCode {
    let mut c = Command::new(&plan.program);
    c.args(&plan.args).envs(plan.env.iter().map(|(k, v)| (k, v)));
    match c.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => {
            eprintln!("bridgeplay: {} left with {s}", plan.program);
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("bridgeplay: cannot start {}: {e}", plan.program);
            ExitCode::FAILURE
        }
    }
}

fn doctor() -> ExitCode {
    println!("bridgeplay doctor");
    let row = |name: &str, path: &str| {
        let s = if Path::new(path).exists() { "present" } else { "missing" };
        println!("  {name:<8} {s:<8} {path}");
    };
    row("steam", STEAM_SCRIPT);
    row("wine", WINE_LOADER);
    row("data", "/data");
    let free = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|m| mem_available_mib(&m));
    match free {
        Some(m) => println!("  memory   {m} MiB available"),
        None => println!("  memory   unknown"),
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Cmd::Version => {
            println!("bridgeplay {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Cmd::Usage(why) => {
            if let Some(w) = &why {
                eprintln!("bridgeplay: {w}");
            }
            println!("{USAGE}");
            if why.is_some() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Cmd::Doctor => doctor(),
        Cmd::Steam => {
            if !Path::new(STEAM_SCRIPT).exists() {
                let how = "Build the volume with tools/common/fetch/fetch-steam-window.sh.";
                eprintln!("{}", missing("Steam", STEAM_SCRIPT, how));
                return ExitCode::FAILURE;
            }
            start(&steam_plan())
        }
        Cmd::Run(prog) => {
            if !Path::new(WINE_LOADER).exists() {
                let how = "Build the volume with tools/common/fetch/fetch-wine.sh.";
                eprintln!("{}", missing("Wine", WINE_LOADER, how));
                return ExitCode::FAILURE;
            }
            let has_data = Path::new("/data").is_dir();
            if has_data {
                // Best effort: Wine makes the prefix itself, but not its parents.
                let _ = std::fs::create_dir_all(PREFIX_DISK);
            }
            start(&wine_plan(&prog, has_data))
        }
    }
}
