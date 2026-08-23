use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Context, Result};

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

/// A path as the caller meant it, from wherever they typed it. The CAD server
/// is a daemon whose working directory is whatever the first `ee` happened to
/// be started from, so a relative path has to be resolved here or the file
/// lands somewhere nobody asked for. `std::path::absolute` and not
/// `canonicalize`: save, export and render all name files that do not exist yet.
pub fn absolute(input: &str) -> Result<PathBuf> {
    let expanded = match input.strip_prefix('~') {
        Some("") => home(),
        Some(rest) => match rest.strip_prefix('/') {
            Some(rest) => home().join(rest),
            // `~user` is a shell feature this does not implement, and guessing
            // at another account's home would be worse than leaving it alone.
            None => PathBuf::from(input),
        },
        None => PathBuf::from(input),
    };

    std::path::absolute(&expanded).with_context(|| format!("resolving {}", expanded.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_resolved_against_the_client() {
        unsafe { std::env::set_var("HOME", "/home/tester") };

        assert_eq!(absolute("~/out.png").unwrap(), PathBuf::from("/home/tester/out.png"));
        assert_eq!(absolute("~").unwrap(), PathBuf::from("/home/tester"));
        assert_eq!(absolute("/tmp/out.png").unwrap(), PathBuf::from("/tmp/out.png"));
        assert!(absolute("~other/out.png").unwrap().is_absolute());

        let here = std::env::current_dir().unwrap();
        assert_eq!(absolute("out.png").unwrap(), here.join("out.png"));
        assert_eq!(absolute("./sub/out.png").unwrap(), here.join("sub/out.png"));
    }
}
