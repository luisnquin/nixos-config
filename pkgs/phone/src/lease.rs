use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::{self, Agent, Harness};
use crate::model::{self, Device, Platform, Unix, View, AVD_PREFIX};
use crate::project::Project;
use crate::ssh::{Status, Where};
use crate::usage::Usage;
use crate::{actions, config, registry, stamps, up};

const TIMEOUT: Duration = Duration::from_secs(20);
const CONFLICT: i32 = 75;
const RETRIES: usize = 6;
pub const BOOT: i64 = 300;
const CACHED: i64 = 60;
const HANDOFF: i64 = 180;

const LEDGER: &str = r#"state="${XDG_STATE_HOME:-$HOME/.local/state}/phone"
mkdir -p "$state" || exit 1
f="$state/leases.json"
lock="$state/leases.lock"
code=0
if [ "$1" != "-" ]; then
  n=0
  until mkdir "$lock" 2>/dev/null; do
    m=$(stat -c %Y "$lock" 2>/dev/null || stat -f %m "$lock" 2>/dev/null || echo 0)
    [ $(( $(date +%s) - m )) -gt 5 ] && mv "$lock" "$lock.$$" 2>/dev/null && rmdir "$lock.$$"
    n=$((n + 1))
    [ "$n" -gt 100 ] && { echo "phone: $lock is stuck" >&2; exit 1; }
    sleep 0.1 2>/dev/null || sleep 1
  done
  trap 'rmdir "$lock" 2>/dev/null' EXIT INT TERM
  if [ "$(cat "$f" 2>/dev/null | cksum)" = "$1" ]; then
    tmp="$f.$$.tmp"
    printf '%s' "$2" > "$tmp" && mv "$tmp" "$f" || exit 1
    if [ -n "$3" ]; then
      u="$state/usage.tsv"
      { cat "$u" 2>/dev/null; } | awk -F '\t' -v OFS='\t' -v d="$3" -v t="$4" -v at="$5" '
NF < 4 { next }
$1 == d && $2 == t { $3 = at; $4 = $4 + 1; hit = 1 }
{ print }
END { if (!hit) print d, t, at, 1 }' > "$u.$$.tmp" && mv "$u.$$.tmp" "$u"
    fi
  else
    code=75
  fi
