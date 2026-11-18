use std::collections::HashMap;
use std::error::Error;
use std::os::fd::AsRawFd;
use std::process::Command;
use std::{env, fs, process, thread};

use nix::fcntl::AT_FDCWD;
use nix::sys::fanotify::{
    EventFFlags, Fanotify, FanotifyResponse, InitFlags, MarkFlags, MaskFlags, Response,
};
use nix::sys::inotify::{AddWatchFlags, InitFlags as InotifyFlags, Inotify};
use nix::sys::signal::{kill, killpg, Signal};
use nix::unistd::{getpgid, getpgrp, Pid};

const MAX_ANCESTORS: usize = 8;
const MAX_CMDLINE: usize = 120;

fn ppid(status: &str) -> Option<i32> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("PPid:"))
        .and_then(|value| value.trim().parse().ok())
}

fn describe(pid: i32) -> String {
    let exe = fs::read_link(format!("/proc/{pid}/exe"))
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let cmdline = fs::read(format!("/proc/{pid}/cmdline"))
        .map(|raw| String::from_utf8_lossy(&raw).replace('\0', " "))
        .unwrap_or_default();
    let cmdline: String = cmdline.trim().chars().take(MAX_CMDLINE).collect();
    format!("{pid} {exe} [{cmdline}]")
}

fn ancestry(mut pid: i32) -> String {
    let mut chain = Vec::new();
    while pid > 1 && chain.len() < MAX_ANCESTORS {
        chain.push(describe(pid));
        pid = fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|status| ppid(&status))
            .unwrap_or(0);
    }
    chain.join(" <- ")
}

fn freeze(pid: i32) -> Option<String> {
    // pid 0 is an opener we cannot see; kill(0) would hit our own group
    if pid <= 1 || pid == process::id() as i32 {
        return None;
    }
    let opener = Pid::from_raw(pid);
    match getpgid(Some(opener)) {
        Ok(group) if group.as_raw() > 1 && group != getpgrp() => {
            killpg(group, Signal::SIGSTOP).ok()?;
            Some(format!("-{group}"))
        }
        _ => {
            kill(opener, Signal::SIGSTOP).ok()?;
            Some(pid.to_string())
        }
    }
}

fn alert(hook: Option<&str>, event: &str, path: &str, frozen: &str, chain: &str) {
    let Some(hook) = hook else { return };
    let status = Command::new(hook)
        .env("TRIPWIRE_EVENT", event)
        .env("TRIPWIRE_PATH", path)
        .env("TRIPWIRE_FROZEN", frozen)
        .env("TRIPWIRE_CHAIN", chain)
        .status();
    if let Err(err) = status {
        eprintln!("hook: {err}");
    }
}

fn watch_removal(decoys: &[String], hook: Option<String>) -> Result<(), Box<dyn Error>> {
    let inotify = Inotify::init(InotifyFlags::IN_CLOEXEC)?;
    let mut watched = HashMap::new();
    for decoy in decoys {
        let flags =
            AddWatchFlags::IN_DELETE_SELF | AddWatchFlags::IN_MOVE_SELF | AddWatchFlags::IN_ATTRIB;
        watched.insert(inotify.add_watch(decoy.as_str(), flags)?, decoy.clone());
    }
    thread::spawn(move || {
        let path = inotify
            .read_events()
            .ok()
            .and_then(|events| {
                events
                    .first()
                    .and_then(|event| watched.get(&event.wd).cloned())
            })
            .unwrap_or_default();
        eprintln!("{path} removed or altered; restarting");
        alert(hook.as_deref(), "removed", &path, "", "");
        process::exit(1);
    });
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let hook = (args.first().map(String::as_str) == Some("--hook") && args.len() > 1).then(|| {
        args.remove(0);
        args.remove(0)
    });
    if args.is_empty() {
        return Err("usage: tripwire [--hook COMMAND] DECOY...".into());
    }

    let fan = Fanotify::init(
        InitFlags::FAN_CLASS_CONTENT | InitFlags::FAN_CLOEXEC,
        EventFFlags::O_RDONLY | EventFFlags::O_CLOEXEC,
    )?;
    for decoy in &args {
        fan.mark(
            MarkFlags::FAN_MARK_ADD,
            MaskFlags::FAN_OPEN_PERM,
            AT_FDCWD,
            Some(decoy.as_str()),
        )
        .map_err(|err| format!("{decoy}: {err}"))?;
    }
    watch_removal(&args, hook.clone())?;
    eprintln!("watching {} decoys", args.len());

    loop {
        for event in fan.read_events()? {
            let Some(fd) = event.fd() else { continue };
            let pid = event.pid();
            let frozen = freeze(pid).unwrap_or_default();
            fan.write_response(FanotifyResponse::new(fd, Response::FAN_DENY))?;

            let path = fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd()))
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let chain = ancestry(pid);
            eprintln!("{path} opened by {chain}; denied, frozen={frozen}");

            let hook = hook.clone();
            thread::spawn(move || alert(hook.as_deref(), "opened", &path, &frozen, &chain));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ppid;

    #[test]
    fn ppid_reads_the_parent_from_proc_status() {
        assert_eq!(
            ppid("Name:\tcat\nPid:\t42\nPPid:\t7\nUid:\t1000\n"),
            Some(7)
        );
        assert_eq!(ppid("Name:\tcat\n"), None);
    }
}
