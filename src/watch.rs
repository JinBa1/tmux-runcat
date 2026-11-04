use crate::animation::{Animation, Settings};
use crate::patrol::Patrol;
use crate::tmux::Tmux;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(50);
const FORMAT: &str = "#{client_name}\t#{session_id}\t#{@runcat-keys}\t#{@runcat-idle}\t#{@runcat-min-fps}\t#{@runcat-max-fps}\t#{@runcat-size}\t#{@runcat-skin}\t#{@runcat-spin-chance}\t#{@runcat-wall-chance}\t#{@runcat-wall-debug}";

struct Session {
    count: u64,
    settings: Settings,
    clients: Vec<Client>,
    color: String,
}

struct Client {
    pid: u32,
    room: i32,
}

#[derive(Default)]
struct View {
    session: String,
    patrol: Patrol,
    published: Option<String>,
}

fn parse_clients(lines: impl Iterator<Item = String>) -> HashMap<String, Session> {
    let mut sessions: HashMap<String, Session> = HashMap::new();
    for line in lines {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 14 {
            continue;
        }
        let Ok(pid) = fields[11].parse::<u32>() else {
            continue;
        };
        let session = sessions
            .entry(fields[1].to_owned())
            .or_insert_with(|| Session {
                count: fields[2].parse().unwrap_or(0),
                settings: Settings::parse(fields[3], fields[4], fields[5], fields[6], fields[7])
                    .with_spin_chance(fields[8])
                    .with_wall_chance(fields[9])
                    .with_wall_debug(fields[10]),
                clients: Vec::new(),
                color: fields[13].to_owned(),
            });
        if !session.clients.iter().any(|client| client.pid == pid) {
            session.clients.push(Client {
                pid,
                room: fields[12].parse().unwrap_or(-1),
            });
        }
    }
    sessions
}

fn publish_option(commands: &mut Vec<String>, session: &str, name: &str, value: String) {
    if !commands.is_empty() {
        commands.push(";".to_owned());
    }
    commands.extend([
        "set-option".to_owned(),
        "-t".to_owned(),
        session.to_owned(),
        name.to_owned(),
        value,
    ]);
}

