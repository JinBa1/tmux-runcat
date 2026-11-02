use crate::animation::SLEEP_POSE;
use crate::tmux::{Tmux, shell_quote};
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn context(explicit: Option<(&OsStr, &OsStr)>) -> io::Result<(OsString, OsString)> {
    if let Some((socket, pid)) = explicit {
        return Ok((socket.to_owned(), pid.to_owned()));
    }
    if let Ok(value) = env::var("TMUX") {
        let mut parts = value.rsplitn(3, ',');
        if let (Some(_), Some(pid), Some(socket)) = (parts.next(), parts.next(), parts.next()) {
            return Ok((socket.into(), pid.into()));
        }
    }
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{socket_path}\t#{pid}"])
        .output()?;
    let value = String::from_utf8_lossy(&output.stdout);
    let (socket, pid) = value
        .trim()
        .split_once('\t')
        .ok_or_else(|| io::Error::other("could not find tmux server"))?;
    Ok((socket.into(), pid.into()))
}

pub fn run(plugin: &Path, explicit: Option<(&OsStr, &OsStr)>) -> io::Result<()> {
    if !plugin.is_dir() {
        return Err(io::Error::other("plugin directory does not exist"));
    }
    let (socket, pid) = context(explicit)?;
    let tmux = Tmux::new(&socket);
    let existing = tmux.output(["list-keys", "-T", "root", "Any"])?;
    if existing.status.success()
        && !existing.stdout.is_empty()
        && !String::from_utf8_lossy(&existing.stdout).contains("@runcat-keys")
    {
        return Err(io::Error::other(
            "root Any is already bound; remove that binding before loading tmux-runcat",
        ));
    }
    for (name, value) in [
        ("@runcat-frame", SLEEP_POSE),
        ("@runcat-state", "sleep"),
        ("@runcat-keys", "0"),
        ("@runcat-idle", "2"),
        ("@runcat-min-fps", "3"),
        ("@runcat-max-fps", "0"),
        ("@runcat-size", "medium"),
        ("@runcat-skin", "classic"),
        ("@runcat-spin-chance", "0"),
        ("@runcat-roam", "off"),
        ("@runcat-color", "inherit"),
    ] {
        if !tmux.output(["show-options", "-gv", name])?.status.success() {
            tmux.execute(["set-option", "-g", name, value])?;
        }
    }
    tmux.source("bind-key -T root Any if-shell -F '#{!=:#{mouse_x},}' {\n    send-keys -M\n} {\n    set-option -F @runcat-keys '#{e|+:#{@runcat-keys},1}'\n    send-keys\n}\n")?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let generation = format!("{}-{nanos}", std::process::id());
    tmux.execute(["set-option", "-g", "@runcat-generation", &generation])?;
    let executable = env::current_exe()?;
    let arguments = [
        executable.as_os_str(),
        OsStr::new("watch"),
        &socket,
        &pid,
        OsStr::new(&generation),
    ];
    let mut worker = OsString::new();
    for argument in arguments {
        if !worker.is_empty() {
            worker.push(" ");
        }
        worker.push(shell_quote(argument));
    }
    tmux.execute([OsStr::new("run-shell"), OsStr::new("-b"), &worker])
}
