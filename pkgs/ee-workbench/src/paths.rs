use std::ffi::OsString;
use std::path::PathBuf;

pub const APP: &str = "ee-workbench";

/// Escape hatch for tests and for a second workbench on the same machine.
pub const DATA_ENV: &str = "EE_WORKBENCH_DATA";

/// Read by `ee` and by `ee-freecad-server` alike; the two must agree on the
/// socket or the client never finds the session.
pub const CAD_SOCKET_ENV: &str = "EE_WORKBENCH_CAD_SOCKET";

fn non_empty(var: &str) -> Option<OsString> {
    std::env::var_os(var).filter(|value| !value.is_empty())
}

fn home() -> PathBuf {
    non_empty("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match non_empty(var) {
        Some(value) => PathBuf::from(value),
        None => home().join(fallback),
    }
}

/// The Git repository that owns every committed byte of the workbench.
pub fn data_root() -> PathBuf {
    match non_empty(DATA_ENV) {
        Some(value) => PathBuf::from(value),
        None => xdg("XDG_DATA_HOME", ".local/share").join(APP),
    }
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join(APP)
}

/// Disposable derived state only. Nothing here is authoritative.
pub fn cache_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache").join(APP)
}

/// Machine-local, never committed: checkout mappings live here.
pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join(APP)
}

/// XDG_RUNTIME_DIR is the only correct home for the CAD socket, but it is
/// absent under cron, in build sandboxes and over bare ssh; a per-user tmp
/// directory keeps the path deterministic there instead of failing outright.
pub fn runtime_dir() -> PathBuf {
    match non_empty("XDG_RUNTIME_DIR") {
        Some(value) => PathBuf::from(value).join(APP),
        None => {
            let user = non_empty("USER")
                .or_else(|| non_empty("LOGNAME"))
                .unwrap_or_else(|| OsString::from("nobody"));

            let mut name = OsString::from(format!("{APP}-"));
            name.push(&user);

            std::env::temp_dir().join(name)
        }
    }
}

pub fn cad_socket() -> PathBuf {
    match non_empty(CAD_SOCKET_ENV) {
        Some(value) => PathBuf::from(value),
        None => runtime_dir().join("cad.sock"),
    }
}

pub fn checkouts_file() -> PathBuf {
    state_dir().join("checkouts.toml")
}