pub fn run(socket: &OsStr, pid: &OsStr, generation: &str) -> io::Result<()> {
    let pid = pid
        .to_str()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value != 0)
        .ok_or_else(|| io::Error::other("invalid server PID"))?;
    let directory = Path::new(socket)
        .parent()
        .ok_or_else(|| io::Error::other("socket has no directory"))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .append(true)
        .open(directory.join(format!("runcat-{pid}.lock")))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => thread::sleep(POLL),
            Err(TryLockError::WouldBlock) => return Ok(()),
            Err(TryLockError::Error(error)) => return Err(error),
        }
    }
    let tmux = Tmux::new(socket);
    let start = Instant::now();
    let mut animations: HashMap<String, Animation> = HashMap::new();
    let mut published: HashMap<String, (String, &'static str)> = HashMap::new();
    let mut views: HashMap<u32, View> = HashMap::new();
    let mut synced = None;
    let list_clients = format!(
        // L: builds a format tree owned by each attached client. This reuses
        // its status-command cache; a normal list-clients tree belongs to the
        // short-lived querying process and loses asynchronous job results.
        "list-clients -F '#{{L:#{{?#{{==:#{{session_name}},#{{client_session}}}},{FORMAT}\t#{{client_pid}}\t{}\t#{{@runcat-color}}\n,}}}}'",
        crate::track::room_format()
    );
    loop {
        let output = tmux.output([
            // A fresh server may still be processing its startup config and
            // have no session yet. Query the global option without a target.
            "show-options",
            "-gv",
            "@runcat-generation",
            ";",
            "show-options",
            "-gqv",
            "status-format[0]",
            ";",
            "show-options",
            "-gqv",
            "@runcat-roam",
            ";",
            // list-clients also requires a session, even without -t. The
            // format-only conditional can safely run before the first one.
            "if-shell",
            "-F",
            "#{session_id}",
            &list_clients,
        ])?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        if !output.status.success() || lines.next() != Some(generation) {
            return Ok(());
        }
        let status_format = lines.next().unwrap_or("");
        let roaming = matches!(lines.next(), Some("on" | "1"));
        let sessions = parse_clients(lines.map(str::to_owned));
        let clients = sessions
            .values()
            .flat_map(|session| session.clients.iter().map(|client| client.pid))
            .collect();
        let client_format = crate::track::client_format(clients);
        let track_state = (status_format.to_owned(), roaming, client_format.clone());
        if synced.as_ref() != Some(&track_state) {
            crate::track::sync(&tmux, status_format, roaming, &client_format)?;
            synced = Some(track_state);
        }
        animations.retain(|session, _| sessions.contains_key(session));
        published.retain(|session, _| sessions.contains_key(session));
        // Remove client options after detach/session switches rather than
        // leaving stale lanes in a long-running server.
        let mut stale = Vec::new();
        views.retain(|pid, view| {
            let attached = roaming
                && sessions
                    .get(&view.session)
                    .is_some_and(|session| session.clients.iter().any(|client| client.pid == *pid));
            if !attached {
                stale.push((view.session.clone(), *pid));
            }
            attached
        });
        for (session, pid) in stale {
            let _ = tmux.output([
                "set-option",
                "-qu",
                "-t",
                &session,
                &format!("@runcat-client-{pid}"),
            ]);
        }
        if sessions.is_empty() {
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        let now = start.elapsed().as_secs_f64();
        let mut next_update = now + POLL.as_secs_f64();
        let mut update = Vec::new();
        for (session, mut activity) in sessions {
            let animation = animations
                .entry(session.clone())
                .or_insert_with(|| Animation::new(activity.count, now));
            let pose = animation
                .update(activity.count, now, activity.settings)
                .to_owned();
            next_update = next_update.min(animation.next_update());
            let snapshot = (pose.clone(), animation.state());
            let previous = published.get(&session);
            if previous.is_none_or(|last| last.0 != pose) {
                publish_option(&mut update, &session, "@runcat-frame", pose);
            }
            let mut display_state = animation.state();
            if roaming {
                activity.clients.sort_by_key(|client| client.pid);
                for client in activity.clients {
                    let view = views.entry(client.pid).or_insert_with(|| View {
                        session: session.clone(),
                        ..View::default()
                    });
                    let frame = view.patrol.update(
                        animation.travel(),
                        client.room,
                        now,
                        animation.state(),
                        activity.settings,
                        animation.pressed(),
                    );
                    if let Some(state) = frame.state {
                        display_state = state;
                    }
                    let pose = frame.pose.unwrap_or_else(|| animation.facing(frame.left));
                    let lane =
                        crate::track::render(client.room, frame.position, &pose, &activity.color);
                    if view.published.as_ref() != Some(&lane) {
                        publish_option(
                            &mut update,
                            &session,
                            &format!("@runcat-client-{}", client.pid),
                            lane.clone(),
                        );
                        view.published = Some(lane);
                    }
                }
            }
            if previous.is_none_or(|last| last.1 != display_state) {
                publish_option(
                    &mut update,
                    &session,
                    "@runcat-state",
                    display_state.to_owned(),
                );
            }
            published.insert(session, (snapshot.0, display_state));
        }
        if !update.is_empty() && !tmux.output(update)?.status.success() {
            published.clear();
            for view in views.values_mut() {
                view.published = None;
            }
        }
        // Poll input at least every 50 ms, waking earlier for fast frames.
        // Time spent querying/redrawing tmux counts toward the deadline.
        let delay = next_update - start.elapsed().as_secs_f64();
        if delay > 0.0 {
            thread::sleep(Duration::from_secs_f64(delay));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clients_are_grouped_by_session_with_independent_counts() {
        let sessions = parse_clients(
            [
                "/dev/pts/1\t$1\t4\t2\t3\t12\tmedium\tclassic\t0\t35\toff\t101\t80\tinherit",
                "/dev/pts/2\t$1\t4\t2\t3\t12\tmedium\tclassic\t0\t35\toff\t102\t40\tinherit",
                "/dev/pts/3\t$2\t9\t1\t4\t8\tlarge\tpet-cats\t15\t75\ton\t103\t90\tcyan",
                "invalid",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions["$1"].clients.len(), 2);
        assert_eq!(sessions["$1"].clients[1].room, 40);
        assert_eq!(sessions["$2"].color, "cyan");
        assert_eq!(sessions["$1"].count, 4);
        assert_eq!(sessions["$2"].count, 9);
        assert_eq!(sessions["$2"].settings.idle, 1.0);
        assert_eq!(
            sessions["$2"].settings.skin,
            crate::animation::Skin::PetCats
        );
        assert_eq!(
            sessions["$2"].settings.size,
            crate::animation::CatSize::Large
        );
        assert_eq!(sessions["$2"].settings.wall_chance, 75.0);
        assert!(sessions["$2"].settings.wall_debug);
    }
}
