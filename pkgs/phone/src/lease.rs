//! Who has a device, so a second project's `up` is refused rather than served.
//!
//! `up` converges one manifest and knows nothing of any other. Two projects
//! naming the same emulator each run to completion, and the second launch puts
//! its app in front of the first's; from then on every snapshot the first agent
//! takes describes the wrong screen, and nothing says so. This is the saying so.
//!
//! Kept on the host the device hangs off, for the reason the stamps give: the
//! projects driving one device may sit on different machines — one handed over
//! to the mac that owns the emulator, another driving it from a laptop over
//! adb — and a file on either would be invisible to the other. The device's
//! host is the one place both go through.
//!
//! Held until `down` rather than until the process exits: `up` returns and the
//! agent keeps driving the device for as long as it likes. A device that is off
//! carries nobody's session, so a lease on one is ignored rather than honoured.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{self, Device, Unix, View};
use crate::project::Project;
use crate::ssh::Where;
use crate::{actions, up};

const TIMEOUT: Duration = Duration::from_secs(20);

/// One round trip for both where the host keeps its state and what is in the
/// file. A file that is not there is the ordinary case on a host nothing has
/// claimed a device on yet.
const OPEN: &str = r#"state="${XDG_STATE_HOME:-$HOME/.local/state}/phone"
printf '%s\n' "$state"
cat "$state/leases.json" 2>/dev/null
exit 0"#;

/// A rename rather than a truncate-and-fill, for the reason the registry gives:
/// every project on the host shares this file.
const WRITE: &str = r#"mkdir -p "$1" || exit 1
tmp="$1/leases.json.$$.tmp"
printf '%s' "$2" > "$tmp" && mv "$tmp" "$1/leases.json""#;

/// The project holding a device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    /// The project's root as the host owning its tree spells it: what the
    /// stamps are keyed by, and the one name every machine driving it agrees
    /// on. Two runs with the same tree are one project renewing its hold.
    pub tree: String,
    /// What to call it to a reader.
    pub project: String,
    pub since: Unix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

impl Holder {
    pub fn of(tree: &str, project: &str) -> Self {
        Holder {
            tree: tree.to_string(),
            project: project.to_string(),
            since: model::now(),
            host: None,
        }
    }

    pub fn on(mut self, host: Option<&str>) -> Self {
        self.host = host.filter(|h| !h.is_empty()).map(str::to_string);

        self
    }

    /// `hotline (2h ago)`, for a message or a status row.
    pub fn label(&self) -> String {
        format!("{} ({})", self.project, model::ago(self.since))
    }

    pub fn at(&self) -> String {
        match &self.host {
            Some(host) => format!("{} on {host}", self.tree),
            None => self.tree.clone(),
        }
    }
}

/// What a device is filed under: its id as the host owning it spells it.
///
/// A simulator on the mac is `3F83…` in the mac's own registry and `mac/AAAA1111…`
/// in a laptop's, and both registries have to land on the one entry in the
/// mac's file. An emulator's `android_id:…` is already the same everywhere.
pub fn key(device: &Device) -> &str {
    let id = device.id.as_str();

    match &device.host {
        Some(host) => id.strip_prefix(&format!("{host}/")).unwrap_or(id),
        None => id,
    }
}

#[derive(Debug, Default)]
pub struct Leases {
    /// Device key -> who holds it. The id rather than the label, since the id
    /// is what survives a transport change; the label is what a message says.
    held: BTreeMap<String, Holder>,

    at: Where,

    /// The directory holding the file, as that host spells it. Empty means
    /// nothing was read and nothing will be written.
    dir: String,
}

impl Leases {
    /// The leases the host at `at` keeps.
    ///
    /// An unreadable file reads as nothing held: refusing every `up` on the
    /// host over a bad file would cost more than one collision.
    pub async fn open(at: &Where) -> Result<Self> {
        let ran = at
            .exec(OPEN, &[], TIMEOUT)
            .await
            .with_context(|| format!("reading the leases on {}", at.label()))?;

        let text = ran.text();
        let (dir, body) = text.split_once('\n').unwrap_or((&text, ""));

        Ok(Self::read(at.clone(), dir.trim(), body.as_bytes()))
    }

