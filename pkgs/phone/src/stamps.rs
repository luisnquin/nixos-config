//! What was last built, and out of what.
//!
//! `up` has to answer "is this still current?" without knowing what any of the
//! declared commands do. The manifest answers it for itself: a `stale` command
//! prints whatever identifies the inputs — a lockfile checksum, an Expo
//! fingerprint, a `git rev-parse` — and this remembers the hash of that. When
//! the print changes, the work runs.
//!
//! Kept beside the registry rather than in the project, because it describes
//! this machine's idea of what it has already done and would be noise in a
//! repository shared with anyone else.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::registry::state_dir;

const FILE: &str = "stamps.json";

/// FNV-1a, 64 bit.
///
/// `DefaultHasher` would be the obvious choice and is the wrong one: its output
/// is explicitly not stable across Rust releases, and this hash outlives the
/// binary that wrote it. A toolchain bump would silently invalidate every stamp
/// and rebuild everything once, which is the kind of bug that gets blamed on
/// the build rather than on the hash.
pub fn hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in bytes {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(0x100_0000_01b3);
    }

    format!("{h:016x}")
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Stamps {
    /// Project root on this machine -> step name -> hash of what it was built
    /// from. Keyed by the local path because that is what tells two checkouts
    /// of the same repository apart.
    #[serde(flatten)]
    projects: BTreeMap<String, BTreeMap<String, String>>,

    #[serde(skip)]
    path: PathBuf,
}

impl Stamps {
    pub fn load() -> Result<Self> {
        Self::load_from(&state_dir().join(FILE))
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        // a stamp file that cannot be read is not worth failing a run over: the
        // worst it costs is one rebuild, and refusing to start would cost more
        let mut stamps = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<Stamps>(&bytes).unwrap_or_default(),
            Err(_) => Stamps::default(),
        };

        stamps.path = path.to_path_buf();

        Ok(stamps)
    }

    pub fn get(&self, project: &str, step: &str) -> Option<&str> {
        self.projects.get(project)?.get(step).map(String::as_str)
    }

    pub fn set(&mut self, project: &str, step: &str, hash: &str) {
        self.projects
            .entry(project.to_string())
            .or_default()
            .insert(step.to_string(), hash.to_string());
    }

    /// Forgets everything known about a project, which is what makes the next
    /// run rebuild. `down` does it: a torn-down device carries nothing, so a
    /// stamp saying otherwise is a lie that survives the teardown.
    pub fn forget(&mut self, project: &str) {
        self.projects.remove(project);
    }

    /// A rename rather than a truncate-and-fill, for the reason the registry
    /// gives: two runs against two projects share this file.
    pub fn save(&self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }

        let dir = self
            .path
            .parent()
            .context("stamp path has no parent directory")?;

        std::fs::create_dir_all(dir)?;

        let tmp = dir.join(format!("{FILE}.{}.tmp", std::process::id()));

        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &self.path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stamps outlive the binary, so the hash has to be the same number
    /// next year. Pinning the values is the only way that claim gets checked.
    #[test]
    fn the_hash_is_the_same_number_every_time() {
        assert_eq!(hash(b""), "cbf29ce484222325");
        assert_eq!(hash(b"a"), "af63dc4c8601ec8c");
        assert_eq!(hash(b"foobar"), "85944171f73967e8");
    }

    #[test]
    fn a_hash_moves_when_the_input_does() {
        assert_ne!(
            hash(b"1234 4096 package-lock.json"),
            hash(b"9999 4096 package-lock.json")
        );
    }

    #[test]
    fn a_step_is_remembered_per_project() {
        let dir = std::env::temp_dir().join(format!("phone-stamps-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join(FILE);
        let mut stamps = Stamps::load_from(&path).unwrap();

        stamps.set("/a", "deps", "1111");
        stamps.set("/a", "build.android", "2222");
        stamps.set("/b", "deps", "3333");
        stamps.save().unwrap();

        let read = Stamps::load_from(&path).unwrap();

        assert_eq!(read.get("/a", "deps"), Some("1111"));
        assert_eq!(read.get("/a", "build.android"), Some("2222"));
        assert_eq!(read.get("/b", "deps"), Some("3333"));
        assert_eq!(read.get("/b", "build.android"), None);
        assert_eq!(read.get("/c", "deps"), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn forgetting_one_project_leaves_the_others() {
        let mut stamps = Stamps::default();

        stamps.set("/a", "deps", "1111");
        stamps.set("/b", "deps", "2222");
        stamps.forget("/a");

        assert_eq!(stamps.get("/a", "deps"), None);
        assert_eq!(stamps.get("/b", "deps"), Some("2222"));
    }

    /// A stamp file left half-written, or written by an older shape of this
    /// struct, must cost one rebuild rather than every command in the project.
    #[test]
    fn an_unreadable_stamp_file_reads_as_nothing_built() {
        let dir = std::env::temp_dir().join(format!("phone-stamps-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join(FILE);
        std::fs::write(&path, b"{ not json").unwrap();

        assert_eq!(Stamps::load_from(&path).unwrap().get("/a", "deps"), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
