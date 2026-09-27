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

use crate::model::{self, Device, Platform, Unix, View, AVD_PREFIX};
use crate::project::Project;
use crate::ssh::Where;
use crate::usage::{self, Usage};
use crate::{actions, up};

const TIMEOUT: Duration = Duration::from_secs(20);

/// One round trip for both where the host keeps its state and what is in the
/// file. A file that is not there is the ordinary case on a host nothing has
/// claimed a device on yet.
const OPEN: &str = r#"state="${XDG_STATE_HOME:-$HOME/.local/state}/phone"
printf '%s\n' "$state"
cat "$state/leases.json" 2>/dev/null
printf '\n@usage\n'
cat "$state/usage.tsv" 2>/dev/null
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

impl Holder {
    pub fn of(tree: &str, project: &str) -> Self {
        Holder {
            tree: tree.to_string(),
            project: project.to_string(),
            since: model::now(),
            host: None,
            session: None,
        }
    }

    pub fn on(mut self, host: Option<&str>) -> Self {
        self.host = host.filter(|h| !h.is_empty()).map(str::to_string);

        self
    }

    pub fn by(mut self, session: Option<&str>) -> Self {
        self.session = session.filter(|s| !s.is_empty()).map(str::to_string);

        self
    }

    /// A side naming no session is let in by its tree alone: a hold stamped
    /// before sessions existed, or a verb typed by hand.
    pub fn admits(&self, tree: &str, session: Option<&str>) -> bool {
        self.tree == tree
            && match (self.session.as_deref(), session) {
                (Some(held), Some(asking)) => held == asking,
                _ => true,
            }
    }

    /// `hotline (2h ago)`, for a message or a status row.
    pub fn label(&self) -> String {
        match &self.session {
            Some(session) => format!(
                "{}, session {} ({})",
                self.project,
                short(session),
                model::ago(self.since)
            ),
            None => format!("{} ({})", self.project, model::ago(self.since)),
        }
    }

    pub fn at(&self) -> String {
        match &self.host {
            Some(host) => format!("{} on {host}", self.tree),
            None => self.tree.clone(),
        }
    }
}

fn short(session: &str) -> &str {
    session.get(..8).unwrap_or(session)
}

pub fn session() -> Option<String> {
    ["PHONE_SESSION", "CLAUDE_CODE_SESSION_ID"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|s| s.trim().to_string())
        .find(|s| !s.is_empty())
}

/// What a device is filed under: its id as the host owning it spells it.
///
/// A simulator on the mac is `3F83…` in the mac's own registry and `mac/AAAA1111…`
/// in a laptop's, and both registries have to land on the one entry in the
/// mac's file. An emulator's `android_id:…` is already the same everywhere.
pub fn key(device: &Device) -> Key {
    let legacy = match device.platform == Platform::Emulator && device.id.starts_with(AVD_PREFIX) {
        true => device
            .aliases
            .iter()
            .filter(|a| a.starts_with("android_id:"))
            .cloned()
            .collect(),
        false => Vec::new(),
    };

    Key {
        id: filed_as(device),
        legacy,
    }
}

fn filed_as(device: &Device) -> String {
    let id = device.id.as_str();

    let Some(host) = &device.host else {
        return id.to_string();
    };

    let scope = format!("{host}/");

    if let Some(name) = id
        .strip_prefix(AVD_PREFIX)
        .and_then(|rest| rest.strip_prefix(&scope))
    {
        return format!("{AVD_PREFIX}{name}");
    }

    id.strip_prefix(&scope).unwrap_or(id).to_string()
}

#[derive(Debug)]
pub struct Key {
    pub id: String,
    legacy: Vec<String>,
}

impl From<&str> for Key {
    fn from(id: &str) -> Self {
        Key {
            id: id.to_string(),
            legacy: Vec::new(),
        }
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

    pub usage: Usage,
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

        Ok(Self::opened(at.clone(), &ran.text()))
    }

    fn opened(at: Where, text: &str) -> Self {
        let (dir, rest) = text.split_once('\n').unwrap_or((text, ""));
        let (body, used) = rest.split_once(usage::MARK).unwrap_or((rest, ""));

        let mut leases = Self::read(at, dir.trim(), body.as_bytes());
        leases.usage = Usage::parse(used);

        leases
    }

    pub fn read(at: Where, dir: &str, body: &[u8]) -> Self {
        Leases {
            held: serde_json::from_slice(body).unwrap_or_default(),
            at,
            dir: dir.to_string(),
            usage: Usage::default(),
        }
    }

    pub fn holder(&self, key: impl Into<Key>) -> Option<&Holder> {
        let key = key.into();

        self.held
            .get(&key.id)
            .or_else(|| key.legacy.iter().find_map(|id| self.held.get(id)))
    }

