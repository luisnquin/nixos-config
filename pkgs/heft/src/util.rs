use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

use crate::model::{Bytes, FsStat};

pub fn human(bytes: Bytes) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{}B", bytes)
    } else if value >= 100.0 {
        format!("{:.0}{}", value, UNITS[unit])
    } else {
        format!("{:.1}{}", value, UNITS[unit])
    }
}

pub fn human_delta(delta: i64) -> String {
    let sign = if delta >= 0 { "+" } else { "-" };
    format!("{sign}{}", human(delta.unsigned_abs()))
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn days_since(unix: i64) -> i64 {
    (now_unix() - unix).max(0) / 86_400
}

pub fn statvfs(path: &str) -> Result<FsStat> {
    let c_path = CString::new(path).context("path contains a NUL byte")?;
    // SAFETY: c_path outlives the call and `st` is a plain POD struct that
    // statvfs fully initialises when it returns 0.
    let st = unsafe {
        let mut st: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut st) != 0 {
            bail!("statvfs({path}) failed: {}", std::io::Error::last_os_error());
        }
        st
    };

    let unit = if st.f_frsize > 0 {
        st.f_frsize as u64
    } else {
        st.f_bsize as u64
    };
    let total = st.f_blocks as u64 * unit;
    let free = st.f_bavail as u64 * unit;
    // Match `df`: used counts reserved blocks, so used + avail < total.
    let used = total - st.f_bfree as u64 * unit;

    Ok(FsStat { total, used, free })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WalkResult {
    pub bytes: Bytes,
    pub files: u64,
    /// Bytes in files not modified within `cold_days`.
    pub cold: Bytes,
    /// Newest mtime seen anywhere in the tree.
    pub newest: i64,
}

/// Recursive on-disk size of `root`, in the same terms `du` reports: allocated
/// blocks rather than apparent size, each inode counted once, and never
/// crossing a filesystem boundary.
pub struct Walker {
    seen: HashSet<(u64, u64)>,
    root_dev: u64,
    cold_cutoff: i64,
    skip: HashSet<PathBuf>,
}

impl Walker {
    pub fn new(cold_days: i64) -> Self {
        Self {
            seen: HashSet::new(),
            root_dev: 0,
            cold_cutoff: now_unix() - cold_days * 86_400,
            skip: HashSet::new(),
        }
    }

    /// Subtrees another collector already accounts for. Without this the census
    /// double-counts anything nested under a claimed path.
    pub fn skipping(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.skip = paths.into_iter().collect();
        self
    }

    /// Share one walker across several roots so a file hardlinked between them
    /// is billed to whichever root reaches it first, never twice.
    pub fn walk(&mut self, root: &Path) -> WalkResult {
        let mut out = WalkResult::default();
        let Ok(meta) = fs::symlink_metadata(root) else {
            return out;
        };
        self.root_dev = meta.dev();
        self.visit(root, &mut out);
        out
    }

    fn visit(&mut self, path: &Path, out: &mut WalkResult) {
        if self.skip.contains(path) {
            return;
        }

        let Ok(meta) = fs::symlink_metadata(path) else {
            return;
        };

        if meta.dev() != self.root_dev {
            return;
        }

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            return;
        }

        if meta.nlink() > 1 && !self.seen.insert((meta.dev(), meta.ino())) {
            return;
        }

        let bytes = meta.blocks() * 512;
        out.bytes += bytes;
        out.newest = out.newest.max(meta.mtime());

        if file_type.is_file() {
            out.files += 1;
            if meta.mtime() < self.cold_cutoff {
                out.cold += bytes;
            }
            return;
        }

        if !file_type.is_dir() {
            return;
        }

        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            self.visit(&entry.path(), out);
        }
    }
}

/// `$HOME` without a trailing slash. The login shell here exports it with one,
/// which turns naive prefix arithmetic into paths like `~Projects`.
pub fn home() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let trimmed = home.trim_end_matches('/');
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Expand a leading `~` against `$HOME`.
pub fn expand_home(path: &str) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_string();
    };
    match home() {
        Some(home) => format!("{home}{rest}"),
        None => path.to_string(),
    }
}

/// Inverse of `expand_home`, for display.
pub fn collapse_home(path: &Path) -> String {
    let text = path.to_string_lossy().to_string();
    match home() {
        Some(home) if text.starts_with(&home) => format!("~{}", &text[home.len()..]),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_uses_binary_units() {
        assert_eq!(human(0), "0B");
        assert_eq!(human(512), "512B");
        assert_eq!(human(1024), "1.0K");
        assert_eq!(human(91_328_398_342), "85.1G");
    }

    #[test]
    fn human_drops_decimals_past_three_digits() {
        assert_eq!(human(1024 * 1024 * 500), "500M");
    }

    #[test]
    fn delta_carries_an_explicit_sign() {
        assert_eq!(human_delta(1024), "+1.0K");
        assert_eq!(human_delta(-1024), "-1.0K");
    }

    #[test]
    fn home_expansion_survives_a_trailing_slash() {
        std::env::set_var("HOME", "/home/u/");
        assert_eq!(expand_home("~/.cache"), "/home/u/.cache");
        assert_eq!(collapse_home(Path::new("/home/u/Projects")), "~/Projects");
        std::env::set_var("HOME", "/home/u");
        assert_eq!(expand_home("~/.cache"), "/home/u/.cache");
        assert_eq!(collapse_home(Path::new("/home/u/Projects")), "~/Projects");
        assert_eq!(collapse_home(Path::new("/var/lib")), "/var/lib");
    }

    #[test]
    fn statvfs_reports_a_plausible_root() {
        let st = statvfs("/").expect("root is always mounted");
        assert!(st.total > 0);
        assert!(st.used <= st.total);
    }
}
