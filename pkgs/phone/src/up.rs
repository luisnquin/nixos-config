//! Converging a project's devices on the state its manifest declares.
//!
//! Every other verb here is imperative: it does one thing to one device and
//! reports what happened. That is the right shape for driving a screen and the
//! wrong shape for getting to the point where a screen can be driven, which is
//! a dozen steps that each have to be skipped when they are already done — boot
//! the emulator, attach to it, forward the bundler's port, unfreeze the
//! animations, install a build that matches the source, start the app.
//!
//! So `up` is not a script of those verbs. It reads what the project says it
//! wants, reads what is actually there, and does the difference. Running it
//! twice does the work once; running it after nothing changed opens the app and
//! stops. That property is what lets an agent begin every session with it
//! rather than having to work out which half of the setup survived.
//!
//! Nothing in here knows what Expo or Gradle or npm are. The manifest supplies
//! a command that prints what the inputs are, and a command that does the work;
//! this decides when to run the second one.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::connect;
use crate::alloc;
use crate::discover::{self, survey};
use crate::lease::{self, Book, Lease, Standing};
use crate::model::{self, Device, Platform, Reach, View};
use crate::pids;
use crate::project::{Build, Level, Project, Spec, Task};
use crate::registry::Registry;
use crate::ssh::{Status, Where};
use crate::stamps::{self, Stamps};
use crate::{actions, apps, config, memory};

/// Long enough for a `git rev-parse` over a cold ssh session or an Expo
/// fingerprint over a large tree, short enough that a hung probe is reported
/// rather than waited on. Freshness is a question, not the work.
const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

/// How long to wait for the bundler's port to start answering after starting
/// it. Metro on a cold cache is the slow case.
const BUNDLER_TIMEOUT: Duration = Duration::from_secs(90);

pub struct Opts {
    pub profile: Option<String>,
    pub target: Option<String>,
    /// Run the build steps whatever the stamps say.
    pub rebuild: bool,
    pub over_budget: bool,
    pub timeout: Duration,
}

/// What one declared device wants and what it has, as `status` prints it and
/// `up` decides from.
#[derive(Debug, Serialize, Deserialize)]
pub struct Row {
    pub name: String,
    pub platform: Option<String>,
    pub want: String,
    pub have: String,
    /// Why it is not there yet, or what it is doing while it is.
    pub note: String,
    /// The project driving it, when that is not this one. Drift in its own
    /// right: a device that is prepared and somebody else's is not usable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<String>,
}

/// One `status` of a whole project: its steps, its devices and the strays
/// beside them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub project: String,
    pub host: Option<String>,
    pub steps: Vec<Row>,
    pub devices: Vec<Row>,

    /// Running, reachable, and named nowhere in the manifest. Reported but not
    /// counted as drift: nothing declared it, so nothing is out of place.
    pub strays: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memory: Vec<String>,
}

impl Row {
    fn in_place(&self) -> bool {
        self.want == self.have && self.held.is_none()
    }
}

impl Report {
    /// Whether everything declared is where it was declared to be. `status`
    /// exits on this, so a script can gate on one command.
    pub fn converged(&self) -> bool {
        self.steps
            .iter()
            .chain(&self.devices)
            .all(Row::in_place)
    }

    /// What `status` exits with: 0 converged, 2 drifted, 4 when every step and
    /// `ours`, the device this shell drives, are in place and only others drifted.
    pub fn exit(&self, ours: Option<&str>) -> i32 {
        if self.converged() {
            return 0;
        }

        let ready = |name: &str| {
            self.steps.iter().all(Row::in_place)
                && self.devices.iter().any(|r| r.name == name && r.in_place())
        };

        match ours {
            Some(name) if ready(name) => 4,
            _ => 2,
        }
    }
}

/// The level a device is at without asking the project anything: how far up the
/// ladder the survey alone can prove it.
///
/// A simulator is drivable the moment its host lists it as booted — there is no
/// transport to attach — so `online` is `attached` for one and only `booted`
/// for anything that speaks adb.
fn reached(view: &View) -> Option<Level> {
    match &view.reach {
        Reach::Attached { .. } => Some(Level::Attached),
        Reach::Online if view.device.platform.is_adb() => Some(Level::Booted),
        Reach::Online => Some(Level::Attached),
        Reach::Unauthorized { .. } => Some(Level::Booted),
        Reach::Off | Reach::Known | Reach::Offline { .. } => None,
    }
}

fn below(level: Level) -> String {
    match level {
        Level::Booted => "off".to_string(),
        other => format!("below {}", other.as_str()),
    }
}

fn shown(level: Option<Level>) -> String {
    match level {
        Some(level) => level.as_str().to_string(),
        None => "off".to_string(),
    }
}

/// The one declared device a name refers to.
///
/// Exact before fuzzy, and an ambiguous name is refused rather than picked
/// from: `up` runs unattended, so there is nobody to answer a prompt, and
/// converging the wrong emulator is worse than saying which two were meant.
fn pick<'a>(views: &'a [View], name: &str) -> Result<&'a View> {
    if let Some(view) = views.iter().find(|v| v.device.is(name)) {
        return Ok(view);
    }

    let fuzzy: Vec<&View> = views.iter().filter(|v| v.device.matches(name)).collect();

    match fuzzy.len() {
        1 => Ok(fuzzy[0]),
        0 => Err(anyhow!("no device named {name}")),
        _ => Err(anyhow!(
            "{name} matches {}; name one of them exactly",
            fuzzy
                .iter()
                .map(|v| v.device.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The build entry that covers a device, by the platform it runs.
fn build_for(project: &Project, platform: Platform) -> Option<(&str, &Build)> {
    let key = build_key(platform);

    project.manifest.build.get(key).map(|build| (key, build))
}

/// What a manifest and an sdk call the platform, which is not what the survey
/// calls it: an emulator builds and installs exactly as a handset does, so
/// `[build.android]` covers both and `expo run:android` is the same command.
fn build_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Android | Platform::Emulator => "android",
        Platform::Ios | Platform::Simulator => "ios",
    }
}

/// Runs a declared command with the project's directory as its cwd and the
/// device it is for named in the environment.
///
/// The directory arrives as a positional rather than interpolated, so a path
/// with a space in it is a path rather than two arguments — and its `~` is
/// expanded where the tilde means something, which is the host's home and not
/// this machine's.
pub(crate) fn scripted(body: &str) -> String {
    format!(
        r#"dir=$1
case "$dir" in "~"|"~/"*) dir="$HOME${{dir#\~}}" ;; esac
cd "$dir" || {{ echo "no such directory: $dir" >&2; exit 1; }}
export PHONE_SERIAL="$2" PHONE_ID="$3" PHONE_PLATFORM="$4" PHONE_KIND="$5" PHONE_DEVICE="$6"
{body}"#
    )
}

/// The six positionals every declared command is handed. A project-wide step is
/// for no device in particular and gets them empty rather than a fake.
///
/// Two of them say what the device is, because the two answers are used for
/// different things: PHONE_PLATFORM is `android` or `ios`, which is what a build
/// command is spelled with, and PHONE_KIND keeps the distinction the survey
/// draws, for a script that only wants `--device` on real hardware.
pub(crate) fn args<'a>(dir: &'a str, view: Option<&'a View>) -> [&'a str; 6] {
    let Some(view) = view else {
        return [dir, "", "", "", "", ""];
    };

    [
        dir,
        view.reach.serial().unwrap_or(""),
        &view.device.id,
        build_key(view.device.platform),
        view.device.platform.as_str(),
        &view.device.label,
    ]
}