    /// Who holds `id`, if it is not the project at `tree`.
    pub fn other(&self, key: impl Into<Key>, tree: &str, session: Option<&str>) -> Option<&Holder> {
        self.holder(key)
            .filter(|holder| !holder.admits(tree, session))
    }

    /// Puts `holder`'s name on `id`. A project already holding it keeps its
    /// original `since`: the hold began when it began, not when it was last
    /// renewed.
    pub fn take(&mut self, key: impl Into<Key>, holder: Holder) {
        let key = key.into();
        let held = match self
            .remove(&key)
            .filter(|had| had.admits(&holder.tree, holder.session.as_deref()))
        {
            Some(mut had) => {
                if had.session.is_none() {
                    had.session = holder.session;
                }

                had
            }
            None => holder,
        };

        self.held.insert(key.id, held);
    }

    pub fn release(&mut self, key: impl Into<Key>) -> Option<Holder> {
        self.remove(&key.into())
    }

    fn remove(&mut self, key: &Key) -> Option<Holder> {
        let had = self.held.remove(&key.id);

        key.legacy
            .iter()
            .fold(had, |had, id| had.or(self.held.remove(id)))
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
    Beside,
    Elsewhere {
        project: String,
        tree: String,
    },
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

            if holder.admits(&tree, session().as_deref()) {
                return Ok(());
            }

            match tree == holder.tree {
                true => Caller::Beside,
                false => Caller::Elsewhere {
                    project: project.name(),
                    tree,
                },
            }
        }
        None => Caller::Nowhere,
    };

    Err(crate::Refused(refusal(&view.device.label, holder, &caller)).into())
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
        Caller::Beside => {
            return format!(
                "{label} is held from this same checkout under a different session id: {}\n\
                 usually another agent working here, whose reinstalls and restarts would land in the middle of your run; but a session id is a best guess, and a `/clear` or a resume gives this same agent a new one\n\
                 if you took this device earlier in this conversation, it is yours: `phone up --take` moves the hold to this session id\n\
                 otherwise pick a device nobody holds (`phone device list` names every holder), or wait for its `phone down`; ask before taking it",
                holder.label()
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

pub async fn ledgers(views: &[View]) -> Vec<(Where, Leases)> {
    let mut hosts: Vec<Where> = Vec::new();

    for view in views {
        let at = actions::where_of(&view.device);

        if !hosts.contains(&at) {
            hosts.push(at);
        }
    }

    let opened = hosts
        .into_iter()
        .map(|at| async move { Leases::open(&at).await.ok().map(|leases| (at, leases)) });

    futures_util::future::join_all(opened)
        .await
        .into_iter()
        .flatten()
        .collect()
}

pub fn holds(views: &[View], ledgers: &[(Where, Leases)]) -> BTreeMap<String, Holder> {
    views
        .iter()
        .filter(|v| actions::running(&v.reach))
        .filter_map(|view| {
            let at = actions::where_of(&view.device);
            let (_, leases) = ledgers.iter().find(|(known, _)| *known == at)?;
            let holder = leases.holder(key(&view.device))?;

            Some((view.device.id.clone(), holder.clone()))
        })
        .collect()
}

pub async fn mine() -> Option<Holder> {
    let project = Project::here().ok().flatten()?;
    let tree = tree(&project).await.ok()?;

    Some(Holder::of(&tree, &project.name()).by(session().as_deref()))
}

/// Drops whatever hold a device this process just booted carried: the session
/// that held it did not survive the shutdown, and a hold with no session behind
/// it would refuse everyone until somebody found the right checkout to run
/// `phone down` in.
pub async fn forget(view: &View) -> Result<Option<String>> {
    let mut leases = Leases::open(&actions::where_of(&view.device)).await?;

    let Some(had) = leases.release(key(&view.device)) else {
        return Ok(None);
    };

    leases.save().await?;

    Ok(Some(format!(
        "{} was held by {}; that session did not survive the shutdown, so the hold is dropped",
        view.device.label,
        had.label()
    )))
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

    async fn opened(state: &str) -> Leases {
        let ran = Where::Here
            .exec(&format!("XDG_STATE_HOME='{state}'\n{OPEN}"), &[], TIMEOUT)
            .await
            .unwrap();

        Leases::opened(Where::Here, &ran.text())
    }

    #[tokio::test]
    async fn one_read_brings_back_the_holds_and_the_usage_beside_them() {
        let state = temp("usage");
        let pixel = Device::new("avd:pixel", "pixel", Platform::Emulator);

        let empty = opened(&state).await;

        assert!(empty.holder("avd:pixel").is_none());
        assert_eq!(empty.usage.of(&pixel, None).all, None);

        std::fs::create_dir_all(format!("{state}/phone")).unwrap();
        std::fs::write(
            format!("{state}/phone/leases.json"),
            r#"{"avd:pixel":{"tree":"/a","project":"alpha","since":1}}"#,
        )
        .unwrap();

        assert!(opened(&state).await.holder("avd:pixel").is_some());

        std::fs::write(format!("{state}/phone/usage.tsv"), "avd:pixel\t/a\t5\t7\n").unwrap();

        let both = opened(&state).await;

        assert_eq!(both.dir, format!("{state}/phone"));
        assert!(both.holder("avd:pixel").is_some());
        assert_eq!(
            both.usage.of(&pixel, Some("/a")).project.map(|u| u.count),
            Some(7)
        );

        std::fs::remove_dir_all(&state).unwrap();
    }

    #[test]
    fn the_holder_is_only_somebody_else_from_another_tree() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));

        assert!(leases.other("emu:1", "/a", None).is_none());
        assert_eq!(
            leases
                .other("emu:1", "/b", None)
                .map(|h| h.project.as_str()),
            Some("alpha")
        );
        assert!(leases.other("emu:2", "/b", None).is_none());
    }

    #[test]
    fn a_second_session_in_the_same_tree_is_somebody_else() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha").by(Some("one")));

        assert!(leases.other("emu:1", "/a", Some("one")).is_none());
        assert_eq!(
            leases
                .other("emu:1", "/a", Some("two"))
                .and_then(|h| h.session.as_deref()),
            Some("one")
        );
    }

    #[test]
    fn a_side_with_no_session_is_let_in_by_its_tree() {
        let old: Holder =
            serde_json::from_str(r#"{"tree":"/a","project":"alpha","since":1000}"#).unwrap();

        assert_eq!(old.session, None);
        assert!(old.admits("/a", Some("two")));
        assert!(!old.admits("/b", Some("two")));

        assert!(Holder::of("/a", "alpha").by(Some("one")).admits("/a", None));
    }

    #[test]
    fn renewing_a_hold_from_before_sessions_stamps_the_session() {
        let mut leases = Leases::default();
        let mut old = Holder::of("/a", "alpha");

        old.since = 1_000;
        leases.take("emu:1", old);
        leases.take("emu:1", Holder::of("/a", "alpha").by(Some("one")));

        let holder = leases.holder("emu:1").unwrap();

        assert_eq!(
            (holder.since, holder.session.as_deref()),
            (1_000, Some("one"))
        );
    }

    #[test]
    fn a_hold_without_a_session_is_written_as_before() {
        let body = serde_json::to_string(&Holder::of("/a", "alpha")).unwrap();

        assert!(!body.contains("session"));
    }

    #[test]
    fn a_second_session_is_told_the_hold_is_in_its_own_checkout() {
        let said = refusal(
            "Pixel 9",
            &Holder::of("/home/x/hotline", "hotline").by(Some("5dac5f28-ad4e-48bf")),
            &Caller::Beside,
        );

        assert!(said.contains("under a different session id: hotline, session 5dac5f28 ("));
        assert!(said.contains("a `/clear` or a resume"));
        assert!(said.ends_with("ask before taking it"));
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

        let mut avd = Device::new("avd:mac/pixel", "pixel", Platform::Emulator);
        avd.host = Some("mac".to_string());

        assert_eq!(key(&avd).id, "avd:pixel");

        assert_eq!(key(&sim).id, "AAAA1111");
        assert_eq!(key(&emu).id, "android_id:3333");
        assert_eq!(key(&local).id, "android_id:4444");
    }

    #[test]
    fn a_hold_under_the_android_id_follows_the_row_to_its_avd() {
        let mut leases = Leases::default();

        leases.take("android_id:dc3f6e59", Holder::of("/a", "alpha"));

        let mut pixel = Device::new("avd:rose/pixel", "pixel", Platform::Emulator);
        pixel.host = Some("rose".to_string());
        pixel.add_alias("android_id:dc3f6e59");

        let mut clone = Device::new("avd:rose/pixel-c", "pixel-c", Platform::Emulator);
        clone.host = Some("rose".to_string());

        assert_eq!(
            leases.other(key(&pixel), "/b", None).map(|h| h.project.as_str()),
            Some("alpha")
        );
        assert!(
            leases.other(key(&clone), "/b", None).is_none(),
            "a clone does not own its source's android_id"
        );

        leases.take(key(&pixel), Holder::of("/a", "alpha"));

        assert!(leases.holder("android_id:dc3f6e59").is_none());
        assert_eq!(
            leases.holder("avd:pixel").map(|h| h.project.as_str()),
            Some("alpha")
        );
    }

    #[tokio::test]
    async fn a_ledger_that_came_from_nowhere_is_not_written_anywhere() {
        let mut leases = Leases::default();

        leases.take("emu:1", Holder::of("/a", "alpha"));
        leases.save().await.unwrap();
    }
}
