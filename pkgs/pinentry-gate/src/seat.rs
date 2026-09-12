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
/// have updated. Only a socket libwayland locked beside itself counts: other
/// daemons park their own `wayland-*` sockets in the same directory, and a
/// terminal told to speak wayland at one of those kills it.
pub fn wayland_display(runtime: &Path) -> Option<String> {
    let mut sockets: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(runtime)
        .ok()?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("wayland-") && !name.ends_with(".lock") && runtime.join(format!("{name}.lock")).is_file()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_daemons_own_socket_is_not_the_display() {
        let dir = std::env::temp_dir().join(format!("pinentry-gate-seat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _compositor = UnixListener::bind(dir.join("wayland-1")).unwrap();
        std::fs::write(dir.join("wayland-1.lock"), "").unwrap();
        let _wallpaper = UnixListener::bind(dir.join("wayland-1-awww-daemon.sock")).unwrap();
        assert_eq!(wayland_display(&dir), Some("wayland-1".to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
