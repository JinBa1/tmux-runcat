mod animation;
mod load;
mod parkour;
mod patrol;
mod pet;
mod tmux;
mod track;
mod watch;

use std::env;
use std::ffi::OsStr;
use std::path::Path;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [command, plugin] if command == "load" => load::run(Path::new(plugin), None)?,
        [command, plugin, socket, pid] if command == "load" => {
            load::run(Path::new(plugin), Some((socket, pid)))?;
        }
        [command, socket, pid, generation] if command == "watch" => {
            watch::run(
                socket,
                pid,
                generation.to_str().ok_or("invalid generation")?,
            )?;
        }
        [command] if command == OsStr::new("--version") => {
            println!("tmux-runcat {}", env!("CARGO_PKG_VERSION"));
        }
        _ => return Err(
            "usage: tmux-runcat load <plugin-dir> [socket pid] | watch <socket> <pid> <generation>"
                .into(),
        ),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("tmux-runcat: {error}");
        std::process::exit(1);
    }
}
