//! Which surface the computer has right now: the active session on seat0.

use std::collections::HashMap;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::process::Command;

pub const LOGINCTL: &str = "@loginctl@";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Graphical,
    Console,
}

fn loginctl(args: &[&str]) -> HashMap<String, String> {
    let Ok(output) = Command::new(LOGINCTL).args(args).output() else {
        return HashMap::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// `Graphical` when the seat is showing this user's own wayland or x11
/// session, `Console` for everything else: a tty login, the greeter, nobody.
pub fn active_session_kind(user: &str) -> Kind {
    let seat = loginctl(&["show-seat", "seat0", "-p", "ActiveSession"]);
    let Some(session) = seat.get("ActiveSession").filter(|id| !id.is_empty()) else {
        return Kind::Console;
    };
    let props = loginctl(&["show-session", session, "-p", "Type", "-p", "Class", "-p", "Name"]);
    let graphical = matches!(props.get("Type").map(String::as_str), Some("wayland" | "x11"));
    let owned = props.get("Class").map(String::as_str) == Some("user") && props.get("Name").map(String::as_str) == Some(user);
    if graphical && owned {
        Kind::Graphical
    } else {
        Kind::Console
    }
}

/// The compositor's socket name, from the runtime dir rather than from the
/// environment: gpg-agent's is the user manager's, which the session may not
/// have updated.
pub fn wayland_display(runtime: &Path) -> Option<String> {
    let mut sockets: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(runtime)
        .ok()?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("wayland-") && !name.ends_with(".lock")
        })
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_socket()))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.file_name().to_string_lossy().into_owned()))
        })
        .collect();
    if sockets.is_empty() {
        return std::env::var("WAYLAND_DISPLAY").ok();
    }
    sockets.sort();
    sockets.pop().map(|(_, name)| name)
}
