//! What was last built, and out of what.
//!
//! `up` has to answer "is this still current?" without knowing what any of the
//! declared commands do. The manifest answers it for itself: a `stale` command
//! prints whatever identifies the inputs — a lockfile checksum, an Expo
//! fingerprint, a `git rev-parse` — and this remembers the hash of that. When
//! the print changes, the work runs.
//!
//! Kept on the host that owns the tree rather than on the machine driving it.
//! The print it remembers is taken there, out of files that live there, and the
//! artifact it describes was written there; a ledger on the client would be one
//! per client, so a second laptop, or CI, or a shell on the host itself would
//! each rebuild what the others had already built.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::ssh::Where;

/// The scripts below spell it themselves, being shell; this is for the tests
/// that go looking for the file afterwards.
#[cfg(test)]
const FILE: &str = "stamps.json";

/// Long enough for a host that has to open an ssh session first, short enough
/// that an unreachable one fails rather than hangs a build.
const TIMEOUT: Duration = Duration::from_secs(20);

/// A rename rather than a truncate-and-fill, for the reason the registry gives:
/// two runs against two projects share this file.
const WRITE: &str = r#"mkdir -p "$1" || exit 1
tmp="$1/stamps.json.$$.tmp"
printf '%s' "$2" > "$tmp" && mv "$tmp" "$1/stamps.json""#;

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

#[derive(Debug, Default)]
pub struct Stamps {
    /// Project root as the host that owns it spells it -> step name -> hash of
    /// what it was built from. Keyed by that path because it is what tells two
    /// checkouts of the same repository apart, and the only spelling every
    /// machine driving the host agrees on.
    projects: BTreeMap<String, BTreeMap<String, String>>,

    at: Where,

    /// The directory holding the file, as that host spells it. Empty means
    /// nothing was read and nothing will be written, which is what the tests
    /// that only exercise the map want.
    dir: String,
}

impl Stamps {
    /// The ledger that `dir` on `at` holds, out of bytes somebody else has
    /// already fetched: the caller reads it in the same round trip it asks the
    /// host what it calls the project, since neither answer is worth a session
    /// of its own.
    ///
    /// A stamp file that cannot be read is not worth failing a run over: the
    /// worst it costs is one rebuild, and refusing to start would cost more.
    pub fn read(at: Where, dir: &str, body: &[u8]) -> Self {
        Stamps {
            projects: serde_json::from_slice(body).unwrap_or_default(),
            at,
            dir: dir.to_string(),
        }
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
    /// run rebuild. `--rebuild` does it: a stamp is a claim about an artifact,
    /// and the point of forcing a build is that the claim is not believed.
    pub fn forget(&mut self, project: &str) {
        self.projects.remove(project);
    }

    pub async fn save(&self) -> Result<()> {
        if self.dir.is_empty() {
            return Ok(());
        }

        let body = serde_json::to_string_pretty(&self.projects)?;
        let ran = self
            .at
            .exec(WRITE, &[&self.dir, &body], TIMEOUT)
            .await
            .with_context(|| format!("writing the stamps on {}", self.at.label()))?;

        match ran.ok() {
            true => Ok(()),
            false => anyhow::bail!("could not write the stamps on {}: {}", self.at.label(), ran.said),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(what: &str) -> String {
        std::env::temp_dir()
            .join(format!("phone-stamps-{what}-{}", std::process::id()))
            .display()
            .to_string()
    }

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

    async fn reread(dir: &str) -> Stamps {
        let ran = Where::Here
            .exec(r#"cat "$1/stamps.json" 2>/dev/null; exit 0"#, &[dir], TIMEOUT)
            .await
            .unwrap();

        Stamps::read(Where::Here, dir, &ran.stdout)
    }

    #[tokio::test]
    async fn a_step_is_remembered_per_project() {
        let dir = temp("kept");
        let mut stamps = Stamps::read(Where::Here, &dir, b"");

        stamps.set("/a", "deps", "1111");
        stamps.set("/a", "build.android", "2222");
        stamps.set("/b", "deps", "3333");
        stamps.save().await.unwrap();

        let read = reread(&dir).await;

        assert_eq!(read.get("/a", "deps"), Some("1111"));
        assert_eq!(read.get("/a", "build.android"), Some("2222"));
        assert_eq!(read.get("/b", "deps"), Some("3333"));
        assert_eq!(read.get("/b", "build.android"), None);
        assert_eq!(read.get("/c", "deps"), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The bug this closes: the ledger was written on the machine running the
    /// command, so the host that had actually done the building was never asked
    /// what it remembered.
    #[tokio::test]
    async fn the_ledger_is_written_where_the_work_happens() {
        let dir = temp("there");
        let mut stamps = Stamps::read(Where::Here, &dir, b"");

        stamps.set("/w", "deps", "4444");
        stamps.save().await.unwrap();

        let written = std::fs::read_to_string(format!("{dir}/{FILE}")).unwrap();

        assert!(
            written.contains("4444"),
            "the host holds the file, not the client: {written}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn forgetting_one_project_leaves_the_others() {
        let mut stamps = Stamps::default();

        stamps.set("/a", "deps", "1111");
        stamps.set("/b", "deps", "2222");
        stamps.forget("/a");

        assert_eq!(stamps.get("/a", "deps"), None);
        assert_eq!(stamps.get("/b", "deps"), Some("2222"));
    }

    /// A stamp file left half-written, or written by an older shape of this
    /// struct, must cost one rebuild rather than every command in the project.
    #[tokio::test]
    async fn an_unreadable_stamp_file_reads_as_nothing_built() {
        let dir = temp("bad");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{FILE}"), b"{ not json").unwrap();

        assert_eq!(reread(&dir).await.get("/a", "deps"), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Nothing was read, so there is nowhere to write: a default ledger is a
    /// scratch map, and saving one would land it on top of a real file.
    #[tokio::test]
    async fn a_ledger_that_came_from_nowhere_is_not_written_anywhere() {
        let mut stamps = Stamps::default();

        stamps.set("/a", "deps", "1111");

        stamps.save().await.unwrap();
    }
}
