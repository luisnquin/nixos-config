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
        let same_process = match (self.pid, me.pid) {
            (Some(pid), Some(mine)) => pid == mine && self.start == me.start,
            _ => true,
        };

        self.agent == me.id && self.host == me.host && same_process
    }

    pub fn idle(&self, now: Unix) -> i64 {
        (now - self.last_seen).max(0)
    }

    pub fn due(&self, now: Unix, running: bool) -> bool {
        self.idle(now) * 4 >= self.ttl || !running && self.idle(now) * 2 >= BOOT
    }

    pub fn describe(&self, now: Unix) -> String {
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    pub v: u32,
    #[serde(default)]
    pub leases: BTreeMap<String, Lease>,
    #[serde(default)]
    pub installed: BTreeMap<String, BTreeSet<String>>,
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
}

impl Judge<'_> {
    pub fn valid(&self, lease: &Lease, running: bool) -> bool {
        let idle = lease.idle(self.now);

        idle < lease.ttl && (running || idle < BOOT) && self.living(lease)
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
    let Some((_, lease)) = ledger.get(key) else {
        return Standing::Free(None);
    };

    match (judge.valid(lease, running), lease.owned_by(me)) {
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
    match acquire(view, take()).await? {
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
            Step::Done(got) => return Ok(got),
            Step::Refresh => book = Book::fetch(&at).await?,
            Step::Write(ledger, lease) => match book.commit(&ledger, &stamp(view, &book).await).await? {
                Ok(_) => return Ok(settled(view, lease)),
                Err(newer) => book = newer,
            },
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

enum Step {
    Done(Got),
    Refresh,
    Write(Ledger, Lease),
}

fn attempt(view: &View, take: Take, book: &Book) -> Step {
    let now = model::now();
    let me = agent::me();
    let key = key(&view.device);
    let judge = book.judge(now);
    let running = actions::running(&view.reach);

    let lease = match standing(&book.ledger, &key, &judge, me, running) {
        Standing::Mine(lease) if !lease.due(now, running) && book.trusted(now) => return Step::Done(Got::Held(lease)),
        Standing::Mine(_) | Standing::Held(_) if !book.fresh => return Step::Refresh,
        Standing::Held(lease) if take == Take::Respect => return Step::Done(Got::Busy(lease)),
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

    Step::Write(ledger, lease)
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

    for _ in 0..RETRIES {
        let key = key(&view.device);

        let had = match letting_go(book.standing(&view.device, actions::running(&view.reach)), take()) {
            Some(Ok(lease)) => lease,
            Some(Err(lease)) => return busy(view, lease),
            None => return Ok(None),
        };

        let mut ledger = book.ledger.clone();
        ledger.drop_key(&key);

        match book.commit(&ledger, &[]).await? {
            Ok(_) => return Ok(Some(had).filter(|l| l.owned_by(agent::me()))),
            Err(newer) => book = newer,
        }
    }

    bail!("the leases on {} kept changing under this run; try again", at.label())
}

fn letting_go(standing: Standing, take: Take) -> Option<Result<Lease, Lease>> {
    match (standing, take) {
        (Standing::Held(lease), Take::Respect) => Some(Err(lease)),
        (Standing::Free(None), _) => None,
        (Standing::Mine(lease) | Standing::Held(lease) | Standing::Free(Some(lease)), _) => Some(Ok(lease)),
    }
}

fn busy(view: &View, lease: Lease) -> Result<Option<Lease>> {
    Err(refusal(view, &lease).into())
}

pub fn refusal(view: &View, lease: &Lease) -> crate::Refused {
    crate::Refused(format!(
        "{} is held by {}; its holder's `phone release -t {}` frees it, `--take` takes it",
        view.device.label,
        lease.describe(model::now()),
        view.device.label
    ))
}

pub async fn guard(view: &View) -> Result<()> {
    let book = Book::fetch(&actions::where_of(&view.device)).await?;

    match (book.standing(&view.device, actions::running(&view.reach)), take()) {
        (Standing::Held(lease), Take::Respect) => Err(refusal(view, &lease).into()),
        _ => Ok(()),
    }
}

pub async fn here_tree() -> Option<String> {
    tree(&Project::here().ok().flatten()?).await.ok()
}

pub async fn release_all(hosts: &[Where]) -> Result<Vec<String>> {
    let mut freed = Vec::new();

    for at in hosts {
        let Ok(mut book) = Book::fetch(at).await else {
            continue;
        };

        for _ in 0..RETRIES {
            let mine: Vec<String> = book
                .ledger
                .leases
                .iter()
                .filter(|(_, l)| l.owned_by(agent::me()))
                .map(|(k, _)| k.clone())
                .collect();

            if mine.is_empty() {
                break;
            }

            let mut ledger = book.ledger.clone();
            ledger.leases.retain(|k, _| !mine.contains(k));

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

    pub fn remember(view: &View, lease: &Lease) {
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

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut resumed = a.clone();
        resumed.start = Some("777".into());

        assert!(matches!(standing(&ledger, &k, &j, &resumed, true), Standing::Held(_)));
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
    fn release_drops_another_agents_lease_only_under_take() {
        let theirs = held(&me("b"), 1_000);

        assert_eq!(letting_go(Standing::Held(theirs.clone()), Take::Respect), Some(Err(theirs.clone())));
        assert_eq!(letting_go(Standing::Held(theirs.clone()), Take::Override), Some(Ok(theirs)));
        assert_eq!(letting_go(Standing::Free(None), Take::Override), None);
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