fi
me=$(uname -n)
printf '%s\n%s\n' "$state" "$me"
cat "$f" 2>/dev/null | cksum
printf '@body\n'
cat "$f" 2>/dev/null
printf '\n@dead\n'
grep -o '"host":"[^"]*","pid":[0-9]*,"start":"[^"]*"' "$f" 2>/dev/null | while IFS= read -r p; do
  h=${p#*\"host\":\"}; h=${h%%\"*}
  [ "$h" = "$me" ] || continue
  pid=${p#*\"pid\":}; pid=${pid%%,*}
  st=${p#*\"start\":\"}; st=${st%\"}
  if [ -r "/proc/$pid/stat" ]; then
    now=$(sed 's/.*) //' "/proc/$pid/stat" | cut -d' ' -f20)
  else
    now=$(ps -o lstart= -p "$pid" 2>/dev/null | awk '{$1=$1; gsub(/ /, "_"); print}')
  fi
  [ "$now" = "$st" ] || echo "dead $pid $st"
done
printf '@usage\n'
cat "$state/usage.tsv" 2>/dev/null
exit $code"#;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub agent: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<Harness>,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    pub since: Unix,
    pub last_seen: Unix,
    pub ttl: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<String>,
}

impl Lease {
    pub fn of(me: &Agent, ttl: Duration, now: Unix) -> Self {
        Lease {
            agent: me.id.clone(),
            label: me.label.clone(),
            harness: Some(me.harness),
            host: me.host.clone(),
            pid: me.pid,
            start: me.start.clone(),
            app: None,
            since: now,
            last_seen: now,
            ttl: ttl.as_secs() as i64,
            took: None,
        }
    }

    pub fn owned_by(&self, me: &Agent) -> bool {
        let parent = me.id.split_once('/').is_some_and(|(session, _)| session == self.agent);

        self.held_by(me) || parent && self.host == me.host
    }

    pub fn held_by(&self, me: &Agent) -> bool {
        self.agent == me.id && self.host == me.host
    }

    pub fn idle(&self, now: Unix) -> i64 {
        (now - self.last_seen).max(0)
    }

    pub fn due(&self, now: Unix, running: bool) -> bool {
        self.idle(now) * 4 >= self.ttl || !running && self.idle(now) * 2 >= BOOT
    }

    pub fn is_lock(&self) -> bool {
        self.agent.starts_with(LOCK)
    }

    pub fn describe(&self, now: Unix) -> String {
        if self.is_lock() {
            return format!("{} (locked {} ago)", self.label, span(now - self.since));
        }

        let harness = self.harness.map(|h| format!("{h:?}").to_lowercase());

        format!(
            "{} ({}{}, idle {})",
            self.label,
            harness.map(|h| format!("{h} on ")).unwrap_or_default(),
            self.host,
            span(self.idle(now))
        )
    }
}

pub fn span(secs: i64) -> String {
    match secs {
        ..=59 => format!("{}s", secs.max(0)),
        60..=3599 => format!("{}m", secs / 60),
        _ => format!("{}h{}m", secs / 3600, secs % 3600 / 60),
    }
}

const LOCK: &str = "lock:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub by: String,
    pub host: String,
    pub since: Unix,
}

impl Lock {
    fn lease(&self) -> Lease {
        Lease {
            agent: format!("{LOCK}{}", self.by),
            label: self.by.clone(),
            harness: Some(Harness::User),
            host: self.host.clone(),
            pid: None,
            start: None,
            app: None,
            since: self.since,
            last_seen: self.since,
            ttl: i64::MAX,
            took: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    pub v: u32,
    #[serde(default)]
    pub leases: BTreeMap<String, Lease>,
    #[serde(default)]
    pub installed: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub displays: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub locks: BTreeMap<String, Lock>,
}

#[derive(Deserialize)]
struct Old {
    project: String,
    since: Unix,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    session: Option<String>,
}

impl Ledger {
    pub fn parse(body: &str) -> Result<Self> {
        if body.trim().is_empty() {
            return Ok(Ledger::default());
        }

        let Value::Object(map) = serde_json::from_str::<Value>(body).context("leases.json is not JSON")? else {
            bail!("leases.json is not a JSON object");
        };

        match map.contains_key("v") {
            true => serde_json::from_value(Value::Object(map)).context("leases.json does not parse as a v2 ledger"),
            false => Ok(Ledger::migrate(map)),
        }
    }

    fn migrate(map: serde_json::Map<String, Value>) -> Self {
        let leases = map
            .into_iter()
            .filter_map(|(key, held)| Some((key, serde_json::from_value::<Old>(held).ok()?)))
            .map(|(key, old)| {
                let lease = Lease {
                    agent: format!("legacy:{}", old.session.as_deref().unwrap_or(&old.project)),
                    label: agent::short(&old.project),
                    harness: None,
                    host: old.host.unwrap_or_default(),
                    pid: None,
                    start: None,
                    app: None,
                    since: old.since,
                    last_seen: old.since,
                    ttl: config::TTL.as_secs() as i64,
                    took: None,
                };

                (key, lease)
            })
            .collect();

        Ledger {
            v: 2,
            leases,
            installed: BTreeMap::new(),
            displays: BTreeSet::new(),
            locks: BTreeMap::new(),
        }
    }

    pub fn body(&self) -> String {
        let mut out = self.clone();
        out.v = 2;

        serde_json::to_string(&out).expect("a ledger serializes")
    }

    pub fn get(&self, key: &Key) -> Option<(&String, &Lease)> {
        std::iter::once(&key.id)
            .chain(&key.legacy)
            .find_map(|id| self.leases.get_key_value(id))
    }

    pub fn put(&mut self, key: &Key, lease: Lease) {
        self.drop_key(key);
        self.leases.insert(key.id.clone(), lease);
    }

    pub fn drop_key(&mut self, key: &Key) -> Option<Lease> {
        let had = self.leases.remove(&key.id);

        key.legacy
            .iter()
            .fold(had, |had, id| had.or(self.leases.remove(id)))
    }

    fn prune(&mut self, judge: &Judge) {
        self.leases.retain(|_, lease| judge.valid(lease, true));
    }
}

pub struct Judge<'a> {
    pub now: Unix,
    pub book_host: &'a str,
    pub dead: &'a BTreeSet<(u32, String)>,
    pub here: &'a str,
    pub alive: fn(u32, &str) -> bool,
    pub blind: bool,
    pub verb: fn(&str, &str) -> Option<Unix>,
}

impl Judge<'_> {
    pub fn valid(&self, lease: &Lease, running: bool) -> bool {
        let idle = lease.idle(self.now);

        idle < lease.ttl && (running || idle < BOOT) && self.living(lease)
    }

    /// A subagent shares its session's pid, so its last verb on the device is
    /// what tells an idle subagent from one still driving.
    pub fn handed_up(&self, lease: &Lease, me: &Agent, device: &str) -> bool {
        let child = lease.agent.split_once('/').is_some_and(|(session, _)| session == me.id);

        child && lease.host == me.host && (self.verb)(&lease.agent, device).is_some_and(|at| self.now - at >= HANDOFF)
    }

    fn living(&self, lease: &Lease) -> bool {
        let (Some(pid), Some(start)) = (lease.pid, lease.start.as_deref()) else {
            return true;
        };

        if self.blind && lease.host == self.here {
            return true;
        }

        if lease.host == self.book_host {
            return !self.dead.contains(&(pid, start.to_string()));
        }

        lease.host != self.here || (self.alive)(pid, start)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Mine(Lease),
    Free(Option<Lease>),
    Held(Lease),
}

pub fn standing(ledger: &Ledger, key: &Key, judge: &Judge, me: &Agent, running: bool) -> Standing {
    if let Some(lock) = ledger.locks.get(&key.id).filter(|_| me.harness != Harness::User) {
        return Standing::Held(lock.lease());
    }

    let Some((_, lease)) = ledger.get(key) else {
        return Standing::Free(None);
    };

    match (judge.valid(lease, running), lease.owned_by(me) || judge.handed_up(lease, me, &key.id)) {
        (true, true) => Standing::Mine(lease.clone()),
        (true, false) => Standing::Held(lease.clone()),
        (false, _) => Standing::Free(Some(lease.clone())),
    }
}

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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Book {
    #[serde(skip)]
    pub at: Where,
    dir: String,
    pub host: String,
    sum: String,
    pub ledger: Ledger,
    dead: BTreeSet<(u32, String)>,
    pub usage: Usage,
    #[serde(default)]
    read: Unix,
    #[serde(skip)]
    pub fresh: bool,
}

impl Book {
    fn parse(at: &Where, text: &str) -> Result<Self> {
        let framed = || {
            let mut head = text.splitn(4, '\n');
            let (dir, host, sum, rest) = (head.next()?, head.next()?, head.next()?, head.next()?);
            let (body, rest) = rest.strip_prefix("@body\n")?.split_once("\n@dead\n")?;

            Some((dir, host, sum, body, rest))
        };
        let (dir, host, sum, body, rest) = framed().context("the ledger script's output is not framed")?;
        let (dead, usage) = rest.split_once("@usage\n").unwrap_or((rest, ""));

        Ok(Book {
            at: at.clone(),
            dir: dir.to_string(),
            host: host.to_string(),
            sum: sum.trim().to_string(),
            ledger: Ledger::parse(body).with_context(|| format!("{dir}/leases.json on {host}"))?,
            dead: dead.lines().filter_map(dead_line).collect(),
            usage: Usage::parse(usage),
            read: model::now(),
            fresh: true,
        })
    }

    fn trusted(&self, now: Unix) -> bool {
        self.fresh || now - self.read < CACHED
    }

    pub async fn fetch(at: &Where) -> Result<Self> {
        let ran = at
            .exec(LEDGER, &["-"], TIMEOUT)
            .await
            .with_context(|| format!("reading the leases on {}", at.label()))?;

        let book = Self::parse(at, &String::from_utf8_lossy(&ran.stdout))
            .with_context(|| format!("the leases on {} came back unreadable: {}", at.label(), ran.said))?;

        book.cache();

        Ok(book)
    }

    pub async fn open(at: &Where) -> Result<Self> {
        match Self::cached(at) {
            Some(book) => Ok(book),
            None => Self::fetch(at).await,
        }
    }

    async fn commit(&self, ledger: &Ledger, stamp: &[String]) -> Result<Result<Self, Self>> {
        let body = ledger.body();
        let mut args = vec![self.sum.as_str(), body.as_str()];
        args.extend(stamp.iter().map(String::as_str));

        let ran = self
            .at
            .exec(LEDGER, &args, TIMEOUT)
            .await
            .with_context(|| format!("writing the leases on {}", self.at.label()))?;

        let book = Self::parse(&self.at, &String::from_utf8_lossy(&ran.stdout));

        match (ran.status, book) {
            (Status::Code(0), Ok(book)) => Ok(Ok(book.cached_now())),
            (Status::Code(CONFLICT), Ok(book)) => Ok(Err(book.cached_now())),
            (_, Err(e)) => Err(e.context(format!("could not write the leases on {}: {}", self.at.label(), ran.said))),
            _ => bail!("could not write the leases on {}: {}", self.at.label(), ran.said),
        }
    }

    fn cached_now(self) -> Self {
        self.cache();
        self
    }

    fn path(at: &Where) -> PathBuf {
        let name = at.host().map(|h| stamps::hash(h.as_bytes())).unwrap_or_else(|| "local".into());

        registry::state_dir().join("ledgers").join(format!("{name}.json"))
    }

    fn cached(at: &Where) -> Option<Self> {
        let body = std::fs::read(Self::path(at)).ok()?;
        let mut book: Book = serde_json::from_slice(&body).ok()?;

        book.at = at.clone();

        Some(book)
    }

    fn cache(&self) {
        let path = Self::path(&self.at);
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));

        let wrote = path.parent().map(std::fs::create_dir_all).transpose().is_ok()
            && std::fs::write(&tmp, serde_json::to_vec(self).unwrap_or_default()).is_ok();

        if wrote {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    pub fn judge(&self, now: Unix) -> Judge<'_> {
        Judge {
            now,
            book_host: &self.host,
            dead: &self.dead,
            here: &agent::me().host,
            alive: agent::alive,
            blind: agent::blind(),
            verb: verbs::at,
        }
    }

    pub fn standing(&self, device: &Device, running: bool) -> Standing {
        standing(&self.ledger, &key(device), &self.judge(model::now()), agent::me(), running)
    }
}

fn dead_line(line: &str) -> Option<(u32, String)> {
    let mut words = line.strip_prefix("dead ")?.splitn(2, ' ');

    Some((words.next()?.parse().ok()?, words.next()?.to_string()))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Take {
    #[default]
    Respect,
    Override,
}

impl From<bool> for Take {
    fn from(take: bool) -> Self {
        match take {
            true => Take::Override,
            false => Take::Respect,
        }
    }
}

static TAKE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_take(take: bool) {
    TAKE.store(take, std::sync::atomic::Ordering::Relaxed);
}

pub fn take() -> Take {
    TAKE.load(std::sync::atomic::Ordering::Relaxed).into()
}

pub async fn hold(view: &View) -> Result<Lease> {
    let take = take();
    let ttl = config::get().ttl(&view.device);

    if take == Take::Respect && config::get().physical(&view.device) {
        if let Some(age) = actions::last_human(&view.server, &view.device).await.filter(|a| *a < ttl) {
            let book = Book::open(&actions::where_of(&view.device)).await?;
            let since = match book.standing(&view.device, actions::running(&view.reach)) {
                Standing::Mine(lease) => Some(model::now() - lease.last_seen),
                _ => None,
            };

            if handed_back(age, since) {
                return Err(crate::Refused(format!(
                    "{} was touched by hand {}s ago, so its owner has it; it frees {}s after the last touch, `--take` takes it now",
                    view.device.label,
                    age.as_secs(),
                    ttl.as_secs()
                ))
                .into());
            }
        }
    }

    match acquire(view, take).await? {
        Got::Held(lease) => Ok(lease),
        Got::Busy(lease) => Err(refusal(view, &lease).into()),
    }
}

#[derive(Debug)]
pub enum Got {
    Held(Lease),
    Busy(Lease),
}

pub async fn acquire(view: &View, take: Take) -> Result<Got> {
    let at = actions::where_of(&view.device);
    let mut book = Book::open(&at).await?;

    for _ in 0..RETRIES {
        match attempt(view, take, &book) {
            Step::Done(got) => return Ok(acted(view, got)),
            Step::Refresh => book = Book::fetch(&at).await?,
            Step::Write(ledger, lease, revert) => match book.commit(&ledger, &stamp(view, &book).await).await? {
                Ok(_) => {
                    if revert {
                        revert_display(view).await;
                    }

                    return Ok(acted(view, settled(view, lease)));
                }
                Err(newer) => book = newer,
            },
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

fn acted(view: &View, got: Got) -> Got {
    if let Got::Held(_) = got {
        verbs::stamp(&key(&view.device).id);
    }

    got
}

enum Step {
    Done(Got),
    Refresh,
    Write(Ledger, Lease, bool),
}

fn attempt(view: &View, take: Take, book: &Book) -> Step {
    let now = model::now();
    let me = agent::me();
    let key = key(&view.device);
    let judge = book.judge(now);
    let running = actions::running(&view.reach);
    let standing = standing(&book.ledger, &key, &judge, me, running);
    let revert = reverts(&standing, take, Moment::Acquire, book.ledger.displays.contains(&key.id));

    let lease = match standing {
        Standing::Mine(lease) if !lease.due(now, running) && book.trusted(now) => return Step::Done(Got::Held(lease)),
        Standing::Mine(_) | Standing::Held(_) if !book.fresh => return Step::Refresh,
        Standing::Held(lease) if stops(&lease, take) => return Step::Done(Got::Busy(lease)),
        Standing::Mine(lease) => Lease { last_seen: now, ..lease },
        Standing::Held(lease) => Lease {
            took: Some(format!("{} by {:?}", lease.label, me.harness).to_lowercase()),
            ..Lease::of(me, config::get().ttl(&view.device), now)
        },
        Standing::Free(_) => Lease::of(me, config::get().ttl(&view.device), now),
    };

    let mut ledger = book.ledger.clone();

    ledger.prune(&judge);
    ledger.put(&key, lease.clone());

    Step::Write(ledger, lease, revert)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    Acquire,
    Release,
}

/// A display override is the holder's alone: it is undone when the hold ends,
/// and by whoever next holds the device when the hold lapsed without that.
fn reverts(standing: &Standing, take: Take, moment: Moment, overridden: bool) -> bool {
    let refused = refuses(standing, take);
    let renewal = matches!(standing, Standing::Mine(_)) && moment == Moment::Acquire;

    overridden && !refused && !renewal
}

async fn revert_display(view: &View) {
    match actions::display_reset(&view.server, &view.device).await {
        Ok(()) => {
            eprintln!("phone: reset the display overrides left on {}", view.device.label);

            if let Err(e) = displayed(view, false).await {
                eprintln!("phone: {e:#}");
            }
        }
        Err(e) => eprintln!("phone: could not reset the display overrides on {}: {e:#}", view.device.label),
    }
}

pub async fn displayed(view: &View, on: bool) -> Result<()> {
    let at = actions::where_of(&view.device);
    let mut book = Book::fetch(&at).await?;
    let key = key(&view.device);

    for _ in 0..RETRIES {
        if book.ledger.displays.contains(&key.id) == on {
            return Ok(());
        }

        let mut ledger = book.ledger.clone();

        match on {
            true => ledger.displays.insert(key.id.clone()),
            false => ledger.displays.remove(&key.id),
        };

        match book.commit(&ledger, &[]).await? {
            Ok(_) => return Ok(()),
            Err(newer) => book = newer,
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

fn settled(view: &View, lease: Lease) -> Got {
    let me = agent::me();

    if let Some(took) = &lease.took {
        eprintln!("phone: took {} from {took}", view.device.label);
    }

    if me.shared {
        eprintln!(
            "phone: {} acquired as Claude session {}; PHONE_AGENT is unset, so every subagent of this session shares it",
            view.device.label, me.label
        );
    }

    sticky::remember(view, &lease);

    Got::Held(lease)
}

async fn stamp(view: &View, book: &Book) -> Vec<String> {
    let mine = book.ledger.get(&key(&view.device)).is_some_and(|(_, l)| l.owned_by(agent::me()));

    if mine {
        return Vec::new();
    }

    let tree = here_tree().await.unwrap_or_default();

    vec![key(&view.device).id, tree, model::now().to_string()]
}

pub async fn release(view: &View) -> Result<Option<Lease>> {
    let at = actions::where_of(&view.device);
    let mut book = Book::fetch(&at).await?;
    let mut reverted = false;

    for _ in 0..RETRIES {
        let key = key(&view.device);
        let standing = book.standing(&view.device, actions::running(&view.reach));
        let revert = reverts(&standing, take(), Moment::Release, book.ledger.displays.contains(&key.id));

        let had = match letting_go(standing, take()) {
            Some(Ok(lease)) => Some(lease),
            Some(Err(lease)) => return busy(view, lease),
            None if !revert => return Ok(None),
            None => None,
        };

        if revert && !reverted {
            reverted = reset_on_release(view).await;
        }

        let mut ledger = book.ledger.clone();
        ledger.drop_key(&key);

        if reverted {
            ledger.displays.remove(&key.id);
        }

        match book.commit(&ledger, &[]).await? {
            Ok(_) => return Ok(had.filter(|l| l.owned_by(agent::me()))),
            Err(newer) => book = newer,
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

async fn reset_on_release(view: &View) -> bool {
    match actions::display_reset(&view.server, &view.device).await {
        Ok(()) => {
            eprintln!("phone: reset the display overrides on {}", view.device.label);
            true
        }
        Err(e) => {
            eprintln!("phone: left the display overrides on {} for its next holder to reset: {e:#}", view.device.label);
            false
        }
    }
}

fn handed_back(touched: Duration, since_my_last_verb: Option<Unix>) -> bool {
    since_my_last_verb.is_none_or(|since| since >= touched.as_secs() as Unix)
}

fn letting_go(standing: Standing, take: Take) -> Option<Result<Lease, Lease>> {
    match (standing, take) {
        (Standing::Held(lease), _) if stops(&lease, take) => Some(Err(lease)),
        (Standing::Free(None), _) => None,
        (Standing::Mine(lease) | Standing::Held(lease) | Standing::Free(Some(lease)), _) => Some(Ok(lease)),
    }
}

fn busy(view: &View, lease: Lease) -> Result<Option<Lease>> {
    Err(refusal(view, &lease).into())
}

pub fn refusal(view: &View, lease: &Lease) -> crate::Refused {
    if lease.is_lock() {
        return locked(&view.device.label, lease);
    }

    crate::Refused(format!(
        "{} is held by {}; its holder's `phone release -t {}` frees it, `--take` takes it",
        view.device.label,
        lease.describe(model::now()),
        view.device.label
    ))
}

pub async fn guard(view: &View) -> Result<()> {
    let book = Book::fetch(&actions::where_of(&view.device)).await?;

    match book.standing(&view.device, actions::running(&view.reach)) {
        Standing::Held(lease) if stops(&lease, take()) => Err(refusal(view, &lease).into()),
        _ => Ok(()),
    }
}

pub async fn unlocked(view: &View) -> Result<()> {
    let book = Book::fetch(&actions::where_of(&view.device)).await?;

    match book.standing(&view.device, true) {
        Standing::Held(lease) if lease.is_lock() => Err(refusal(view, &lease).into()),
        _ => Ok(()),
    }
}

fn refuses(standing: &Standing, take: Take) -> bool {
    matches!(standing, Standing::Held(lease) if stops(lease, take))
}

fn stops(lease: &Lease, take: Take) -> bool {
    take == Take::Respect || lease.is_lock()
}

pub async fn lock(view: &View, on: bool) -> Result<bool> {
    let me = agent::me();

    if me.harness != Harness::User {
        return Err(crate::Refused(format!(
            "only a human shell may {} a device; ask the user to run `phone {} -t {}`",
            if on { "lock" } else { "unlock" },
            if on { "lock" } else { "unlock" },
            view.device.label
        ))
        .into());
    }

    let at = actions::where_of(&view.device);
    let mut book = Book::fetch(&at).await?;
    let key = key(&view.device);

    for _ in 0..RETRIES {
        let mut ledger = book.ledger.clone();
        let changed = match on {
            true if ledger.locks.contains_key(&key.id) => false,
            true => {
                let lock = Lock {
                    by: me.label.clone(),
                    host: me.host.clone(),
                    since: model::now(),
                };

                ledger.locks.insert(key.id.clone(), lock);
                ledger.drop_key(&key);
                true
            }
            false => ledger.locks.remove(&key.id).is_some(),
        };

        if !changed {
            return Ok(false);
        }

        match book.commit(&ledger, &[]).await? {
            Ok(_) => return Ok(true),
            Err(newer) => book = newer,
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

fn alarm(device: &str, lease: &Lease) {
    let me = agent::me();
    let body = format!(
        "{} ({:?} on {}) tried to use it; locked by {}",
        me.label, me.harness, me.host, lease.label
    )
    .to_lowercase();

    let _ = std::process::Command::new("notify-send")
        .args(["--app-name=phone", "--urgency=critical", "--", &format!("{device} is locked"), &body])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn locked(device: &str, lease: &Lease) -> crate::Refused {
    alarm(device, lease);

    crate::Refused(format!(
        "{device} is locked by {}: the user is using it and has been notified. Stop and ask them; `--take` does not lift a lock",
        lease.describe(model::now())
    ))
}

pub async fn here_tree() -> Option<String> {
    tree(&Project::here().ok().flatten()?).await.ok()
}

pub async fn release_all(views: &[View]) -> Result<Vec<String>> {
    let mut freed = Vec::new();

    for at in &hosts_of(views) {
        let Ok(mut book) = Book::fetch(at).await else {
            continue;
        };

        let mut reverted = BTreeSet::new();

        for _ in 0..RETRIES {
            let mine: Vec<String> = book
                .ledger
                .leases
                .iter()
                .filter(|(_, l)| l.held_by(agent::me()))
                .map(|(k, _)| k.clone())
                .collect();

            if mine.is_empty() {
                break;
            }

            revert_held(views, at, &mine, &book.ledger.displays, &mut reverted).await;

            let mut ledger = book.ledger.clone();
            ledger.leases.retain(|k, _| !mine.contains(k));
            ledger.displays.retain(|k| !reverted.contains(k));

            match book.commit(&ledger, &[]).await? {
                Ok(_) => {
                    freed.extend(mine.iter().map(|k| format!("{k} on {}", at.label())));
                    break;
                }
                Err(newer) => book = newer,
            }
        }
    }

    Ok(freed)
}

async fn revert_held(views: &[View], at: &Where, mine: &[String], displays: &BTreeSet<String>, reverted: &mut BTreeSet<String>) {
    let due: Vec<&String> = mine.iter().filter(|k| displays.contains(*k) && !reverted.contains(*k)).collect();

    for id in due {
        let view = views.iter().find(|v| actions::where_of(&v.device) == *at && key(&v.device).id == *id);

        if let Some(view) = view.filter(|v| v.reach.is_attached()) {
            if reset_on_release(view).await {
                reverted.insert(id.clone());
            }
        }
    }
}

pub async fn installed(view: &View, app: &str) -> Result<()> {
    let at = actions::where_of(&view.device);
    let mut book = Book::open(&at).await?;
    let key = key(&view.device);

    for _ in 0..RETRIES {
        let mut ledger = book.ledger.clone();

        ledger.installed.entry(key.id.clone()).or_default().insert(app.to_string());

        if let Some(lease) = ledger.leases.get_mut(&key.id).filter(|l| l.owned_by(agent::me())) {
            lease.app = Some(app.to_string());
        }

        match book.commit(&ledger, &[]).await? {
            Ok(_) => return Ok(()),
            Err(newer) => book = newer,
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

pub struct Beat(tokio::task::JoinHandle<()>);

impl Drop for Beat {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn heartbeat(view: &View) -> Beat {
    let view = view.clone();
    let every = cadence(config::get().ttl(&view.device));

    Beat(tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;

            let _ = acquire(&view, Take::Respect).await;
        }
    }))
}

fn cadence(ttl: Duration) -> Duration {
    (ttl / 4).min(Duration::from_secs(BOOT as u64 / 2)).max(Duration::from_secs(15))
}

pub async fn tree(project: &Project) -> Result<String> {
    let at = Where::of(project.host());
    let dir = project.dir();
    let ran = at
        .exec(&up::scripted("pwd -P"), &up::args(&dir, None), TIMEOUT)
        .await
        .with_context(|| format!("locating {dir} on {}", at.label()))?;

    if !ran.ok() {
        bail!("locating {dir} on {}: {}", at.label(), ran.said);
    }

    let tree = ran.text().trim().to_string();

    crate::calls::tree(&tree);

    Ok(tree)
}

pub fn hosts_of(views: &[View]) -> Vec<Where> {
    let mut hosts: Vec<Where> = Vec::new();

    for view in views {
        let at = actions::where_of(&view.device);

        if !hosts.contains(&at) {
            hosts.push(at);
        }
    }

    hosts
}

pub async fn books(views: &[View]) -> Vec<Book> {
    let opened = hosts_of(views).into_iter().map(|at| async move { Book::fetch(&at).await.ok() });

    futures_util::future::join_all(opened)
        .await
        .into_iter()
        .flatten()
        .collect()
}

pub fn book_of<'a>(books: &'a [Book], device: &Device) -> Option<&'a Book> {
    let at = actions::where_of(device);

    books.iter().find(|b| b.at == at)
}

pub fn holds(views: &[View], books: &[Book]) -> BTreeMap<String, Lease> {
    views
        .iter()
        .filter_map(|view| {
            let book = book_of(books, &view.device)?;

            if let Some(lock) = book.ledger.locks.get(&key(&view.device).id) {
                return Some((view.device.id.clone(), lock.lease()));
            }

            match book.standing(&view.device, actions::running(&view.reach)) {
                Standing::Mine(lease) | Standing::Held(lease) => Some((view.device.id.clone(), lease)),
                Standing::Free(_) => None,
            }
        })
        .collect()
}

pub mod sticky {
    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Last {
        pub device: String,
        pub label: String,
        pub seen: Unix,
    }

    pub fn last() -> Option<Last> {
        let body = std::fs::read(agent::file("agents", &agent::me().id)).ok()?;

        serde_json::from_slice(&body).ok()
    }

    pub fn forget() {
        let _ = std::fs::remove_file(agent::file("agents", &agent::me().id));
    }

    pub fn remember(view: &View, lease: &Lease) {
        if !config::get().pick(&view.device).sticks() {
            return;
        }

        let path = agent::file("agents", &agent::me().id);
        let last = Last {
            device: view.device.id.clone(),
            label: view.device.label.clone(),
            seen: lease.last_seen,
        };

        let _ = path.parent().map(std::fs::create_dir_all);
        let _ = serde_json::to_vec(&last).map(|body| std::fs::write(path, body));
    }
}

pub mod verbs {
    use super::*;

    fn read(agent: &str) -> BTreeMap<String, Unix> {
        std::fs::read(agent::file("verbs", agent))
            .ok()
            .and_then(|body| serde_json::from_slice(&body).ok())
            .unwrap_or_default()
    }

    pub fn at(agent: &str, device: &str) -> Option<Unix> {
        read(agent).get(device).copied()
    }

    pub fn stamp(device: &str) {
        let me = &agent::me().id;
        let path = agent::file("verbs", me);
        let mut verbs = read(me);

        verbs.insert(device.to_string(), model::now());

        let _ = path.parent().map(std::fs::create_dir_all);
        let _ = serde_json::to_vec(&verbs).map(|body| std::fs::write(path, body));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_touch_hands_a_physical_device_back_unless_the_holder_acted_after_it() {
        let touched = Duration::from_secs(30);

        assert!(handed_back(touched, None), "nobody held it");
        assert!(handed_back(touched, Some(90)), "touched after the holder's last verb");
        assert!(!handed_back(touched, Some(10)), "the holder acted after the touch");
    }

    fn me(id: &str) -> Agent {
        Agent {
            id: id.into(),
            label: agent::short(id),
            host: "nyx".into(),
            harness: Harness::Claude,
            pid: Some(10),
            start: Some("500".into()),
            shared: false,
        }
    }

    fn held(by: &Agent, last_seen: Unix) -> Lease {
        Lease {
            last_seen,
            since: last_seen,
            ..Lease::of(by, Duration::from_secs(1200), last_seen)
        }
    }

    fn living(_: u32, start: &str) -> bool {
        start == "500"
    }

    fn judge(now: Unix, dead: &BTreeSet<(u32, String)>) -> Judge<'_> {
        Judge {
            now,
            book_host: "rose",
            dead,
            here: "nyx",
            alive: living,
            blind: false,
            verb: verbed,
        }
    }

    #[test]
    fn a_lease_lapses_after_its_ttl() {
        let none = BTreeSet::new();
        let lease = held(&me("a"), 1_000);

        assert!(judge(1_000 + 1199, &none).valid(&lease, true));
        assert!(!judge(1_000 + 1200, &none).valid(&lease, true));
    }

    #[test]
    fn a_dead_holder_frees_the_device_at_once() {
        let none = BTreeSet::new();
        let mut lease = held(&me("a"), 1_000);

        lease.start = Some("499".into());
        assert!(!judge(1_001, &none).valid(&lease, true), "same host as the client, wrong start");

        let mut there = held(&me("a"), 1_000);
        there.host = "rose".into();
        let dead = BTreeSet::from([(10, "500".to_string())]);

        assert!(!judge(1_001, &dead).valid(&there, true), "reported dead by the ledger host");
        assert!(judge(1_001, &none).valid(&there, true));

        let mut elsewhere = held(&me("a"), 1_000);
        elsewhere.host = "laptop".into();
        assert!(judge(1_001, &dead).valid(&elsewhere, true), "a third machine is ttl only");
    }

    #[test]
    fn a_reader_in_a_pid_sandbox_trusts_holders_on_its_own_host() {
        let mut stale = held(&me("a"), 1_000);
        stale.start = Some("499".into());

        let mut local = held(&me("a"), 1_000);
        local.host = "rose".into();
        let dead = BTreeSet::from([(10, "500".to_string())]);

        let mut blind = judge(1_001, &dead);
        blind.here = "rose";
        blind.blind = true;

        assert!(blind.valid(&local, true), "the book's @dead came from inside the sandbox");

        blind.here = "nyx";

        assert!(blind.valid(&stale, true), "the pid probe cannot see outside the sandbox");
        assert!(!blind.valid(&local, true), "another host's @dead is still trusted");
    }

    #[test]
    fn a_lease_on_an_off_device_is_a_boot_reservation() {
        let none = BTreeSet::new();
        let lease = held(&me("a"), 1_000);

        assert!(judge(1_000 + BOOT - 1, &none).valid(&lease, false));
        assert!(!judge(1_000 + BOOT, &none).valid(&lease, false));
        assert!(judge(1_000 + BOOT, &none).valid(&lease, true));
    }

    #[test]
    fn standing_says_whose_it_is() {
        let none = BTreeSet::new();
        let j = judge(1_100, &none);
        let (a, b) = (me("a"), me("b"));
        let mut ledger = Ledger::default();
        let k = Key::from("avd:pixel");

        assert_eq!(standing(&ledger, &k, &j, &a, true), Standing::Free(None));

        ledger.put(&k, held(&a, 1_000));

        assert!(matches!(standing(&ledger, &k, &j, &a, true), Standing::Mine(_)));
        assert!(matches!(standing(&ledger, &k, &j, &b, true), Standing::Held(_)));

        let mut drifted = a.clone();
        drifted.pid = Some(11);
        drifted.start = Some("777".into());

        assert!(matches!(standing(&ledger, &k, &j, &drifted, true), Standing::Mine(_)));
    }

    #[test]
    fn a_subagent_uses_its_sessions_device_but_not_the_reverse() {
        let none = BTreeSet::new();
        let j = judge(1_100, &none);
        let session = me("1f636eb3-8a91");
        let sub = Agent {
            pid: Some(12),
            start: Some("600".into()),
            ..me("1f636eb3-8a91/a6a8204d")
        };
        let stranger = me("0199aaaa/a6a8204d");
        let elsewhere = Agent {
            host: "rose".into(),
            ..sub.clone()
        };
        let mut ledger = Ledger::default();
        let k = Key::from("avd:pixel");

        ledger.put(&k, held(&session, 1_000));

        assert!(matches!(standing(&ledger, &k, &j, &sub, true), Standing::Mine(_)));
        assert!(matches!(standing(&ledger, &k, &j, &stranger, true), Standing::Held(_)));
        assert!(matches!(standing(&ledger, &k, &j, &elsewhere, true), Standing::Held(_)));

        ledger.put(&k, held(&sub, 1_000));

        assert!(!ledger.leases[&k.id].owned_by(&session));
        assert!(!ledger.leases[&k.id].held_by(&elsewhere));
    }

    fn verbed(agent: &str, device: &str) -> Option<Unix> {
        (agent == "1f636eb3-8a91/a6a8204d" && device == "avd:pixel").then_some(1_000)
    }

    #[test]
    fn a_session_uses_its_subagents_device_once_the_subagent_goes_quiet() {
        let none = BTreeSet::new();
        let session = me("1f636eb3-8a91");
        let sub = me("1f636eb3-8a91/a6a8204d");
        let unstamped = me("1f636eb3-8a91/0000ffff");
        let elsewhere = Agent {
            host: "rose".into(),
            ..session.clone()
        };
        let mut ledger = Ledger::default();
        let k = Key::from("avd:pixel");
        let at = |now: Unix, who: &Agent, ledger: &Ledger| standing(ledger, &k, &judge(now, &none), who, true);

        ledger.put(&k, held(&sub, 1_000));

        assert!(matches!(at(1_009, &session, &ledger), Standing::Held(_)), "9s after the subagent's last verb");
        assert!(matches!(at(1_000 + HANDOFF - 1, &session, &ledger), Standing::Held(_)));
        assert!(matches!(at(1_000 + HANDOFF, &session, &ledger), Standing::Mine(_)));
        assert!(matches!(at(1_000 + HANDOFF, &me("0199aaaa"), &ledger), Standing::Held(_)));
        assert!(matches!(at(1_000 + HANDOFF, &elsewhere, &ledger), Standing::Held(_)));
        assert!(!ledger.leases[&k.id].held_by(&session), "release --all stays exact-holder");

        ledger.put(&k, held(&sub, 800));
        assert!(matches!(at(1_100, &session, &ledger), Standing::Held(_)), "last_seen lags the last verb");

        ledger.put(&k, held(&unstamped, 1_000));
        assert!(matches!(at(1_000 + HANDOFF, &session, &ledger), Standing::Held(_)), "no verb on record");
    }

    #[test]
    fn the_same_agent_owns_its_lease_across_a_sandbox_toggle() {
        let outside = me("a");
        let sandboxed = Agent {
            pid: None,
            start: None,
            ..outside.clone()
        };

        assert!(held(&outside, 1_000).owned_by(&sandboxed));
        assert!(held(&sandboxed, 1_000).owned_by(&outside));
        assert!(!held(&sandboxed, 1_000).owned_by(&me("b")));
    }

    #[test]
    fn a_lock_holds_off_every_agent_take_or_not_but_not_the_human() {
        let none = BTreeSet::new();
        let j = judge(1_000_000, &none);
        let k = Key::from("avd:pixel");
        let human = Agent {
            harness: Harness::User,
            pid: None,
            start: None,
            ..me("luisnquin@nyx")
        };
        let mut ledger = Ledger::default();

        ledger.locks.insert(
            k.id.clone(),
            Lock {
                by: "luisnquin@nyx".into(),
                host: "nyx".into(),
                since: 1_000,
            },
        );

        let Standing::Held(lock) = standing(&ledger, &k, &j, &me("a"), false) else {
            panic!("an agent sees a locked device as held, running or not, long past any ttl");
        };

        assert!(lock.is_lock());
        assert_eq!(letting_go(Standing::Held(lock.clone()), Take::Override), Some(Err(lock.clone())));
        assert!(refuses(&Standing::Held(lock.clone()), Take::Override));
        assert_eq!(standing(&ledger, &k, &j, &human, true), Standing::Free(None));
        assert!(Ledger::parse(&ledger.body()).unwrap().locks.contains_key("avd:pixel"));
        assert!(!Ledger::default().body().contains("locks"));
    }

    #[test]
    fn release_drops_another_agents_lease_only_under_take() {
        let theirs = held(&me("b"), 1_000);

        assert_eq!(letting_go(Standing::Held(theirs.clone()), Take::Respect), Some(Err(theirs.clone())));
        assert_eq!(letting_go(Standing::Held(theirs.clone()), Take::Override), Some(Ok(theirs)));
        assert_eq!(letting_go(Standing::Free(None), Take::Override), None);
    }

    #[test]
    fn an_overridden_display_reverts_when_its_lease_ends_or_changes_hands() {
        let (mine, theirs) = (held(&me("a"), 1_000), held(&me("b"), 1_000));
        let at = |s: Standing, take: Take, moment: Moment| reverts(&s, take, moment, true);

        assert!(at(Standing::Mine(mine.clone()), Take::Respect, Moment::Release));
        assert!(at(Standing::Free(Some(theirs.clone())), Take::Respect, Moment::Release));
        assert!(at(Standing::Free(None), Take::Respect, Moment::Release), "the lease lapsed and was pruned");
        assert!(!at(Standing::Held(theirs.clone()), Take::Respect, Moment::Release), "refused, so untouched");
        assert!(at(Standing::Held(theirs.clone()), Take::Override, Moment::Release));

        assert!(!at(Standing::Mine(mine.clone()), Take::Respect, Moment::Acquire), "a renewal keeps it");
        assert!(at(Standing::Free(Some(theirs.clone())), Take::Respect, Moment::Acquire), "an expired takeover");
        assert!(at(Standing::Free(None), Take::Respect, Moment::Acquire));
        assert!(!at(Standing::Held(theirs.clone()), Take::Respect, Moment::Acquire), "busy, so untouched");
        assert!(at(Standing::Held(theirs), Take::Override, Moment::Acquire));

        assert!(!reverts(&Standing::Free(None), Take::Override, Moment::Release, false));
    }

    #[test]
    fn the_display_record_survives_a_prune_and_a_round_trip() {
        let none = BTreeSet::new();
        let mut ledger = Ledger::default();
        let k = Key::from("avd:pixel");

        ledger.put(&k, held(&me("a"), 1_000));
        ledger.displays.insert(k.id.clone());
        ledger.prune(&judge(1_000 + 1200, &none));

        assert!(ledger.leases.is_empty());
        assert!(Ledger::parse(&ledger.body()).unwrap().displays.contains("avd:pixel"));
        assert!(!Ledger::default().body().contains("displays"));
    }

    #[test]
    fn renewal_waits_for_a_quarter_of_the_ttl() {
        let lease = held(&me("a"), 1_000);

        assert!(!lease.due(1_000 + 299, true));
        assert!(lease.due(1_000 + 300, true));
    }

    #[test]
    fn a_cached_ledger_is_trusted_for_a_minute() {
        let cached = Book {
            read: 1_000,
            ..Book::default()
        };

        assert!(cached.trusted(1_000 + CACHED - 1));
        assert!(!cached.trusted(1_000 + CACHED));
        assert!(Book { fresh: true, ..cached }.trusted(1_000 + 3_600));
    }

    #[test]
    fn a_boot_reservation_is_renewed_before_it_lapses() {
        let lease = held(&me("a"), 1_000);
        let every = cadence(Duration::from_secs(1200)).as_secs() as i64;

        assert!(every < BOOT);
        assert!(lease.due(1_000 + every, false));
        assert!(!lease.due(1_000 + every, true));
        assert_eq!(cadence(Duration::from_secs(40)), Duration::from_secs(15));
    }

    #[test]
    fn old_leases_import_expired_from_since() {
        let ledger = Ledger::parse(
            r#"{"avd:pixel":{"tree":"/a","project":"hotline","since":1000,"session":"5dac5f28-ad4e"}}"#,
        )
        .unwrap();
        let lease = &ledger.leases["avd:pixel"];
        let none = BTreeSet::new();

        assert_eq!((lease.agent.as_str(), lease.last_seen), ("legacy:5dac5f28-ad4e", 1_000));
        assert!(!judge(1_000 + 1200, &none).valid(lease, true));
        assert_eq!(Ledger::parse("").unwrap(), Ledger::default());
    }

    #[test]
    fn an_unreadable_ledger_is_an_error_rather_than_an_empty_one() {
        for body in ["{ not json", "[]", r#"{"v":2,"leases":{"avd:pixel":{"agent":"a"}}}"#] {
            assert!(Ledger::parse(body).is_err(), "{body}");
        }

        let framed = "/s\nrose\n1 2\n@body\n{\"v\":2,\"leases\":7}\n@dead\n@usage\n";

        assert!(Book::parse(&Where::Here, framed).is_err());
    }

    #[test]
    fn the_body_puts_the_liveness_probe_where_the_host_script_greps_for_it() {
        let mut ledger = Ledger::default();
        ledger.put(&Key::from("avd:pixel"), held(&me("a"), 1_000));

        assert!(ledger.body().contains(r#""host":"nyx","pid":10,"start":"500""#));
        assert_eq!(Ledger::parse(&ledger.body()).unwrap(), Ledger { v: 2, ..ledger });
    }

    #[test]
    fn a_hold_under_the_android_id_follows_the_row_to_its_avd() {
        let mut ledger = Ledger::default();
        ledger.leases.insert("android_id:dc3f6e59".into(), held(&me("a"), 1_000));

        let mut pixel = Device::new("avd:rose/pixel", "pixel", Platform::Emulator);
        pixel.host = Some("rose".to_string());
        pixel.add_alias("android_id:dc3f6e59");

        let mut clone = Device::new("avd:rose/pixel-c", "pixel-c", Platform::Emulator);
        clone.host = Some("rose".to_string());

        assert!(ledger.get(&key(&pixel)).is_some());
        assert!(ledger.get(&key(&clone)).is_none());

        ledger.put(&key(&pixel), held(&me("b"), 1_000));

        assert!(!ledger.leases.contains_key("android_id:dc3f6e59"));
        assert_eq!(ledger.leases["avd:pixel"].agent, "b");
    }

    #[test]
    fn a_device_is_filed_as_its_own_host_spells_it() {
        let mut sim = Device::new("mac/AAAA1111", "iPhone 17", Platform::Simulator);
        sim.host = Some("mac".to_string());

        let mut avd = Device::new("avd:mac/pixel", "pixel", Platform::Emulator);
        avd.host = Some("mac".to_string());

        assert_eq!(key(&avd).id, "avd:pixel");
        assert_eq!(key(&sim).id, "AAAA1111");
        assert_eq!(key(&Device::new("android_id:4444", "p", Platform::Emulator)).id, "android_id:4444");
    }

    async fn run(state: &str, args: &[&str]) -> (Status, Book) {
        let ran = Where::Here
            .exec(&format!("XDG_STATE_HOME='{state}'\n{LEDGER}"), args, TIMEOUT)
            .await
            .unwrap();

        (ran.status, Book::parse(&Where::Here, &String::from_utf8_lossy(&ran.stdout)).unwrap())
    }

    #[tokio::test]
    async fn the_host_script_writes_only_over_the_sum_it_was_shown() {
        let state = std::env::temp_dir()
            .join(format!("phone-ledger-{}", std::process::id()))
            .display()
            .to_string();

        let (status, empty) = run(&state, &["-"]).await;
        assert_eq!((status, empty.ledger.leases.len()), (Status::Code(0), 0));

        let mut ledger = Ledger::default();
        ledger.put(&Key::from("avd:pixel"), held(&me("a"), 1_000));

        let body = ledger.body();
        let (status, wrote) = run(&state, &[&empty.sum, &body, "avd:pixel", "/a", "5"]).await;
        assert_eq!(status, Status::Code(0));
        assert_eq!(wrote.ledger.leases["avd:pixel"].agent, "a");
        assert_ne!(wrote.sum, empty.sum);
        assert!(wrote.usage.of(&Device::new("avd:pixel", "pixel", Platform::Emulator), Some("/a")).project.is_some());

        let (status, stale) = run(&state, &[&empty.sum, "{}"]).await;
        assert_eq!(status, Status::Code(CONFLICT));
        assert_eq!(stale.ledger.leases["avd:pixel"].agent, "a", "a stale write leaves the file alone");
        assert_eq!(stale.sum, wrote.sum);

        std::fs::remove_dir_all(&state).unwrap();
    }

    #[tokio::test]
    async fn a_stale_lock_is_broken_by_renaming_it_first() {
        let state = std::env::temp_dir().join(format!("phone-lock-{}", std::process::id()));
        let lock = state.join("phone/leases.lock");

        std::fs::create_dir_all(&lock).unwrap();
        std::fs::File::open(&lock)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - Duration::from_secs(60))
            .unwrap();

        let dir = state.display().to_string();
        let (_, empty) = run(&dir, &["-"]).await;
        let (status, _) = run(&dir, &[&empty.sum, "{\"v\":2}"]).await;

        let left: Vec<_> = std::fs::read_dir(state.join("phone"))
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|name| name.starts_with("leases.lock"))
            .collect();

        assert_eq!(status, Status::Code(0));
        assert!(left.is_empty(), "{left:?}");

        std::fs::remove_dir_all(&state).unwrap();
    }

    #[tokio::test]
    async fn the_host_script_reports_holders_on_its_own_machine_that_died() {
        let state = std::env::temp_dir()
            .join(format!("phone-dead-{}", std::process::id()))
            .display()
            .to_string();

        let host = agent::hostname();
        let pid = std::process::id();
        let start = agent::start(pid).unwrap();
        let mut ledger = Ledger::default();

        for (k, start) in [("live", start.as_str()), ("dead", "1")] {
            let mut lease = held(&me("a"), 1_000);
            (lease.host, lease.pid, lease.start) = (host.clone(), Some(pid), Some(start.to_string()));
            ledger.put(&Key::from(k), lease);
        }

        let (_, empty) = run(&state, &["-"]).await;
        let (_, book) = run(&state, &[&empty.sum, &ledger.body()]).await;

        assert_eq!(book.dead, BTreeSet::from([(pid, "1".to_string())]));

        std::fs::remove_dir_all(&state).unwrap();
    }
}
