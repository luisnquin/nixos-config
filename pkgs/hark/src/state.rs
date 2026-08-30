use crate::format::now_ms;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// The moment the centre was last opened. Everything newer is unread, which is
/// what the envelope counts. Runtime state on purpose: mako's history does not
/// outlive the session either, so a stale marker from a previous boot would
/// only ever be wrong.
pub fn marker() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)?;
    Some(base.join("hark/seen"))
}

pub fn seen_at() -> u64 {
    marker()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| raw.trim().parse().ok())
        .unwrap_or(0)
}

pub fn mark_seen() -> Result<(), Box<dyn Error>> {
    let Some(path) = marker() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, now_ms().to_string())?;
    Ok(())
}