    pub fn read(at: Where, dir: &str, body: &[u8]) -> Self {
        Leases {
            held: serde_json::from_slice(body).unwrap_or_default(),
            at,
            dir: dir.to_string(),
        }
    }

    pub fn holder(&self, id: &str) -> Option<&Holder> {
        self.held.get(id)
    }

    /// Who holds `id`, if it is not the project at `tree`.
    pub fn other(&self, id: &str, tree: &str) -> Option<&Holder> {
        self.holder(id).filter(|holder| holder.tree != tree)
    }

    /// Puts `holder`'s name on `id`. A project already holding it keeps its
    /// original `since`: the hold began when it began, not when it was last
    /// renewed.
    pub fn take(&mut self, id: &str, holder: Holder) {
        match self.held.get(id) {
            Some(had) if had.tree == holder.tree => {}
            _ => {
                self.held.insert(id.to_string(), holder);
            }
        }
    }

    pub fn release(&mut self, id: &str) -> Option<Holder> {
        self.held.remove(id)
    }

    pub async fn save(&self) -> Result<()> {
        if self.dir.is_empty() {
            return Ok(());
        }

        let body = serde_json::to_string_pretty(&self.held)?;
        let ran = self
            .at
            .exec(WRITE, &[&self.dir, &body], TIMEOUT)
            .await
            .with_context(|| format!("writing the leases on {}", self.at.label()))?;

        match ran.ok() {
            true => Ok(()),
            false => anyhow::bail!(
                "could not write the leases on {}: {}",
                self.at.label(),
                ran.said
            ),
        }
    }
}


/// Where a project's tree is, as the host owning it spells it: the name `up`
/// filed its hold under, so a verb typed in the same checkout finds its own
/// hold rather than somebody else's. The same script `up` runs, so the two
/// cannot drift apart on a symlink or a tilde.
pub async fn tree(project: &Project) -> Result<String> {
    let at = Where::of(project.host());
    let dir = project.dir();
    let ran = at
        .exec(&up::scripted("pwd -P"), &up::args(&dir, None), TIMEOUT)
        .await
        .with_context(|| format!("locating {dir} on {}", at.label()))?;

    if !ran.ok() {
        anyhow::bail!("locating {dir} on {}: {}", at.label(), ran.said);
    }

    Ok(ran.text().trim().to_string())
}

pub enum Caller {
    Nowhere,
    Elsewhere { project: String, tree: String },
}

/// Refuses a running device that another project holds.
///
/// `claim` guards `up`; this guards everything after it. A hold only `up`
/// honoured would stop a second project's launch and let a second agent's
/// `tap` straight through, and the tap is the invasion: from then on the
/// holder's snapshots describe a screen somebody else is driving.
///
/// A project of one's own is needed to be let in, not to be refused: a verb
/// typed outside any checkout is nobody, and nobody is not the holder.
pub async fn check(view: &View) -> Result<()> {
    if !actions::running(&view.reach) {
        return Ok(());
    }

    let leases = Leases::open(&actions::where_of(&view.device)).await?;

    let Some(holder) = leases.holder(key(&view.device)) else {
        return Ok(());
    };

    let caller = match Project::here().ok().flatten() {
        Some(project) => {
            let tree = tree(&project).await?;

            if tree == holder.tree {
                return Ok(());
            }

            Caller::Elsewhere {
                project: project.name(),
                tree,
            }
        }
        None => Caller::Nowhere,
    };

    anyhow::bail!("{}", refusal(&view.device.label, holder, &caller))
}

