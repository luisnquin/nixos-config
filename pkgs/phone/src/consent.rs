use std::collections::BTreeMap;
use std::future::Future;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::{self, Agent, Harness};
use crate::cli::Verdict;
use crate::lease::{self, Book, Lease, Ledger, Lock};
use crate::model::{self, Device, Unix, View};
use crate::registry::{self, Registry};
use crate::ssh::Where;
use crate::config;

const WINDOW: Unix = 15 * 60;
const PROMPT: Duration = Duration::from_secs(70);
const WAIT: Duration = Duration::from_secs(10 * 60);
const POLL: Duration = Duration::from_secs(2);
const STALE: Unix = 90;
const RECENT: Unix = 7 * 24 * 3600;
const ANY: &str = "*";
const MARKS: usize = 3;
const CLICKS: &str = "left: allow · middle: 15m · right: refuse";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Allow,
    Until,
    Refuse,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Unix>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

impl Grant {
    fn active(&self, now: Unix) -> bool {
        match self.state {
            State::Allow => true,
            State::Until => self.until.is_some_and(|until| now < until),
            State::Refuse => false,
        }
    }

    fn lapsed(&self, now: Unix) -> bool {
        self.state == State::Until && !self.active(now)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub agent: String,
    pub label: String,
    pub at: Unix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

fn asking(me: &Agent, now: Unix) -> Pending {
    Pending {
        agent: me.id.clone(),
        label: tag(Some(me.harness), &me.label),
        at: now,
        why: lease::why(),
    }
}

fn session(agent: &str) -> &str {
    agent.split_once('/').map_or(agent, |(session, _)| session)
}

fn hand_over(ledger: &mut Ledger, key: &lease::Key, to: &str) -> Vec<(String, Grant)> {
    if to == ANY {
        return Vec::new();
    }

    let mut gone = Vec::new();

    if let Some(grants) = ledger.grants.get_mut(&key.id) {
        let others: Vec<String> = grants
            .iter()
            .filter(|(agent, g)| session(agent) != session(to) && agent.as_str() != ANY && g.state != State::Refuse)
            .map(|(agent, _)| agent.clone())
            .collect();

        for agent in others {
            if let Some(grant) = grants.remove(&agent) {
                gone.push((agent, grant));
            }
        }
    }

    if ledger.get(key).is_some_and(|(_, l)| l.harness != Some(Harness::User) && session(&l.agent) != session(to)) {
        ledger.drop_key(key);
    }

    gone
}

#[derive(Debug, PartialEq, Eq)]
enum Ruling {
    Granted,
    Refused,
    Ask,
}

fn grant_of<'a>(ledger: &'a Ledger, device: &str, agent: &str) -> Option<(&'a String, &'a Grant)> {
    let grants = ledger.grants.get(device)?;

    grants
        .get_key_value(agent)
        .or_else(|| grants.get_key_value(agent.split_once('/')?.0))
        .or_else(|| grants.get_key_value(ANY))
}

fn ruling(ledger: &Ledger, device: &str, agent: &str, now: Unix) -> Ruling {
    match grant_of(ledger, device, agent) {
        Some((_, grant)) if grant.active(now) => Ruling::Granted,
        Some((_, grant)) if grant.state == State::Refuse => Ruling::Refused,
        _ => Ruling::Ask,
    }
}

fn revoke(ledger: &mut Ledger, device: &str, agent: &str) -> Option<Grant> {
    let grants = ledger.grants.get_mut(device)?;
    let was = grants.remove(agent);

    if grants.is_empty() {
        ledger.grants.remove(device);
    }

    was
}

fn tag(harness: Option<Harness>, label: &str) -> String {
    let harness = harness.map_or("agent".to_string(), |h| format!("{h:?}").to_lowercase());

    format!("{harness}:{label}")
}

fn held_by(lease: &Lease) -> String {
    tag(lease.harness, &lease.label)
}

pub async fn gate(view: &View) -> Result<bool> {
    let me = agent::me();

    if me.harness == Harness::User || !config::get().consent(&view.device) {
        return Ok(false);
    }

    let at = lease::ledger_of(&view.device);
    let key = lease::key(&view.device);
    let id = key.id.clone();
    let name = &view.device.label;
    let now = model::now();
    let mut lapsed = None;

    let ledger = lease::amend(&at, |ledger| {
        lapsed = None;

        if ruling(ledger, &id, &me.id, now) != Ruling::Ask {
            return false;
        }

        if let Some((key, _)) = grant_of(ledger, &id, &me.id).filter(|(_, g)| g.lapsed(now)) {
            lapsed = revoke(ledger, &id, &key.clone());
        }

        ledger.asking.insert(id.clone(), asking(me, now));
        true
    })
    .await?;

    match ruling(&ledger, &id, &me.id, now) {
        Ruling::Granted => return Ok(true),
        Ruling::Refused => return Err(refused(name).into()),
        Ruling::Ask => {}
    }

    if let Some(old) = &lapsed {
        record(name, &me.id, &old.label, Some(old), None, "expiry");
    }

    let answered = tokio::select! {
        grant = consult(prompt, name, me, now) => grant,
        () = elsewhere(&at, &id, me) => None,
    };

    let decided = match answered {
        Some(grant) => Some(Some(grant)),
        None if tokio::time::timeout(WAIT, elsewhere(&at, &id, me)).await.is_ok() => None,
        None => Some(None),
    };

    let ledger = settle(&at, &key, name, me, decided).await?;

    match ruling(&ledger, &id, &me.id, model::now()) {
        Ruling::Granted => Ok(true),
        Ruling::Refused => Err(refused(name).into()),
        Ruling::Ask => Err(crate::Refused(format!(
            "{name} needs the user's consent and nobody answered in {}m; ask the user in chat to allow it from the phone panel or `phone grant {name} allow`, then retry",
            WAIT.as_secs() / 60
        ))
        .into()),
    }
}

async fn settle(at: &Where, key: &lease::Key, name: &str, me: &Agent, decided: Option<Option<Grant>>) -> Result<Ledger> {
    let id = &key.id;
    let mut was = None;
    let mut gone = Vec::new();
    let ledger = lease::amend(at, |ledger| {
        if ledger.asking.get(id).is_some_and(|p| p.agent == me.id) {
            ledger.asking.remove(id);
        }

        was = grant_of(ledger, id, &me.id).map(|(_, g)| g.clone());
        gone.clear();

        if let Some(Some(grant)) = &decided {
            if grant.state != State::Refuse {
                gone = hand_over(ledger, key, &me.id);
            }

            ledger.grants.entry(id.to_string()).or_default().insert(me.id.clone(), grant.clone());
        }

        true
    })
    .await?;

    for (agent, grant) in &gone {
        record(name, agent, &grant.label, Some(grant), None, "handover");
    }

    match &decided {
        Some(Some(grant)) => record(name, &me.id, &grant.label, was.as_ref(), Some(grant), "prompt"),
        Some(None) => record(name, &me.id, &tag(Some(me.harness), &me.label), None, None, "timeout"),
        None => {}
    }

    Ok(ledger)
}

fn refused(name: &str) -> crate::Refused {
    crate::Refused(format!(
        "the user refused this agent the use of {name}; stop and ask them in chat, `--take` does not lift it"
    ))
}

async fn elsewhere(at: &Where, device: &str, me: &Agent) {
    loop {
        tokio::time::sleep(POLL).await;

        let now = model::now();
        let ledger = lease::amend(at, |ledger| {
            if ruling(ledger, device, &me.id, now) != Ruling::Ask {
                return false;
            }

            let stale = ledger.asking.get(device).is_none_or(|p| match p.agent == me.id {
                true => now - p.at >= STALE / 3,
                false => now - p.at >= STALE,
            });

            if stale {
                ledger.asking.insert(device.to_string(), asking(me, now));
            }

            stale
        })
        .await;

        if ledger.is_ok_and(|l| ruling(&l, device, &me.id, now) != Ruling::Ask) {
            return;
        }
    }
}

async fn consult<F: Future<Output = Option<String>>>(ask: impl FnOnce(String, String) -> F, name: &str, me: &Agent, now: Unix) -> Option<Grant> {
    let summary = format!("{name}: {} wants the phone", format!("{:?}", me.harness).to_lowercase());
    let pane = std::env::var("HERDR_PANE_ID").or_else(|_| std::env::var("TMUX_PANE")).ok();
    let body = [
        Some(format!("{} on {}", tag(Some(me.harness), &me.label), me.host)),
        pane.filter(|p| !p.is_empty()).map(|p| format!("pane {p}")),
        lease::why().map(|why| format!("for \"{why}\"")),
        Some(CLICKS.to_string()),
    ];
    let body = body.into_iter().flatten().collect::<Vec<_>>().join("\n");

    answer(ask(summary, body).await.as_deref(), tag(Some(me.harness), &me.label), now).map(|grant| Grant { why: lease::why(), ..grant })
}

fn answer(said: Option<&str>, label: String, now: Unix) -> Option<Grant> {
    let (state, until) = match said.map(str::trim)? {
        "allow" => (State::Allow, None),
        "window" => (State::Until, Some(now + WINDOW)),
        "refuse" => (State::Refuse, None),
        _ => return None,
    };

    Some(Grant { state, until, label, why: None })
}

async fn prompt(summary: String, body: String) -> Option<String> {
    std::env::var_os("WAYLAND_DISPLAY")?;

    let asked = tokio::process::Command::new("notify-send")
        .args([
            "--app-name=phone-consent",
            "--urgency=critical",
            "--wait",
            "--expire-time=60000",
            "-A",
            "allow=Allow",
            "-A",
            "window=15m",
            "-A",
            "refuse=Refuse",
            "--",
            &summary,
            &body,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();

    let out = tokio::time::timeout(PROMPT, asked).await.ok()?.ok()?;

    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn phrase(grant: Option<&Grant>) -> String {
    let Some(grant) = grant else {
        return "asks again".to_string();
    };

    match (grant.state, grant.until) {
        (State::Until, Some(until)) => format!("allowed until {}", clock(until)),
        (State::Allow | State::Until, _) => "allowed".to_string(),
        (State::Refuse, _) => "refused".to_string(),
    }
}

fn clock(at: Unix) -> String {
    chrono::DateTime::from_timestamp(at, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| at.to_string())
}

fn record(device: &str, agent: &str, label: &str, from: Option<&Grant>, to: Option<&Grant>, by: &str) {
    let state = |g: Option<&Grant>| g.map_or(json!("ask"), |g| json!(g.state));
    let entry = json!({
        "ts": model::now(),
        "device": device,
        "agent": agent,
        "from": state(from),
        "to": state(to),
        "until": to.and_then(|g| g.until),
        "by": by,
    });

    let dir = registry::state_dir();
    let log = std::fs::create_dir_all(&dir).and_then(|()| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("grants.jsonl"))
    });

    if let Ok(mut log) = log {
        let _ = writeln!(log, "{entry}");
    }

    let said = match by {
        "timeout" => "not answered".to_string(),
        _ => phrase(to),
    };

    quietly(Command::new("notify-send").args(["--app-name=phone", "--urgency=low", "--", &format!("{device}: {label} {said} ({by})")]));
    refresh_bar();
}

fn quietly(command: &mut Command) {
    let _ = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn refresh_bar() {
    let Some(n) = std::env::var("PHONE_WAYBAR_SIGNAL").ok().filter(|n| n.parse::<u8>().is_ok()) else {
        return;
    };
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return;
    };

    for pid in procs.flatten().filter_map(|e| e.file_name().into_string().ok()) {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();

        if matches!(comm.trim(), "waybar" | ".waybar-wrapped") {
            quietly(Command::new("kill").args(["-s", &format!("RTMIN+{n}"), &pid]));
        }
    }
}

fn human_only(me: &Agent, name: &str) -> Result<()> {
    match me.harness {
        Harness::User => Ok(()),
        _ => Err(crate::Refused(format!(
            "only a human shell may grant a device; ask the user to run `phone grant {name} allow`"
        ))
        .into()),
    }
}

pub fn named(reg: &Registry, want: &str) -> Result<Device> {
    let found: BTreeMap<String, &Device> = reg
        .find(want)
        .into_iter()
        .filter(|d| config::get().consent(d))
        .map(|d| (lease::key(d).id, d))
        .collect();

    match found.values().collect::<Vec<_>>()[..] {
        [device] => Ok((*device).clone()),
        [] => bail!("no device that needs consent matches '{want}'"),
        _ => bail!("'{want}' matches {} devices that need consent; be more specific", found.len()),
    }
}

fn target(ledger: &Ledger, device: &str, agent: Option<&str>, holder: Option<&Lease>, pending: Option<&Pending>, asker_first: bool) -> (String, String) {
    if let Some(id) = agent {
        let label = holder
            .filter(|l| l.agent == id)
            .map(held_by)
            .or_else(|| pending.filter(|p| p.agent == id).map(|p| p.label.clone()))
            .or_else(|| grant_of(ledger, device, id).map(|(_, g)| g.label.clone()))
            .unwrap_or_else(|| agent::short(id));

        return (id.to_string(), label);
    }

    let held = holder.map(|l| (l.agent.clone(), held_by(l)));
    let asked = pending.map(|p| (p.agent.clone(), p.label.clone()));
    let picked = match asker_first {
        true => asked.or(held),
        false => held.or(asked),
    };

    picked.unwrap_or_else(|| (ANY.to_string(), "any agent".to_string()))
}

fn verdict_grant(verdict: Verdict, window: Option<Duration>, label: &str, now: Unix) -> Option<Grant> {
    let label = label.to_string();

    match verdict {
        Verdict::Allow => Some(Grant {
            state: window.map_or(State::Allow, |_| State::Until),
            until: window.map(|w| now + w.as_secs() as Unix),
            label,
            why: None,
        }),
        Verdict::Refuse => Some(Grant {
            state: State::Refuse,
            until: None,
            label,
            why: None,
        }),
        Verdict::Stop | Verdict::Ask => None,
    }
}

pub async fn rule(device: &Device, verdict: Verdict, agent: Option<&str>, window: Option<Duration>) -> Result<String> {
    let name = &device.label;

    human_only(agent::me(), name)?;

    if window.is_some() && verdict != Verdict::Allow {
        bail!("--for only goes with allow");
    }

    let at = lease::ledger_of(device);
    let key = lease::key(device);
    let book = Book::fetch(&at).await?;
    let now = model::now();
    let judge = book.judge(now);
    let holder = book
        .ledger
        .get(&key)
        .map(|(_, l)| l)
        .filter(|l| l.harness != Some(Harness::User) && judge.valid(l, true));
    let pending = book.ledger.asking.get(&key.id).filter(|p| now - p.at < STALE);

    let asker_first = matches!(verdict, Verdict::Allow | Verdict::Refuse);
    let (who, label) = target(&book.ledger, &key.id, agent, holder, pending, asker_first);
    let why = pending
        .filter(|p| p.agent == who)
        .and_then(|p| p.why.clone())
        .or_else(|| grant_of(&book.ledger, &key.id, &who).and_then(|(_, g)| g.why.clone()));
    let to = verdict_grant(verdict, window, &label, now).map(|grant| Grant { why, ..grant });

    let mut was = None;
    let mut gone = Vec::new();

    lease::amend(&at, |ledger| {
        gone = match verdict {
            Verdict::Allow => hand_over(ledger, &key, &who),
            _ => Vec::new(),
        };

        was = match &to {
            Some(grant) => ledger.grants.entry(key.id.clone()).or_default().insert(who.clone(), grant.clone()),
            None => revoke(ledger, &key.id, &who),
        };

        if verdict == Verdict::Stop && ledger.get(&key).is_some_and(|(_, l)| l.agent == who) {
            ledger.drop_key(&key);
        }

        true
    })
    .await?;

    for (agent, grant) in &gone {
        record(name, agent, &grant.label, Some(grant), None, "handover");
    }

    record(name, &who, &label, was.as_ref(), to.as_ref(), "user");

    Ok(match verdict {
        Verdict::Stop => format!("{name}: stopped {label}; its next verb asks again"),
        _ => format!("{name}: {label} {}", phrase(to.as_ref())),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mood {
    Idle,
    Locked,
    Granted,
    Held,
    Asking,
}

#[derive(Debug, Serialize)]
struct Row {
    name: String,
    state: Mood,
    detail: String,
}

fn details(asking: Option<&Pending>, holder: Option<&Lease>, grants: &[(&String, &Grant)], lock: Option<&Lock>, now: Unix) -> Vec<String> {
    let mut said = Vec::new();

    if let Some(p) = asking {
        said.push(format!("{} is asking ({}s){}", p.label, now - p.at, reason(p.why.as_ref())));
    }

    if let Some(lease) = holder {
        said.push(format!("held by {}", held_by(lease)));
    }

    for (agent, grant) in grants.iter().filter(|(_, g)| g.active(now) || g.state == State::Refuse) {
        match holder.is_some_and(|l| &l.agent == *agent) {
            true => said.push(format!("{}{}", phrase(Some(grant)), reason(grant.why.as_ref()))),
            false => said.push(format!("{} {}{}", grant.label, phrase(Some(grant)), reason(grant.why.as_ref()))),
        }
    }

    if let Some(lock) = lock {
        said.push(format!("locked by {}", lock.by));
    }

    if said.is_empty() {
        said.push("asks first".to_string());
    }

    said
}

fn reason(why: Option<&String>) -> String {
    why.map(|why| format!(" for \"{why}\"")).unwrap_or_default()
}

fn row(name: &str, ledger: &Ledger, device: &str, holder: Option<&Lease>, now: Unix) -> Row {
    let asking = ledger.asking.get(device).filter(|p| now - p.at < STALE);
    let grants: Vec<(&String, &Grant)> = ledger.grants.get(device).into_iter().flatten().collect();
    let granted = grants.iter().any(|(_, g)| g.active(now));
    let lock = ledger.locks.get(device);
    let said = details(asking, holder, &grants, lock, now);

    let state = match () {
        _ if asking.is_some() => Mood::Asking,
        _ if holder.is_some() => Mood::Held,
        _ if granted => Mood::Granted,
        _ if lock.is_some() => Mood::Locked,
        _ => Mood::Idle,
    };

    Row {
        name: name.to_string(),
        state,
        detail: said.join(" · "),
    }
}

fn waybar(rows: &[Row]) -> serde_json::Value {
    let mood = rows.iter().map(|r| r.state).max().unwrap_or(Mood::Idle);
    let tooltip = rows.iter().map(|r| format!("{}: {}", r.name, r.detail)).collect::<Vec<_>>().join("\n");

    let marks = rows.iter().take(MARKS).map(|r| format!("{:?}", r.state).to_lowercase()).collect::<Vec<_>>();

    json!({ "text": " ", "tooltip": tooltip, "class": [mood, format!("marks-{}", marks.join("-"))] })
}

pub async fn report(reg: &Registry, bar: bool) -> Result<()> {
    let now = model::now();
    let mut books: Vec<(Where, Option<Book>)> = Vec::new();
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();

    for device in reg.devices.iter().filter(|d| config::get().consent(d)) {
        let key = lease::key(device);
        let at = lease::ledger_of(device);

        if rows.contains_key(&key.id) {
            continue;
        }

        if !books.iter().any(|(w, _)| *w == at) {
            books.push((at.clone(), Book::fetch(&at).await.ok()));
        }

        let row = match books.iter().find(|(w, _)| *w == at).and_then(|(_, b)| b.as_ref()) {
            Some(book) => {
                let judge = book.judge(now);
                let holder = book
                    .ledger
                    .get(&key)
                    .map(|(_, l)| l)
                    .filter(|l| l.harness != Some(Harness::User) && judge.valid(l, true));

                row(&device.label, &book.ledger, &key.id, holder, now)
            }
            None => Row {
                name: device.label.clone(),
                state: Mood::Idle,
                detail: format!("ledger on {} unreachable", at.label()),
            },
        };

        if row.state == Mood::Idle && device.last_connected.is_none_or(|t| now - t > RECENT) {
            continue;
        }

        rows.insert(key.id.clone(), row);
    }

    let rows: Vec<Row> = rows.into_values().collect();

    match bar {
        true => println!("{}", waybar(&rows)),
        false => println!("{}", json!({ "devices": rows })),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me(id: &str, harness: Harness) -> Agent {
        Agent {
            id: id.into(),
            label: agent::short(id),
            host: "nyx".into(),
            harness,
            pid: None,
            start: None,
            shared: false,
        }
    }

    fn grant(state: State, until: Option<Unix>) -> Grant {
        Grant {
            state,
            until,
            label: "claude:a".into(),
            why: None,
        }
    }

    #[test]
    fn grants_and_a_pending_ask_survive_the_ledger_and_old_ledgers_still_load() {
        let mut ledger = Ledger::default();

        ledger.grants.entry("R58N".into()).or_default().insert("a".into(), grant(State::Until, Some(2_000)));
        ledger.asking.insert(
            "R58N".into(),
            Pending {
                agent: "b".into(),
                label: "codex:b".into(),
                at: 1_000,
                why: None,
            },
        );

        let back = Ledger::parse(&ledger.body()).unwrap();

        assert_eq!(back.grants, ledger.grants);
        assert_eq!(back.asking, ledger.asking);
        assert!(!Ledger::default().body().contains("grants"));
        assert!(!Ledger::default().body().contains("asking"));
        assert!(Ledger::parse(r#"{"v":2,"leases":{}}"#).unwrap().grants.is_empty());
    }

    #[test]
    fn a_window_lapses_and_a_refusal_sticks() {
        let mut ledger = Ledger::default();
        let grants = ledger.grants.entry("R58N".into()).or_default();

        grants.insert("a".into(), grant(State::Until, Some(2_000)));
        grants.insert("b".into(), grant(State::Refuse, None));
        grants.insert("c".into(), grant(State::Allow, None));

        assert_eq!(ruling(&ledger, "R58N", "a", 1_999), Ruling::Granted);
        assert_eq!(ruling(&ledger, "R58N", "a", 2_000), Ruling::Ask);
        assert!(grant_of(&ledger, "R58N", "a").unwrap().1.lapsed(2_000));
        assert_eq!(ruling(&ledger, "R58N", "b", 1_000), Ruling::Refused);
        assert_eq!(ruling(&ledger, "R58N", "c", i64::MAX), Ruling::Granted);
        assert_eq!(ruling(&ledger, "R58N", "c/sub", 1_000), Ruling::Granted, "a subagent inherits its session's grant");
        assert_eq!(ruling(&ledger, "R58N", "d", 1_000), Ruling::Ask);
        assert_eq!(ruling(&ledger, "other", "c", 1_000), Ruling::Ask);

        ledger.grants.get_mut("R58N").unwrap().insert(ANY.into(), grant(State::Allow, None));
        assert_eq!(ruling(&ledger, "R58N", "d", 1_000), Ruling::Granted, "a device-wide grant covers any agent");
        assert_eq!(ruling(&ledger, "R58N", "b", 1_000), Ruling::Refused, "but not one refused by name");

        revoke(&mut ledger, "R58N", "a");
        revoke(&mut ledger, "R58N", "b");
        revoke(&mut ledger, "R58N", "c");
        revoke(&mut ledger, "R58N", ANY);
        assert!(ledger.grants.is_empty());
    }

    #[test]
    fn the_bar_shows_the_most_urgent_device() {
        let mut ledger = Ledger::default();
        let holder = Lease::of(&me("h", Harness::Claude), Duration::from_secs(60), 1_000);

        assert_eq!(row("f", &ledger, "R58N", None, 1_000).state, Mood::Idle);
        assert_eq!(row("f", &ledger, "R58N", None, 1_000).detail, "asks first");

        ledger.locks.insert(
            "R58N".into(),
            lease::Lock {
                by: "luis@nyx".into(),
                host: "nyx".into(),
                since: 1_000,
            },
        );
        assert_eq!(row("f", &ledger, "R58N", None, 1_000).state, Mood::Locked);

        ledger.grants.entry("R58N".into()).or_default().insert("h".into(), grant(State::Until, Some(2_000)));
        assert_eq!(row("f", &ledger, "R58N", None, 1_000).state, Mood::Granted);
        assert_eq!(row("f", &ledger, "R58N", None, 2_000).state, Mood::Locked, "the window closed");
        assert_eq!(row("f", &ledger, "R58N", Some(&holder), 1_000).state, Mood::Held);

        ledger.asking.insert(
            "R58N".into(),
            Pending {
                agent: "b".into(),
                label: "codex:b".into(),
                at: 1_000,
                why: None,
            },
        );
        assert_eq!(row("f", &ledger, "R58N", Some(&holder), 1_000).state, Mood::Asking);
        assert_eq!(row("f", &ledger, "R58N", Some(&holder), 1_000 + STALE).state, Mood::Held, "a stale ask");

        let rows = [
            row("a", &Ledger::default(), "x", None, 1_000),
            row("f", &ledger, "R58N", Some(&holder), 1_000),
        ];
        let bar = waybar(&rows);

        assert_eq!(bar["class"], json!(["asking", "marks-idle-asking"]));
        assert_eq!(bar["text"], " ");
        assert_eq!(bar["tooltip"].as_str().unwrap().lines().count(), 2);
        assert_eq!(waybar(&rows[..1])["class"], json!(["idle", "marks-idle"]));
        assert_eq!(waybar(&[])["class"], json!(["idle", "marks-"]));
    }

    #[test]
    fn allowing_one_agent_hands_the_device_over() {
        let mut ledger = Ledger::default();
        let key = lease::Key::from("R58N");
        let grants = ledger.grants.entry("R58N".into()).or_default();

        grants.insert("old".into(), grant(State::Allow, None));
        grants.insert("no".into(), grant(State::Refuse, None));
        grants.insert(ANY.into(), grant(State::Allow, None));
        ledger.put(&key, Lease::of(&me("old/sub", Harness::Claude), Duration::from_secs(60), 1_000));

        assert!(hand_over(&mut ledger.clone(), &key, "old/sub").is_empty(), "its own subagent keeps the lease");

        let gone = hand_over(&mut ledger, &key, "new");

        assert_eq!(gone.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>(), ["old"]);
        assert_eq!(ruling(&ledger, "R58N", "no", 1_000), Ruling::Refused, "a refusal survives");
        assert!(ledger.grants["R58N"].contains_key(ANY));
        assert!(ledger.get(&key).is_none(), "the old holder lost its lease");

        let pending = Pending {
            agent: "new".into(),
            label: "claude:new".into(),
            at: 1_000,
            why: Some("login bug".into()),
        };
        let holder = Lease::of(&me("old", Harness::Claude), Duration::from_secs(60), 1_000);

        assert_eq!(target(&ledger, "R58N", None, Some(&holder), Some(&pending), true).0, "new", "allow goes to the asker");
        assert_eq!(target(&ledger, "R58N", None, Some(&holder), Some(&pending), false).0, "old", "stop goes to the holder");
        assert_eq!(reason(pending.why.as_ref()), " for \"login bug\"");
    }

    #[test]
    fn only_a_human_shell_grants() {
        assert!(human_only(&me("luis@nyx", Harness::User), "faraday").is_ok());

        for harness in [Harness::Claude, Harness::Codex, Harness::Env] {
            let refused = human_only(&me("a", harness), "faraday").unwrap_err();

            assert!(refused.is::<crate::Refused>());
        }
    }

    #[tokio::test]
    async fn the_prompts_answer_becomes_the_grant_and_silence_stores_nothing() {
        let agent = me("1f636eb3-8a91", Harness::Claude);
        let ask = |said: Option<&'static str>| {
            move |summary: String, body: String| async move {
                assert_eq!(summary, "faraday: claude wants the phone");
                assert!(body.starts_with("claude:1f636eb3 on nyx\n"));
                assert!(body.ends_with(CLICKS));

                said.map(str::to_string)
            }
        };

        let allow = consult(ask(Some("allow\n")), "faraday", &agent, 1_000).await.unwrap();
        let window = consult(ask(Some("window")), "faraday", &agent, 1_000).await.unwrap();
        let refuse = consult(ask(Some("refuse")), "faraday", &agent, 1_000).await.unwrap();

        assert_eq!((allow.state, allow.until, allow.label.as_str()), (State::Allow, None, "claude:1f636eb3"));
        assert_eq!((window.state, window.until), (State::Until, Some(1_000 + WINDOW)));
        assert_eq!(refuse.state, State::Refuse);
        assert_eq!(consult(ask(Some("")), "faraday", &agent, 1_000).await, None);
        assert_eq!(consult(ask(None), "faraday", &agent, 1_000).await, None);
        assert_eq!(consult(ask(Some("2")), "faraday", &agent, 1_000).await, None);
    }
}