fn failed(what: &str, status: &Status, said: &str) -> anyhow::Error {
    let said = said.trim();
    let tail = match said.is_empty() {
        true => String::new(),
        false => format!(": {}", said.lines().next_back().unwrap_or(said)),
    };

    match status {
        Status::Code(code) => anyhow!("{what} exited {code}{tail}"),
        Status::Garbled(v) => anyhow!("{what} ended with an unreadable status '{v}'{tail}"),
        Status::Missing => anyhow!("{what} never reported a status{tail}"),
    }
}

/// Where a project's declared commands run, and under what name the host
/// running them remembers what it has already done. Every step asks those two
/// questions and no step answers them differently, so they travel together
/// rather than as three more parameters on each.
struct Site {
    at: Where,
    dir: String,
    key: String,
}

impl Site {
    /// The site, and what its host remembers having built there.
    ///
    /// One round trip for both. Neither answer is worth a session of its own,
    /// and `status` pays for them before it can start on any device.
    async fn of(at: Where, dir: String) -> Result<(Self, Stamps)> {
        let ran = at
            .exec(&scripted(OPEN), &args(&dir, None), Duration::from_secs(20))
            .await
            .with_context(|| format!("opening {dir} on {}", at.label()))?;

        if !ran.ok() {
            return Err(failed("locating the project", &ran.status, &ran.said));
        }

        let read = ran.text();
        let mut lines = read.splitn(3, '\n');
        let key = lines.next().unwrap_or_default().to_string();
        let state = lines.next().unwrap_or_default();
        let stamps = Stamps::read(at.clone(), state, lines.next().unwrap_or_default().as_bytes());

        Ok((Site { at, dir, key }, stamps))
    }

    #[cfg(test)]
    fn here(dir: &str, key: &str) -> Self {
        Site {
            at: Where::Here,
            dir: dir.to_string(),
            key: key.to_string(),
        }
    }
}

/// What the host calls the tree, where it keeps its ledger, and the ledger.
///
/// The name comes first because the stamps are filed under it, and it has to be
/// one every machine driving the host agrees on: `~/p` and `/Users/x/p` are one
/// checkout and not two, and the path this machine sees is not a name that host
/// would recognise at all. A ledger that is not there yet is the normal first
/// run, not a failure.
const OPEN: &str = r#"pwd -P
state="${XDG_STATE_HOME:-$HOME/.local/state}/phone"
printf '%s\n' "$state"
cat "$state/stamps.json" 2>/dev/null
exit 0"#;

/// The hash of what a step says its inputs are, or `None` when it declares no
/// way of telling — which makes it stale forever, and is worth being explicit
/// about rather than treating as fresh.
async fn probe(site: &Site, view: Option<&View>, stale: Option<&str>) -> Result<Option<String>> {
    let Some(stale) = stale else {
        return Ok(None);
    };

    let ran = site
        .at
        .exec(&scripted(stale), &args(&site.dir, view), PROBE_TIMEOUT)
        .await
        .with_context(|| format!("asking {} what changed", site.at.label()))?;

    if !ran.ok() {
        return Err(failed("the freshness check", &ran.status, &ran.said));
    }

    Ok(Some(stamps::hash(&ran.stdout)))
}

/// Runs a step, live, and remembers what it was built from.
///
/// The stamp is written after the work rather than before it, so a build that
/// fails halfway is stale on the next run. It is also written from the hash
/// taken *before* the work, because a build that touches the tree it was
/// fingerprinted from would otherwise stamp itself as already out of date.
async fn perform(
    site: &Site,
    view: Option<&View>,
    step: &str,
    run: &str,
    hash: Option<String>,
    stamps: &Mutex<Stamps>,
    tag: Option<&str>,
) -> Result<()> {
    let status = site
        .at
        .stream(&scripted(run), &args(&site.dir, view), tag)
        .await
        .with_context(|| format!("running {step} on {}", site.at.label()))?;

    if status != Status::Code(0) {
        return Err(failed(step, &status, ""));
    }

    if let Some(hash) = hash {
        let mut stamps = stamps.lock().await;

        stamps.set(&site.key, step, &hash);
        stamps.save().await?;
    }

    Ok(())
}

/// What the last run stamped a step with, copied out rather than borrowed.
///
/// The ledger is one file shared by every device converging at once, and the
/// work between reading it and writing it is minutes long. Holding it open for
/// that would make the devices take turns again.
async fn stamped(stamps: &Mutex<Stamps>, key: &str, step: &str) -> Option<String> {
    stamps.lock().await.get(key, step).map(str::to_string)
}

/// Whether a `Task` still needs doing, and the hash that will stamp it if it
/// does.
async fn due(
    site: &Site,
    view: Option<&View>,
    task: &Task,
    prior: Option<&str>,
    force: bool,
) -> Result<(bool, Option<String>)> {
    let hash = probe(site, view, task.stale.as_deref()).await?;

    let fresh = !force
        && match (&hash, prior) {
            (Some(now), Some(then)) => now == then,
            // a step with no way of telling whether it is done is never done
            _ => false,
        };

    Ok((!fresh, hash))
}

/// Whether something is listening on a port on a machine.
///
/// Two tools, because neither is everywhere: `lsof` is on macOS by default and
/// often absent from a stripped Linux, `nc` the other way round. Asked over
/// HTTP it would only answer for an HTTP bundler, and nothing here knows that
/// the bundler speaks HTTP.
const LISTENING: &str = r#"port=$1
if command -v lsof >/dev/null 2>&1; then
  lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1 && exit 0
fi
if command -v nc >/dev/null 2>&1; then
  nc -z 127.0.0.1 "$port" >/dev/null 2>&1 && exit 0
fi
exit 1"#;

async fn listening(at: &Where, port: u16) -> bool {
    at.exec(LISTENING, &[&port.to_string()], Duration::from_secs(20))
        .await
        .map(|ran| ran.ok())
        .unwrap_or(false)
}

/// Starts the bundler detached, with its output going to a log rather than to
/// this terminal.
///
/// It has to outlive this process: `up` returns and the devices keep talking to
/// it. Its stdio goes to the log because a background job holding the ssh
/// session's stdout would keep the session — and so this command — from ever
/// finishing.
const START_BUNDLER: &str = r#"log=${TMPDIR:-/tmp}/phone-bundler-$7.log
: > "$log"
nohup sh -c "$8" >>"$log" 2>&1 &
echo "$log""#;

/// Kills whatever holds the port, rather than a pid remembered from when it was
/// started. The bundler is a shell that spawns a runtime that spawns workers,
/// so the pid that was backgrounded is rarely the one holding the socket.
const STOP_BUNDLER: &str = r#"port=$1
pids=$(lsof -ti tcp:"$port" -sTCP:LISTEN 2>/dev/null)
[ -z "$pids" ] && exit 0
kill $pids 2>/dev/null
exit 0"#;