/// What a refused agent reads. It has to say whose the device is, why the
/// refusal is not a bug to route around, and the two ways past it — with the
/// one that walks over the other session named last and as a question to ask.
pub fn refusal(label: &str, holder: &Holder, caller: &Caller) -> String {
    let (project, tree) = match caller {
        Caller::Nowhere => {
            return format!(
                "{label} is held by {}, and this is not running from a project\n\
                 a verb typed outside any checkout is nobody, and nobody is not the holder, so the hold refuses it even when the hold is its own\n\
                 run it from {}, or pick a device nobody holds (`phone device list` names every holder)",
                holder.label(),
                holder.at()
            );
        }
        Caller::Elsewhere { project, tree } => (project, tree),
    };

    if *project == holder.project {
        return format!(
            "{label} is held by {}, which is another checkout of {project} and not this one\n\
             the hold is on {}, this is {tree}\n\
             run it from there, or `phone up --take` here to move the hold; ask before using it",
            holder.label(),
            holder.at()
        );
    }

    format!(
        "{label} is held by {}: another agent's session is on it, and driving it from here would put your screens in front of theirs\n\
         pick a device nobody holds (`phone device list` names every holder), or have that agent release it with `phone down` in {}\n\
         `phone up --take` there overrides the hold; ask before using it",
        holder.label(),
        holder.at()
    )
}

pub async fn holds(views: &[View]) -> BTreeMap<String, Holder> {
    let mut found = BTreeMap::new();
    let mut read: Vec<(Where, Leases)> = Vec::new();

    for view in views.iter().filter(|v| actions::running(&v.reach)) {
        let at = actions::where_of(&view.device);

        let leases = match read.iter().position(|(known, _)| *known == at) {
            Some(i) => &read[i].1,
            None => match Leases::open(&at).await {
                Ok(leases) => {
                    read.push((at, leases));

                    &read.last().expect("just pushed").1
                }
                Err(_) => continue,
            },
        };

        if let Some(holder) = leases.holder(key(&view.device)) {
            found.insert(view.device.id.clone(), holder.clone());
        }
    }

    found
}

pub async fn mine() -> Option<String> {
    let project = Project::here().ok().flatten()?;

    tree(&project).await.ok()
}

/// Drops whatever hold a device this process just booted carried: the session
/// that held it did not survive the shutdown, and a hold with no session behind
/// it would refuse everyone until somebody found the right checkout to run
/// `phone down` in.
pub async fn forget(view: &View) -> Result<()> {
    let mut leases = Leases::open(&actions::where_of(&view.device)).await?;

    if let Some(had) = leases.release(key(&view.device)) {
        eprintln!(
            "phone: {} was held by {}; that session did not survive the shutdown, so the hold is dropped",
            view.device.label,
            had.label()
        );

        leases.save().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Platform;

    fn temp(what: &str) -> String {
        std::env::temp_dir()
            .join(format!("phone-leases-{what}-{}", std::process::id()))
            .display()
            .to_string()
    }

    async fn reread(dir: &str) -> Leases {
        let ran = Where::Here
            .exec(
                r#"cat "$1/leases.json" 2>/dev/null; exit 0"#,
                &[dir],
                TIMEOUT,
            )
            .await
            .unwrap();

        Leases::read(Where::Here, dir, &ran.stdout)
    }

    #[tokio::test]
    async fn a_hold_survives_the_process_that_took_it() {
        let dir = temp("kept");
        let mut leases = Leases::read(Where::Here, &dir, b"");

        leases.take("emu:1", Holder::of("/a", "alpha"));
        leases.save().await.unwrap();

        let read = reread(&dir).await;

        assert_eq!(
            read.holder("emu:1").map(|h| h.project.as_str()),
            Some("alpha")
        );
        assert_eq!(read.holder("emu:2"), None);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_holder_is_only_somebody_else_from_another_tree() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));

        assert!(leases.other("emu:1", "/a").is_none());
        assert_eq!(
            leases.other("emu:1", "/b").map(|h| h.project.as_str()),
            Some("alpha")
        );
        assert!(leases.other("emu:2", "/b").is_none());
    }

    fn elsewhere(project: &str, tree: &str) -> Caller {
        Caller::Elsewhere {
            project: project.to_string(),
            tree: tree.to_string(),
        }
    }

    #[test]
    fn a_refusal_names_the_holder_and_where_to_release_it() {
        let said = refusal(
            "Pixel 9",
            &Holder::of("/home/x/hotline", "hotline"),
            &elsewhere("clipz", "/home/x/clipz"),
        );

        assert!(said.starts_with("Pixel 9 is held by hotline ("));
        assert!(said.contains("`phone down` in /home/x/hotline"));
        assert!(said.ends_with("ask before using it"));
    }

    #[test]
    fn a_refusal_says_which_machine_the_tree_is_on() {
        let said = refusal(
            "Pixel 9",
            &Holder::of("/ext/projects/hotline", "hotline").on(Some("rose")),
            &elsewhere("clipz", "/home/x/clipz"),
        );

        assert!(said.contains("`phone down` in /ext/projects/hotline on rose"));
    }

    #[test]
    fn nobody_is_not_told_it_walked_into_somebody() {
        let said = refusal(
            "Pixel 9",
            &Holder::of("/home/x/hotline", "hotline"),
            &Caller::Nowhere,
        );

        assert!(said.contains("this is not running from a project"));
        assert!(said.contains("run it from /home/x/hotline"));
        assert!(!said.contains("another agent's session"));
    }

    #[test]
    fn a_second_checkout_is_told_it_is_the_same_project() {
        let said = refusal(
            "Pixel 9",
            &Holder::of("/ext/projects/hotline", "hotline").on(Some("rose")),
            &elsewhere("hotline", "/home/x/hotline"),
        );

        assert!(said.contains("another checkout of hotline and not this one"));
        assert!(
            said.contains("the hold is on /ext/projects/hotline on rose, this is /home/x/hotline")
        );
        assert!(!said.contains("another agent's session"));
    }

    #[test]
    fn renewing_a_hold_keeps_when_it_began() {
        let mut leases = Leases::default();
        let mut first = Holder::of("/a", "alpha");

        first.since = 1_000;
        leases.take("emu:1", first);
        leases.take("emu:1", Holder::of("/a", "alpha"));

        assert_eq!(leases.holder("emu:1").map(|h| h.since), Some(1_000));
    }

    #[test]
    fn taking_over_replaces_the_holder_outright() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));
        leases.take("emu:1", Holder::of("/b", "beta"));

        let holder = leases.holder("emu:1").unwrap();

        assert_eq!(
            (holder.tree.as_str(), holder.project.as_str()),
            ("/b", "beta")
        );
    }

    #[test]
    fn a_release_leaves_the_others_held() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));
        leases.take("emu:2", Holder::of("/b", "beta"));

        assert_eq!(
            leases.release("emu:1").map(|h| h.project),
            Some("alpha".into())
        );
        assert!(leases.holder("emu:1").is_none());
        assert!(leases.holder("emu:2").is_some());
    }

    /// Half a file, or one from an older shape of the struct, must cost one
    /// collision at most rather than every `up` on the host.
    #[tokio::test]
    async fn an_unreadable_file_reads_as_nothing_held() {
        let dir = temp("bad");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/leases.json"), b"{ not json").unwrap();

        assert!(reread(&dir).await.holder("emu:1").is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_device_is_filed_as_its_own_host_spells_it() {
        let mut sim = Device::new("mac/AAAA1111", "iPhone 17", Platform::Simulator);
        sim.host = Some("mac".to_string());

        let mut emu = Device::new("android_id:3333", "pixel", Platform::Emulator);
        emu.host = Some("mac".to_string());

        let local = Device::new("android_id:4444", "pixel", Platform::Emulator);

        assert_eq!(key(&sim), "AAAA1111");
        assert_eq!(key(&emu), "android_id:3333");
        assert_eq!(key(&local), "android_id:4444");
    }

    #[tokio::test]
    async fn a_ledger_that_came_from_nowhere_is_not_written_anywhere() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));
        leases.save().await.unwrap();
    }
}