/// Brings the project's steps and devices to what the manifest declares.
pub async fn up(reg: &mut Registry, project: &Project, opts: &Opts) -> Result<()> {
    let (site, ledger) = Site::of(Where::of(project.host()), project.dir()).await?;
    let declared = project.platforms(opts.profile.as_deref())?;

    if declared.is_empty() {
        bail!(
            "{} builds for no platform and has no [android] or [ios] section",
            crate::project::FILE
        );
    }

    // behind a lock from here on: the devices converge together below and each
    // one stamps its own build in the same ledger
    let stamps = Mutex::new(ledger);

    if opts.rebuild {
        let mut ledger = stamps.lock().await;

        ledger.forget(&site.key);
        ledger.save().await?;
    }

    eprintln!(
        "phone: {} on {}",
        project.name(),
        Where::of(project.host()).label()
    );

    // the tree first: a build against half-installed dependencies fails in a
    // way that reads as a broken project rather than as a missing step
    if let Some(task) = &project.manifest.deps {
        let prior = stamped(&stamps, &site.key, "deps").await;
        let (due_now, hash) = due(&site, None, task, prior.as_deref(), opts.rebuild).await?;

        match due_now {
            true => {
                eprintln!("phone: deps");
                perform(&site, None, "deps", &task.run, hash, &stamps, None).await?;
            }
            false => eprintln!("phone: deps are current"),
        }
    }

    if let Some(bundler) = &project.manifest.bundler {
        match listening(&site.at, bundler.port).await {
            true => eprintln!(
                "phone: bundler already up on {}:{}",
                site.at.label(),
                bundler.port
            ),
            false => {
                let log = start_bundler(&site, project, bundler.port, &bundler.run).await?;

                eprintln!(
                    "phone: bundler up on {}:{} ({log})",
                    site.at.label(),
                    bundler.port
                );
            }
        }
    }

    // one survey for every device, rather than one each: it is the most
    // expensive thing a command does and it answers the same question for all
    let views = survey(reg).await;
    reg.save()?;

    let (mut wanted, views) = allocate(reg, project, views, declared, opts.target.as_deref()).await?;

    // boots run together because they are minutes of waiting each and touch
    // nothing in common; everything after this needs the registry, and so is
    // one at a time
    let cold: Vec<&Climb> = wanted
        .iter()
        .filter(|(_, spec, view)| spec.state >= Level::Booted && reached(view).is_none())
        .collect();

    let booted: Vec<String> = cold.iter().map(|(name, _, _)| name.to_string()).collect();
    let started: Vec<Device> = cold
        .iter()
        .map(|(_, _, view)| view.device.clone())
        .collect();

    if !cold.is_empty() {
        let booting: Vec<&Device> = cold.iter().map(|(_, _, view)| &view.device).collect();

        memory::admit(reg, &views, &booting, opts.over_budget).await?;

        let (rep, drain) = crate::reporter();

        let booting = cold
            .iter()
            .map(|(_, _, view)| actions::boot(&view.device, opts.timeout, &rep));

        let outcomes = futures_util::future::join_all(booting).await;

        drop(rep);
        drain.await;

        for outcome in outcomes {
            eprintln!("phone: {}", outcome?);
        }
    }

    // whatever just came up has a transport now that it did not have during the
    // first survey, and everything below reads the serial off it
    let views = match cold.is_empty() {
        true => views,
        false => {
            let views = discover::arrived(reg, &started).await;
            reg.save()?;

            views
        }
    };

    for (name, _, view) in &mut wanted {
        *view = refind(&views, view)?;

        if booted.iter().any(|b| b == name) && view.device.platform == Platform::Emulator {
            for changed in actions::quiet(&view.server, &view.device).await? {
                eprintln!("phone: {name} {changed}");
            }
        }
    }

    // the attach writes to the registry, so it is the one rung a device cannot
    // climb beside another. It is also seconds rather than the minutes
    // everything above it takes.
    for (name, spec, view) in &mut wanted {
        *view = attach(reg, name, spec, view.clone()).await?;
    }

    let _beats = claim(&wanted).await?;

    let lanes = lanes(&wanted);
    let tagged = lanes.len() > 1;

    let climbing = lanes
        .iter()
        .map(|lane| converge(project, &site, lane, &stamps, tagged));

    all_of(futures_util::future::join_all(climbing).await)
}

/// One result out of work that ran together, carrying every complaint.
///
/// Not the first error and out: the whole point of running the devices at once
/// is that a run reports on all of them, and stopping at the first would leave
/// the rest looking like they worked.
fn all_of(outcomes: Vec<Result<()>>) -> Result<()> {
    let failures: Vec<String> = outcomes
        .into_iter()
        .filter_map(|outcome| outcome.err())
        .map(|err| format!("{err:#}"))
        .collect();

    match failures.is_empty() {
        true => Ok(()),
        false => bail!("{}", failures.join("\n")),
    }
}

/// A device, the profile its platform declares, and where it currently is.
type Climb = (String, Spec, View);

async fn allocate(
    reg: &mut Registry,
    project: &Project,
    mut views: Vec<View>,
    declared: Vec<(&'static str, Spec)>,
    pin: Option<&str>,
) -> Result<(Vec<Climb>, Vec<View>)> {
    let pinned = pin.map(|name| pick(&views, name).cloned()).transpose()?;

    if let Some(p) = pinned
        .as_ref()
        .filter(|p| declared.iter().all(|(os, _)| *os != p.device.platform.os()))
    {
        bail!(
            "{} runs {}, which this run does not build for",
            p.device.label,
            p.device.platform.os()
        );
    }

    let mut wanted = Vec::new();

    for (os, spec) in declared {
        let view = match pinned.as_ref().filter(|p| p.device.platform.os() == os) {
            Some(p) => {
                lease::hold(p).await?;
                p.clone()
            }
            None => seated(reg, project, &mut views, os).await?,
        };

        eprintln!("phone: {os} runs on {}", view.device.label);
        wanted.push((view.device.label.clone(), spec, view));
    }

    Ok((wanted, views))
}

async fn seated(
    reg: &mut Registry,
    project: &Project,
    views: &mut Vec<View>,
    os: &'static str,
) -> Result<View> {
    for _ in 0..3 {
        let view = match alloc::choose(views, reg, Some(project), &[os], true).await? {
            alloc::Chosen::Use(view) | alloc::Chosen::Boot(view) => view,
            alloc::Chosen::Clone(source) => cloned(reg, views, &source).await?,
        };

        match lease::hold(&view).await {
            Err(e) if e.is::<crate::Refused>() => continue,
            held => return held.map(|_| view),
        }
    }

    Err(crate::Refused(format!("every {os} device this run chose was taken first; run it again")).into())
}

async fn cloned(reg: &mut Registry, views: &mut Vec<View>, source: &View) -> Result<View> {
    let name = alloc::clone_name(views, source);

    eprintln!(
        "phone: {}",
        actions::clone(&source.device, &source.reach, &name).await?
    );

    *views = survey(reg).await;
    reg.save()?;

    views
        .iter()
        .find(|v| v.device.host == source.device.host && v.device.is(&name))
        .cloned()
        .ok_or_else(|| anyhow!("{name} was cloned but no survey lists it"))
}

fn refind(views: &[View], was: &View) -> Result<View> {
    views
        .iter()
        .find(|v| v.device.id == was.device.id)
        .or_else(|| {
            views
                .iter()
                .find(|v| v.device.host == was.device.host && v.device.is(&was.device.label))
        })
        .cloned()
        .ok_or_else(|| anyhow!("{} dropped out of the survey", was.device.label))
}

async fn claim(wanted: &[Climb]) -> Result<Vec<lease::Beat>> {
    let mut refused = Vec::new();

    for (name, _, view) in wanted {
        if let lease::Got::Busy(holder) = lease::acquire(view, lease::take()).await? {
            refused.push(format!(
                "{name} is held by {}; its holder's `phone release` frees it, `--take` takes it",
                holder.describe(model::now())
            ));
        }
    }

    if !refused.is_empty() {
        return Err(crate::Refused(refused.join("\n")).into());
    }

    for (_, _, view) in wanted {
        pids::forget(&view.device.id);
    }

    Ok(wanted.iter().map(|(_, _, view)| lease::heartbeat(view)).collect())
}

async fn hold_on(view: &View) -> Option<(Lease, bool)> {
    reached(view)?;

    let book = Book::fetch(&actions::where_of(&view.device)).await.ok()?;

    match book.standing(&view.device, actions::running(&view.reach)) {
        Standing::Mine(lease) => Some((lease, true)),
        Standing::Held(lease) => Some((lease, false)),
        Standing::Free(_) => None,
    }
}

/// The devices grouped by the build they share.
///
/// Two devices on one platform run the same build in the same directory, and a
/// second gradle or xcodebuild started there while the first is going fails on
/// a lock it does not hold — so they take turns. Two platforms share nothing
/// but the tree they read, so they do not: an iPhone stops waiting out an
/// android build it has no part in.
fn lanes(wanted: &[Climb]) -> Vec<Vec<&Climb>> {
    let mut lanes: BTreeMap<&'static str, Vec<&Climb>> = BTreeMap::new();

    for climb in wanted {
        lanes
            .entry(build_key(climb.2.device.platform))
            .or_default()
            .push(climb);
    }

    lanes.into_values().collect()
}

/// One lane's devices, in the order they were declared.
async fn converge(
    project: &Project,
    site: &Site,
    lane: &[&Climb],
    stamps: &Mutex<Stamps>,
    tagged: bool,
) -> Result<()> {
    let mut outcomes = Vec::new();

    for (name, spec, view) in lane.iter().copied() {
        // only once more than one lane is running: a single build streaming to
        // a terminal it has to itself reads better without a name in front of
        // every line of it
        let tag = tagged.then(|| format!("{name}: "));

        // the next device is still tried: they share a build, not a fate, and
        // an app that would not start on one says nothing about the other
        outcomes.push(
            raise(project, site, name, spec, view, stamps, tag.as_deref())
                .await
                .with_context(|| format!("bringing up {name}")),
        );
    }

    all_of(outcomes)
}

/// The transport, which is the one thing a device needs that is written down.
async fn attach(reg: &mut Registry, name: &str, spec: &Spec, view: View) -> Result<View> {
    if spec.state < Level::Attached
        || reached(&view) >= Some(Level::Attached)
        || !view.device.platform.is_adb()
    {
        return Ok(view);
    }

    let (rep, drain) = crate::reporter();
    let opts = connect::Opts::default();
    let res = connect::connect(reg, &view.server, &view.device, &opts, &rep).await;

    drop(rep);
    drain.await;

    res.with_context(|| format!("attaching to {name}"))?;

    // the transport it just came up on is what every later step names
    let view = refind(&survey(reg).await, &view)?;

    reg.save()?;

    Ok(view)
}

async fn start_bundler(site: &Site, project: &Project, port: u16, run: &str) -> Result<String> {
    let name = project.name();
    let mut args = args(&site.dir, None).to_vec();

    args.push(&name);
    args.push(run);

    let ran = site
        .at
        .exec(&scripted(START_BUNDLER), &args, Duration::from_secs(30))
        .await
        .with_context(|| format!("starting the bundler on {}", site.at.label()))?;

    if !ran.ok() {
        return Err(failed("the bundler", &ran.status, &ran.said));
    }

    let log = ran.text();
    let began = std::time::Instant::now();

    while began.elapsed() < BUNDLER_TIMEOUT {
        if listening(&site.at, port).await {
            return Ok(log);
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    bail!(
        "the bundler never started answering on port {port}; see {log} on {}",
        site.at.label()
    )
}

/// Takes one attached device the rest of the way to what it was declared to be.
async fn raise(
    project: &Project,
    site: &Site,
    name: &str,
    spec: &Spec,
    view: &View,
    stamps: &Mutex<Stamps>,
    tag: Option<&str>,
) -> Result<()> {
    let want = spec.state;

    if want < Level::Ready {
        eprintln!("phone: {name} is {}", shown(reached(view)));

        return Ok(());
    }

    let restart = ready(view, spec, name).await?;

    if want < Level::Prepared {
        eprintln!("phone: {name} is ready");

        return Ok(());
    }

    prepared(project, site, name, view, stamps, tag, restart).await
}

/// The forwards and the settings, which are what turn an attached device into
/// one an app can be driven on. Both are Android's; a simulator shares its
/// host's loopback already and takes its configuration from the host.
async fn ready(view: &View, spec: &Spec, name: &str) -> Result<bool> {
    if let Some(why) = exempt(view, config::get().physical(&view.device)) {
        if !spec.reverse.is_empty() || !spec.settings.is_empty() {
            eprintln!("phone: {name} {why}");
        }

        return Ok(false);
    }

    if !spec.reverse.is_empty() {
        let open = actions::reversed(&view.server, &view.device).await?;

        for port in &spec.reverse {
            if open.iter().any(|(device, _)| device == port) {
                continue;
            }

            let what = actions::Reverse::Open {
                device: *port,
                host: *port,
            };

            eprintln!(
                "phone: {}",
                actions::reverse(&view.server, &view.device, what).await?
            );
        }
    }

    let want = spec.settings.each();

    let changed = actions::settings(&view.server, &view.device, &want).await?;

    for setting in &changed {
        eprintln!("phone: {name} {setting}");
    }

    Ok(!changed.is_empty())
}

/// The build, and the app in front.
///
/// Two questions, not one: the artifact can be current and the device still not
/// carry it — a fresh emulator, a wipe, an uninstall between runs. Asking both
/// is what makes `up` right on a device it has never seen and cheap on one it
/// converged a minute ago.
async fn prepared(
    project: &Project,
    site: &Site,
    name: &str,
    view: &View,
    stamps: &Mutex<Stamps>,
    tag: Option<&str>,
    restart: bool,
) -> Result<()> {
    let Some((platform, build)) = build_for(project, view.device.platform) else {
        bail!(
            "{name} wants a build and {} declares none for {}",
            crate::project::FILE,
            view.device.platform
        );
    };

    let step = format!("build.{platform}");
    let task = build.task();

    let prior = stamped(stamps, &site.key, &step).await;
    let (stale, hash) = due(site, Some(view), &task, prior.as_deref(), false).await?;
    let here = apps::installed(&view.server, &view.device, &build.app).await?;

    if stale || !here {
        eprintln!(
            "phone: {step} for {name} ({})",
            match here {
                false => format!("{} is not installed", build.app),
                true => "the sources moved".to_string(),
            }
        );

        perform(site, Some(view), &step, &build.run, hash, stamps, tag).await?;
    }

    // an app reads some settings once at launch: Reanimated takes reduced motion
    // from transition_animation_scale and keeps it until the process dies
    if restart && here {
        eprintln!(
            "phone: {}, so it starts under the new settings",
            apps::stop(&view.server, &view.device, &build.app).await?
        );
    }

    eprintln!(
        "phone: {}",
        apps::launch(&view.server, &view.device, &build.app, &build.args, &[]).await?
    );

    if let Ok(app) = apps::app_id(&build.app) {
        lease::installed(view, app).await?;
    }

    // after the launch and not instead of it: a simulator delivers a url to a
    // running app and silently drops one aimed at an app that is not, so the
    // icon has to come first and the link second
    if let Some(url) = &build.open {
        eprintln!(
            "phone: {}",
            apps::open(&view.server, &view.device, url).await?
        );
    }

    loaded(view, name, build.open.as_deref()).await
}

/// What a development client shows in place of the app when the bundle does not
/// arrive; the launch and the link both succeed either way.
const FAILED: [&str; 5] = [
    "There was a problem loading the project",
    "Could not connect to development server",
    "Unable to load script",
    "Unable to resolve module",
    LAUNCHER,
];
/// The dev launcher's home, where a link sent before it was ready is dropped.
const LAUNCHER: &str = "Development servers";
const LOADING: [&str; 3] = ["Bundling", "Downloading", "Loading from"];
const LOAD_LIMIT: Duration = Duration::from_secs(120);
const STUCK_LIMIT: Duration = Duration::from_secs(10);

fn patience(said: &str, resend: bool) -> Duration {
    if resend && said.to_lowercase().contains(&LAUNCHER.to_lowercase()) {
        Duration::ZERO
    } else {
        STUCK_LIMIT
    }
}

#[derive(Debug, PartialEq)]
enum Front {
    Loading,
    Shown,
    Failed(String),
}

fn front(nodes: &[crate::a11y::Node]) -> Front {
    let named = |marks: &[&str]| {
        nodes
            .iter()
            .map(|n| n.name())
            .find(|name| {
                let name = name.to_lowercase();
                marks.iter().any(|m| name.contains(&m.to_lowercase()))
            })
            .map(str::to_string)
    };

    match (named(&FAILED), named(&LOADING)) {
        (Some(said), _) => Front::Failed(said),
        (None, Some(_)) => Front::Loading,
        (None, None) => Front::Shown,
    }
}

async fn loaded(view: &View, name: &str, mut open: Option<&str>) -> Result<()> {
    if view.device.platform.is_hosted() {
        return Ok(());
    }

    let t = crate::target_of(view, None).await?;
    let began = std::time::Instant::now();
    let mut failing = None;
    let mut shown = false;

    loop {
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let read = crate::a11y::dump(&t).await;
        let now = read.as_ref().map_or(Front::Loading, |screen| front(&screen.nodes));
        let unread = read.err().and_then(|e| e.downcast::<crate::a11y::NotIdle>().ok());

        match &now {
            Front::Shown if shown => {
                eprintln!("phone: {name} is showing the app");
                return Ok(());
            }
            Front::Failed(said)
                if failing.get_or_insert_with(std::time::Instant::now).elapsed()
                    >= patience(said, open.is_some()) =>
            {
                let Some(url) = open.take() else {
                    bail!(
                        "{name} is not showing the app, it is stuck on \"{said}\"; \
                         is the bundler up and reachable from the device?"
                    );
                };

                eprintln!(
                    "phone: {name} is on \"{said}\", sending the link again: {}",
                    apps::open(&view.server, &view.device, url).await?
                );
                failing = None;
            }
            Front::Failed(_) => {}
            _ => failing = None,
        }

        shown = now == Front::Shown;

        if began.elapsed() >= LOAD_LIMIT {
            match &unread {
                Some(stuck) => bail!("{name} is up, but its screen cannot be read: {stuck}"),
                None => bail!("{name} did not finish loading the app in {}s", LOAD_LIMIT.as_secs()),
            }
        }
    }
}

type Seat = (&'static str, Spec, Option<View>);

fn seats(
    project: &Project,
    profile: Option<&str>,
    views: &[View],
    books: &[Book],
    only: Option<&str>,
) -> Result<Vec<Seat>> {
    let pinned = only.map(|name| pick(views, name).cloned()).transpose()?;
    let last = lease::sticky::last();

    let seated: Vec<Seat> = project
        .platforms(profile)?
        .into_iter()
        .filter(|(os, _)| pinned.as_ref().is_none_or(|p| p.device.platform.os() == *os))
        .map(|(os, spec)| {
            let view = pinned
                .clone()
                .or_else(|| ours(views, books, os, last.as_ref()));

            (os, spec, view)
        })
        .collect();

    if let Some(p) = pinned.as_ref().filter(|_| seated.is_empty()) {
        bail!("{} runs {}, which {} does not declare", p.device.label, p.device.platform.os(), crate::project::FILE);
    }

    Ok(seated)
}

fn ours(views: &[View], books: &[Book], os: &str, last: Option<&lease::sticky::Last>) -> Option<View> {
    let on = |v: &&View| v.device.platform.os() == os;
    let mine = |v: &&View| {
        lease::book_of(books, &v.device).is_some_and(|b| {
            matches!(b.standing(&v.device, actions::running(&v.reach)), Standing::Mine(_))
        })
    };

    views
        .iter()
        .filter(on)
        .filter(mine)
        .max_by_key(|v| actions::running(&v.reach))
        .or_else(|| {
            let last = last?;

            views
                .iter()
                .filter(on)
                .find(|v| v.device.id == last.device && config::get().pick(&v.device).sticks())
        })
        .cloned()
}

fn held(books: &[Book]) -> impl Fn(&View) -> bool + '_ {
    |v| {
        lease::book_of(books, &v.device).is_none_or(|b| {
            !matches!(b.standing(&v.device, actions::running(&v.reach)), Standing::Free(_))
        })
    }
}

/// Reads what is there against what was declared, without changing any of it.
pub async fn status(
    reg: &mut Registry,
    project: &Project,
    profile: Option<&str>,
    only: Option<&str>,
) -> Result<Report> {
    let (site, stamps) = Site::of(Where::of(project.host()), project.dir()).await?;
    let views = survey(reg).await;
    reg.save()?;

    let books = lease::books(&views).await;
    let seated = seats(project, profile, &views, &books, only)?;
    let mut steps = Vec::new();

    if let Some(task) = &project.manifest.deps {
        let prior = stamps.get(&site.key, "deps");
        let (stale, _) = due(&site, None, task, prior, false).await?;

        steps.push(Row {
            name: "deps".to_string(),
            platform: None,
            want: "current".to_string(),
            have: match stale {
                true => "stale".to_string(),
                false => "current".to_string(),
            },
            held: None,
            note: match (stale, task.stale.is_some()) {
                (true, false) => "declares no freshness check, so it always runs".to_string(),
                (true, true) => task.run.clone(),
                (false, _) => String::new(),
            },
        });
    }

    if let Some(bundler) = &project.manifest.bundler {
        let up = listening(&site.at, bundler.port).await;

        steps.push(Row {
            name: "bundler".to_string(),
            platform: None,
            want: "up".to_string(),
            have: match up {
                true => "up".to_string(),
                false => "down".to_string(),
            },
            held: None,
            // the port alone: whoever computed this report is on the machine
            // holding the bundler, so naming it "this machine" here would name
            // the wrong one to a client reading the report from elsewhere. The
            // host belongs to the report, not to the row.
            note: bundler.port.to_string(),
        });
    }

    // each row ends in the same fingerprint over the whole tree that a build
    // measures itself against, and no two of them read anything the others
    // write. Asked one at a time this is the slower half of the pair a test
    // script runs before every e2e round.
    let devices = futures_util::future::join_all(
        seated
            .iter()
            .map(|(os, spec, view)| row(project, &site, &stamps, os, spec, view.as_ref())),
    )
    .await;

    let mine: Vec<&View> = seated.iter().filter_map(|(_, _, view)| view.as_ref()).collect();

    let mut memory = memory::report(reg, &views, &memory::hosts_of(mine.iter().copied())).await;

    for view in mine
        .iter()
        .filter(|v| v.device.platform == Platform::Emulator)
    {
        let Some(serial) = view.reach.serial() else {
            continue;
        };

        if let Some(guest) = memory::guest(&view.server, serial).await {
            if guest.heavy() {
                memory.push(memory::swapping(&view.device.label, &guest));
            }
        }
    }

    Ok(Report {
        project: project.name(),
        host: project.host().map(str::to_string),
        steps,
        devices,
        strays: match only {
            Some(_) => Vec::new(),
            None => strays(&views, held(&books)),
        },
        memory,
    })
}

async fn row(
    project: &Project,
    site: &Site,
    stamps: &Stamps,
    os: &str,
    spec: &Spec,
    view: Option<&View>,
) -> Row {
    let want = spec.state;

    let Some(view) = view else {
        return Row {
            name: os.to_string(),
            platform: Some(os.to_string()),
            want: want.as_str().to_string(),
            have: "none".to_string(),
            note: "this agent holds no device for it; `phone up` allocates one".to_string(),
            held: None,
        };
    };
    let name = view.device.label.as_str();

    let mut note = String::new();
    let mut have = reached(view);

    // each rung is only asked about once the one below it holds, because the
    // question does not mean anything otherwise: a device with no transport
    // cannot be asked what it has installed
    if have >= Some(Level::Attached) && want >= Level::Ready {
        match settled(view, spec).await {
            Ok(true) => have = Some(Level::Ready),
            Ok(false) => note = "its forwards or settings have drifted".to_string(),
            Err(e) => note = e.to_string(),
        }
    }

    if have >= Some(Level::Ready) && want >= Level::Prepared {
        match current(project, site, stamps, view).await {
            Ok(true) => have = Some(Level::Prepared),
            Ok(false) => note = "its build is behind the sources or is not installed".to_string(),
            Err(e) => note = e.to_string(),
        }
    }

    if have.is_none() {
        note = below(want);
    }

    // whatever rung it is on: a device that is prepared for this project and
    // being driven by another is the one case the rest of the row cannot see
    let hold = hold_on(view).await;

    // ours is worth printing but is not drift
    let held = hold
        .as_ref()
        .filter(|(_, mine)| !mine)
        .map(|(holder, _)| holder.describe(model::now()));

    if let Some((holder, mine)) = &hold {
        let whose = match mine {
            true => format!("yours ({})", model::ago(holder.since)),
            false => format!("held by {}", holder.describe(model::now())),
        };

        note = match note.is_empty() {
            true => whose,
            false => format!("{whose}; {note}"),
        };
    }

    Row {
        name: name.to_string(),
        platform: Some(view.device.platform.as_str().to_string()),
        want: want.as_str().to_string(),
        have: shown(have),
        note,
        held,
    }
}

/// Whether a device's forwards and settings are already what was declared.
fn exempt(view: &View, physical: bool) -> Option<String> {
    if !view.device.platform.is_adb() {
        return Some(format!(
            "is a {}, so `reverse` and `settings` do not apply to it",
            view.device.platform
        ));
    }

    physical.then(|| "is a physical device; `reverse` and `settings` apply to emulators only".to_string())
}

async fn settled(view: &View, spec: &Spec) -> Result<bool> {
    if exempt(view, config::get().physical(&view.device)).is_some() {
        return Ok(true);
    }

    if !spec.reverse.is_empty() {
        let open = actions::reversed(&view.server, &view.device).await?;

        if !spec
            .reverse
            .iter()
            .all(|port| open.iter().any(|(device, _)| device == port))
        {
            return Ok(false);
        }
    }

    Ok(
        actions::drifted(&view.server, &view.device, &spec.settings.each())
            .await?
            .is_empty(),
    )
}

/// Whether a device carries a build no older than the sources. Read-only, which
/// is why the stamp is compared rather than written.
async fn current(project: &Project, site: &Site, stamps: &Stamps, view: &View) -> Result<bool> {
    let Some((platform, build)) = build_for(project, view.device.platform) else {
        bail!("no [build.*] entry covers {}", view.device.platform);
    };

    if !apps::installed(&view.server, &view.device, &build.app).await? {
        return Ok(false);
    }

    let hash = probe(site, Some(view), build.stale.as_deref()).await?;

    Ok(
        match (hash, stamps.get(&site.key, &format!("build.{platform}"))) {
            (Some(now), Some(then)) => now == then,
            _ => false,
        },
    )
}

/// Puts a project's devices back where it found them: the bundler stopped, the
/// forwards dropped, whatever was started shut down.
///
/// A handset is never one of those. It was already on when `up` ran, `up` did
/// not turn it on, and turning off the phone somebody is holding to make a
/// teardown symmetrical is not a trade worth making.
pub async fn down(reg: &mut Registry, project: &Project) -> Result<()> {
    let (site, _) = Site::of(Where::of(project.host()), project.dir()).await?;
    let at = &site.at;
    let declared = project.every_platform();

    if let Some(bundler) = &project.manifest.bundler {
        let ran = at
            .exec(
                STOP_BUNDLER,
                &[&bundler.port.to_string()],
                Duration::from_secs(20),
            )
            .await?;

        match ran.ok() {
            true => eprintln!("phone: bundler on {}:{} stopped", at.label(), bundler.port),
            false => eprintln!("phone: could not stop the bundler: {}", ran.said),
        }
    }

    let views = survey(reg).await;
    reg.save()?;

    let books = lease::books(&views).await;

    for (os, spec) in declared {
        let Some(view) = ours(&views, &books, os, None) else {
            continue;
        };

        retire(&view, &spec).await;
        lease::release(&view).await?;
        eprintln!("phone: released {}", view.device.label);
    }

    Ok(())
}

async fn retire(view: &View, spec: &Spec) {
    let name = &view.device.label;

    if !actions::running(&view.reach) {
        eprintln!("phone: {name} is already off");

        return;
    }

    if matches!(view.device.platform, Platform::Android | Platform::Ios) {
        eprintln!("phone: {name} is a handset, so it is left running");

        return;
    }

    // while a transport remains, and only this manifest's ports: another
    // agent's `--take` may have left its own on the same device
    if view.device.platform.is_adb() {
        for port in &spec.reverse {
            let what = actions::Reverse::Close { device: *port };
            let _ = actions::reverse(&view.server, &view.device, what).await;
        }
    }

    match actions::stop(&view.device, &view.reach).await {
        Ok(said) => eprintln!("phone: {said}"),
        Err(e) => eprintln!("phone: {name}: {e}"),
    }
}

/// An emulator or simulator that is running and held by no agent. Not an
/// error: the point is to say so, since it is memory on its host that nobody
/// is using.
///
/// Handsets are left out. One is attached because somebody plugged it in or
/// paired it, which is a deliberate act rather than something left running, and
/// reporting it on every `status` would train the reader to skip the line.
pub fn strays(views: &[View], held: impl Fn(&View) -> bool) -> Vec<String> {
    views
        .iter()
        // the same reading of "running" the rest of this module uses: a
        // simulator has no transport to attach over and would never qualify
        .filter(|v| reached(v) >= Some(Level::Attached))
        .filter(|v| matches!(v.device.platform, Platform::Emulator | Platform::Simulator))
        .filter(|v| !held(v))
        .map(|v| v.device.label.clone())
        .collect()
}

pub fn print(report: &Report) {
    let mut out = Vec::new();
    let _ = write(report, &mut out);

    crate::calls::out(&String::from_utf8_lossy(&out));

    // nothing to do about a closed stdout that printing an error would not also
    // hit; `status` still leaves through its exit code
    let _ = std::io::Write::write_all(&mut std::io::stdout(), &out);
}

/// Split from `print` so the table can be read back in a test. Everything the
/// reader needs is in the report, including which machine answered.
fn write(report: &Report, out: &mut impl std::io::Write) -> std::io::Result<()> {
    // Named only when the run happened somewhere else. A report computed here
    // has no host to name, and a line saying so on every `status` is noise.
    if let Some(host) = &report.host {
        writeln!(out, "{} on {host}", report.project)?;
    }

    let rows: Vec<&Row> = report.steps.iter().chain(&report.devices).collect();

    let width = |f: fn(&Row) -> &str| rows.iter().map(|r| f(r).len()).max().unwrap_or(0);

    let name = width(|r| &r.name).max(6);
    let want = width(|r| &r.want).max(4);
    let have = width(|r| &r.have).max(4);

    for row in rows {
        let mark = match row.in_place() {
            true => " ",
            false => "!",
        };

        writeln!(
            out,
            "{mark} {:name$}  {:want$} -> {:have$}  {}",
            row.name, row.want, row.have, row.note
        )?;
    }

    for label in &report.strays {
        writeln!(
            out,
            "  {label:name$}  {:want$}    {:have$}  running, held by no agent",
            "", ""
        )?;
    }

    for line in &report.memory {
        writeln!(out, "{line}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Device, Platform, Reach, View};

    fn view(label: &str, platform: Platform, reach: Reach) -> View {
        View::new(Device::new(label, label, platform), reach)
    }

    fn attached(label: &str) -> View {
        view(
            label,
            Platform::Emulator,
            Reach::Attached {
                serial: "emulator-5554".to_string(),
                wireless: false,
            },
        )
    }

    #[test]
    fn a_physical_device_never_takes_the_project_profile() {
        let handset = view("faraday", Platform::Android, Reach::Online);
        let config = config::Config::default();

        assert!(exempt(&handset, config.physical(&handset.device)).is_some_and(|why| why.contains("emulators only")));
        assert_eq!(exempt(&attached("pixel"), config.physical(&attached("pixel").device)), None);
        assert!(exempt(&attached("pixel"), true).is_some(), "kind = \"physical\" wins over the platform");
        assert!(exempt(&view("iPhone 17", Platform::Simulator, Reach::Online), false).is_some());
    }

    /// The rung a survey alone can prove, which is not the same question for a
    /// device with a transport and one without.
    #[test]
    fn a_simulator_is_attached_the_moment_its_host_lists_it() {
        assert_eq!(
            reached(&view("iPhone 17", Platform::Simulator, Reach::Online)),
            Some(Level::Attached)
        );
        assert_eq!(
            reached(&view("pixel", Platform::Emulator, Reach::Online)),
            Some(Level::Booted),
            "an emulator that is running without a transport is only booted"
        );
        assert_eq!(reached(&attached("pixel")), Some(Level::Attached));
        assert_eq!(
            reached(&view("pixel", Platform::Emulator, Reach::Off)),
            None
        );
        assert_eq!(
            reached(&view(
                "f",
                Platform::Android,
                Reach::Offline { last_seen: None }
            )),
            None
        );
    }

    /// An unattended run has nobody to answer a picker, so a name that could
    /// mean two devices has to end the run rather than pick one.
    #[test]
    fn an_ambiguous_name_is_refused_rather_than_chosen_from() {
        let views = [attached("pixel_7-api36"), attached("pixel_7-api35")];

        assert!(pick(&views, "pixel_7").is_err());
        assert_eq!(
            pick(&views, "pixel_7-api36").unwrap().device.label,
            "pixel_7-api36"
        );
        assert!(pick(&views, "nothing").is_err());
    }

    /// A name that is a prefix of another is what `use` and `-t` settle with a
    /// prompt; here the exact spelling has to win outright.
    #[test]
    fn an_exact_name_beats_the_longer_one_it_is_inside_of() {
        let views = [attached("pixel"), attached("pixel_7-api36")];

        assert_eq!(pick(&views, "pixel").unwrap().device.label, "pixel");
    }

    #[test]
    fn a_build_is_chosen_by_the_platform_the_device_runs() {
        let manifest = Project::parse(
            "[build.android]\napp = \"a\"\nrun = \"x\"\n[build.ios]\napp = \"b\"\nrun = \"y\"\n",
        )
        .unwrap();

        let project = Project {
            root: "/tmp/p".into(),
            manifest,
        };

        assert_eq!(
            build_for(&project, Platform::Emulator).unwrap().0,
            "android"
        );
        assert_eq!(build_for(&project, Platform::Android).unwrap().0, "android");
        assert_eq!(build_for(&project, Platform::Simulator).unwrap().0, "ios");
        assert_eq!(build_for(&project, Platform::Ios).unwrap().0, "ios");
    }

    /// The manifest's `dir` is written the way it is typed into a shell, and a
    /// `~` in it means the home of the machine the command runs on.
    #[test]
    fn the_project_directory_is_a_positional_and_its_tilde_is_the_hosts() {
        let script = scripted("pwd");

        assert!(script.contains("dir=$1"), "{script}");
        assert!(script.contains("$HOME"), "{script}");
        assert!(
            !script.contains("cd ~"),
            "the tilde must not be expanded before it is sent"
        );
    }

    #[test]
    fn a_project_wide_step_names_no_device() {
        assert_eq!(args("/tmp/p", None), ["/tmp/p", "", "", "", "", ""]);

        let view = attached("pixel");
        let bound = args("/tmp/p", Some(&view));

        assert_eq!(bound[0], "/tmp/p");
        assert_eq!(bound[1], "emulator-5554");
        assert_eq!(bound[3], "android", "what a build command is spelled with");
        assert_eq!(bound[4], "emu", "and what the survey calls the same device");
        assert_eq!(bound[5], "pixel");
    }

    /// The bug this closes: a lane stopped at its first failure, so a run that
    /// broke two devices complained about one and left the other looking like
    /// it had converged.
    #[test]
    fn work_that_ran_together_reports_every_failure() {
        let outcomes = vec![
            Err::<(), _>(anyhow!("no space left")).context("bringing up pixel_7-api36"),
            Ok(()),
            Err::<(), _>(anyhow!("the app would not start")).context("bringing up iPhone 17"),
        ];

        let complaint = all_of(outcomes).unwrap_err().to_string();

        assert!(
            complaint.contains("pixel_7-api36: no space left"),
            "the context a failure was wrapped in belongs in the report: {complaint}"
        );
        assert!(
            complaint.contains("iPhone 17: the app would not start"),
            "the second failure is not hidden by the first: {complaint}"
        );
        assert!(all_of(vec![Ok(()), Ok(())]).is_ok());
    }

    /// The two halves of what makes a run concurrent: devices that would run
    /// the same build take turns, and devices that would not, do not.
    #[test]
    fn devices_are_grouped_by_the_build_they_share() {
        let spec = Spec::default();

        let wanted: Vec<Climb> = vec![
            ("pixel".into(), spec.clone(), attached("pixel")),
            (
                "iPhone 17".into(),
                spec.clone(),
                view("iPhone 17", Platform::Simulator, Reach::Online),
            ),
            ("nexus".into(), spec, attached("nexus")),
        ];

        let grouped: Vec<Vec<&str>> = lanes(&wanted)
            .iter()
            .map(|lane| lane.iter().map(|(name, _, _)| name.as_str()).collect())
            .collect();

        assert_eq!(
            grouped,
            [vec!["pixel", "nexus"], vec!["iPhone 17"]],
            "the emulators share a lane, in the order they were declared"
        );
    }

    /// The bug this closes: the stamps were filed under the path the *client*
    /// saw, so `~/p` from this machine and `/Users/x/p` from a shell on the host
    /// were two projects, and neither could read what the other had built.
    #[tokio::test]
    async fn a_project_is_named_by_the_host_that_owns_it() {
        // the nix sandbox builds with no home at all, where a tilde expands to
        // nothing that can be entered
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .filter(|home| home.is_dir());

        if let Some(home) = home {
            assert_eq!(
                Site::of(Where::Here, "~".to_string()).await.unwrap().0.key,
                std::fs::canonicalize(&home).unwrap().display().to_string(),
                "a tilde is the host's, and the host is the one that expands it"
            );
        }

        let real = std::env::temp_dir().join(format!("phone-named-{}", std::process::id()));
        let link = std::env::temp_dir().join(format!("phone-link-{}", std::process::id()));

        std::fs::create_dir_all(&real).unwrap();
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&real, &link).unwrap();

        assert_eq!(
            Site::of(Where::Here, link.display().to_string())
                .await
                .unwrap()
                .0
                .key,
            std::fs::canonicalize(&real).unwrap().display().to_string(),
            "two names for one checkout are one name, or it gets built twice"
        );

        std::fs::remove_file(&link).unwrap();
        std::fs::remove_dir_all(&real).unwrap();
    }

    /// Not knowing whether a step is done is not the same as knowing it is, and
    /// treating it as done would skip the work forever.
    #[tokio::test]
    async fn a_step_with_no_freshness_check_is_never_current() {
        let stamps = Stamps::default();
        let task = Task {
            stale: None,
            run: "true".to_string(),
        };

        let (stale, hash) = due(
            &Site::here("/tmp", "/tmp/p"),
            None,
            &task,
            stamps.get("/tmp/p", "deps"),
            false,
        )
        .await
        .unwrap();

        assert!(stale);
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn a_step_is_current_only_while_what_it_prints_stays_the_same() {
        let dir = std::env::temp_dir();
        let key = "/tmp/p";

        let task = Task {
            stale: Some("echo one".to_string()),
            run: "true".to_string(),
        };

        let mut stamps = Stamps::default();
        let site = Site::here(&dir.display().to_string(), key);

        let (stale, hash) = due(&site, None, &task, stamps.get(key, "deps"), false)
            .await
            .unwrap();

        assert!(stale, "nothing has been stamped yet");
        stamps.set(key, "deps", hash.as_deref().unwrap());

        let (stale, _) = due(&site, None, &task, stamps.get(key, "deps"), false)
            .await
            .unwrap();

        assert!(!stale, "the same print means the same inputs");

        let moved = Task {
            stale: Some("echo two".to_string()),
            run: "true".to_string(),
        };

        let (stale, _) = due(&site, None, &moved, stamps.get(key, "deps"), false)
            .await
            .unwrap();

        assert!(stale, "a different print means different inputs");
    }

    /// `--rebuild` is the escape hatch for when the freshness check is right
    /// and the answer is still wrong.
    #[tokio::test]
    async fn a_forced_step_is_due_however_fresh_it_looks() {
        let mut stamps = Stamps::default();
        let task = Task {
            stale: Some("echo one".to_string()),
            run: "true".to_string(),
        };

        let site = Site::here(&std::env::temp_dir().display().to_string(), "/p");

        let (_, hash) = due(&site, None, &task, stamps.get("/p", "deps"), false)
            .await
            .unwrap();

        stamps.set("/p", "deps", hash.as_deref().unwrap());

        let (stale, _) = due(&site, None, &task, stamps.get("/p", "deps"), true)
            .await
            .unwrap();

        assert!(stale);
    }

    /// A freshness check that cannot run is not a reason to rebuild: it is a
    /// broken manifest, and rebuilding would hide it behind twenty minutes of
    /// gradle every single time.
    #[tokio::test]
    async fn a_freshness_check_that_fails_ends_the_run() {
        let site = Site::here(&std::env::temp_dir().display().to_string(), "/p");
        let err = probe(&site, None, Some("exit 3"))
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("exited 3"), "{err}");
    }

    #[tokio::test]
    async fn a_step_that_cannot_reach_the_project_directory_says_which_one() {
        let err = probe(
            &Site::here("/nowhere/at/all", "/p"),
            None,
            Some("echo hello"),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("/nowhere/at/all"), "{err}");
    }

    fn row_of(name: &str, want: &str, have: &str) -> Row {
        Row {
            name: name.to_string(),
            platform: None,
            want: want.to_string(),
            have: have.to_string(),
            note: String::new(),
            held: None,
        }
    }

    /// `status` exits on this, so a script gates on one command rather than on
    /// parsing the table.
    #[test]
    fn a_report_has_converged_only_when_every_row_has() {
        let mut report = Report {
            strays: Vec::new(),
            memory: Vec::new(),
            project: "p".to_string(),
            host: None,
            steps: vec![row_of("deps", "current", "current")],
            devices: vec![row_of("pixel", "prepared", "prepared")],
        };

        assert!(report.converged());

        report.devices.push(row_of("iphone", "ready", "off"));

        assert!(!report.converged());
    }

    #[test]
    fn drift_only_beside_the_driven_device_exits_apart_from_drift_on_it() {
        let mut report = Report {
            strays: Vec::new(),
            memory: Vec::new(),
            project: "p".to_string(),
            host: None,
            steps: vec![row_of("deps", "current", "current")],
            devices: vec![row_of("pixel", "prepared", "prepared")],
        };

        assert_eq!(report.exit(Some("pixel")), 0);

        report.devices.push(row_of("iphone", "prepared", "off"));

        assert_eq!(report.exit(Some("pixel")), 4);
        assert_eq!(report.exit(Some("iphone")), 2);
        assert_eq!(report.exit(Some("elsewhere")), 2);
        assert_eq!(report.exit(None), 2);

        report.steps.push(row_of("bundler", "up", "down"));

        assert_eq!(report.exit(Some("pixel")), 2);
    }

    fn shown(report: &Report) -> String {
        let mut out = Vec::new();

        write(report, &mut out).unwrap();

        String::from_utf8(out).unwrap()
    }

    /// The bug this closes: the host that answered was named inside a row, by a
    /// side that had just been told to forget its own name — so `status` on a
    /// delegated project reported the bundler as running on "this machine",
    /// meaning the one it was not running on.
    #[test]
    fn a_report_that_came_from_elsewhere_names_where_it_came_from() {
        let mut report = Report {
            strays: Vec::new(),
            memory: Vec::new(),
            project: "sample-app".to_string(),
            host: Some("mac".to_string()),
            steps: vec![Row {
                note: "8081".to_string(),
                ..row_of("bundler", "up", "down")
            }],
            devices: Vec::new(),
        };

        let table = shown(&report);

        assert!(table.starts_with("sample-app on mac\n"), "{table}");
        assert!(!table.contains("this machine"), "{table}");

        report.host = None;

        assert!(!shown(&report).contains(" on "), "{}", shown(&report));
    }

    #[test]
    fn a_running_emulator_no_agent_holds_is_named_rather_than_ignored() {
        let views = [attached("pixel_7-api36"), attached("leftover-avd")];

        assert_eq!(
            strays(&views, |v| v.device.label == "pixel_7-api36"),
            ["leftover-avd"]
        );
    }

    #[test]
    fn a_device_that_is_not_attached_is_not_a_stray() {
        let views = [view("pixel", Platform::Emulator, Reach::Off)];

        assert!(strays(&views, |_| false).is_empty());
    }

    /// A simulator never reaches `attached` — there is no transport to attach
    /// over — so asking that question of one would hide every loose simulator.
    #[test]
    fn a_running_simulator_nobody_declared_is_a_stray() {
        let views = [view("iPhone 17", Platform::Simulator, Reach::Online)];

        assert_eq!(
            strays(&views, |_| false),
            ["iPhone 17"]
        );
    }

    fn screen(texts: &[&str]) -> Vec<crate::a11y::Node> {
        let nodes: String = texts
            .iter()
            .map(|t| format!(r#"<node text="{t}" class="android.widget.TextView" bounds="[0,0][10,10]"/>"#))
            .collect();

        crate::a11y::parse(&format!("<hierarchy>{nodes}</hierarchy>")).unwrap()
    }

    #[test]
    fn a_dev_client_that_never_got_its_bundle_is_not_the_app() {
        assert_eq!(
            front(&screen(&["Could not connect to development server."])),
            Front::Failed("Could not connect to development server.".into())
        );
        assert_eq!(front(&screen(&["Development servers"])), Front::Failed("Development servers".into()));
        assert_eq!(front(&screen(&["DEVELOPMENT SERVERS"])), Front::Failed("DEVELOPMENT SERVERS".into()));
        assert_eq!(front(&screen(&["Bundling 42%"])), Front::Loading);
        assert_eq!(front(&screen(&["Operaciones en curso"])), Front::Shown);
    }

    #[test]
    fn only_the_launcher_home_gets_the_link_again_at_once() {
        assert_eq!(patience("DEVELOPMENT SERVERS", true), Duration::ZERO);
        assert_eq!(patience("DEVELOPMENT SERVERS", false), STUCK_LIMIT);
        assert_eq!(patience("Could not connect to development server.", true), STUCK_LIMIT);
        assert_eq!(patience("Unable to load script", true), STUCK_LIMIT);
    }
}
