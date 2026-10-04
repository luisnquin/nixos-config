use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::ValueEnum;
use serde::Serialize;

use crate::hook::Harness;
use crate::model::Device;
use crate::{lease, registry};

const HEAD: usize = 4096;
const TAIL: usize = 2048;
const ROTATE_AT: u64 = 32 << 20;

const ENV: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_PID",
    "CODEX_THREAD_ID",
    "CODEX_SANDBOX",
    "CODEX_CI",
    "HERDR_SESSION",
    "HERDR_WORKSPACE_ID",
    "HERDR_TAB_ID",
    "HERDR_PANE_ID",
    "PHONE_TARGET",
];

static CALL: Mutex<Option<Call>> = Mutex::new(None);

#[derive(Clone, Copy)]
pub enum Cost {
    Adb,
    Ssh,
    Dump,
    Capture,
}

impl Cost {
    fn name(self) -> &'static str {
        match self {
            Cost::Adb => "adb",
            Cost::Ssh => "ssh",
            Cost::Dump => "dump",
            Cost::Capture => "capture",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Spent {
    pub n: u64,
    pub ms: u64,
}

#[derive(Debug, Default)]
struct Capture {
    head: String,
    tail: String,
    len: usize,
}

impl Capture {
    fn push(&mut self, text: &str) {
        self.len += text.len();

        let room = floor(text, HEAD.saturating_sub(self.head.len()));
        self.head.push_str(&text[..room]);
        self.tail.push_str(&text[room..]);

        if self.tail.len() > 2 * TAIL {
            let cut = ceil(&self.tail, self.tail.len() - TAIL);
            self.tail.drain(..cut);
        }
    }

    fn text(&self) -> Option<String> {
        if self.len == 0 {
            return None;
        }

        let tail = &self.tail[ceil(&self.tail, self.tail.len().saturating_sub(TAIL))..];
        let cut = self.len - self.head.len() - tail.len();

        Some(match cut {
            0 => format!("{}{tail}", self.head),
            _ => format!("{}\n…[{cut} bytes cut]…\n{tail}", self.head),
        })
    }
}

fn floor(s: &str, at: usize) -> usize {
    let mut at = at.min(s.len());

    while !s.is_char_boundary(at) {
        at -= 1;
    }

    at
}

fn ceil(s: &str, at: usize) -> usize {
    let mut at = at.min(s.len());

    while !s.is_char_boundary(at) {
        at += 1;
    }

    at
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Seen {
    pub id: String,
    pub label: String,
    pub platform: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug)]
struct Call {
    started: Instant,
    at: u64,
    argv: Vec<String>,
    quiet: bool,
    out: Capture,
    err: Capture,
    piped: u64,
    device: Option<Seen>,
    project: Option<String>,
    tree: Option<String>,
    secrets: Vec<String>,
    plain_focused: bool,
    spent: BTreeMap<&'static str, Spent>,
}

#[derive(Debug, Default, Serialize)]
pub struct Version {
    pub exe: Option<String>,
    pub pkg: &'static str,
}

#[derive(Debug, Default, Serialize)]
pub struct Record {
    pub at: u64,
    pub ms: u64,
    pub exit: Option<i32>,
    pub version: Version,
    pub argv: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub ppid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<&'static str, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<Seen>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    pub out_len: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<String>,
    pub err_len: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub piped: u64,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub spent: BTreeMap<&'static str, Spent>,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

fn with(f: impl FnOnce(&mut Call)) {
    if let Ok(mut call) = CALL.lock() {
        if let Some(call) = call.as_mut() {
            f(call);
        }
    }
}

pub fn begin() {
    if std::env::var("PHONE_CALLS").is_ok_and(|v| v == "0") {
        return;
    }

    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();

    if let Ok(mut call) = CALL.lock() {
        *call = Some(Call {
            started: Instant::now(),
            at,
            argv: std::env::args().skip(1).collect(),
            quiet: false,
            out: Capture::default(),
            err: Capture::default(),
            piped: 0,
            device: None,
            project: None,
            tree: None,
            secrets: Vec::new(),
            plain_focused: false,
            spent: BTreeMap::new(),
        });
    }
}

pub fn skip() {
    if let Ok(mut call) = CALL.lock() {
        *call = None;
    }
}

pub fn quiet() {
    with(|c| c.quiet = true);
}

pub fn out(text: &str) {
    said(text, |c| &mut c.out);
}

pub fn err(text: &str) {
    said(text, |c| &mut c.err);
}

fn said(text: &str, to: fn(&mut Call) -> &mut Capture) {
    with(|c| {
        if !c.quiet {
            to(c).push(text)
        }
    });
}

pub fn piped(bytes: usize) {
    with(|c| c.piped += bytes as u64);
}

pub fn device(device: &Device) {
    with(|c| {
        c.device = Some(Seen {
            id: device.id.clone(),
            label: device.label.clone(),
            platform: device.platform.as_str(),
            host: device.host.clone(),
        })
    });
}

pub fn project(root: &Path) {
    with(|c| c.project = Some(root.display().to_string()));
}

pub fn tree(tree: &str) {
    with(|c| c.tree = Some(tree.to_string()));
}

pub fn secret(text: &str) {
    with(|c| push_secret(c, text));
}

fn push_secret(c: &mut Call, text: &str) {
    if !text.is_empty() && !c.secrets.iter().any(|s| s == text) {
        c.secrets.push(text.to_string());
    }
}

pub fn focused_plain(yes: bool) {
    with(|c| c.plain_focused = yes);
}

pub fn secret_if(yes: bool, text: &str) {
    if yes {
        secret(text);
    }
}

pub fn typed(text: &str) {
    with(|c| {
        if !c.plain_focused {
            push_secret(c, text)
        }
    });
}

pub struct Timer(Cost, Instant);

pub fn time(cost: Cost) -> Timer {
    Timer(cost, Instant::now())
}

impl Drop for Timer {
    fn drop(&mut self) {
        let ms = self.1.elapsed().as_millis() as u64;

        with(|c| {
            let spent = c.spent.entry(self.0.name()).or_default();
            spent.n += 1;
            spent.ms += ms;
        });
    }
}

pub fn clap_exit(e: &clap::Error) {
    let text = e.render().to_string();

    match e.use_stderr() {
        true => err(&text),
        false => out(&text),
    }

    finish(Some(e.exit_code()), None);
}

pub fn exec(cmd: &std::process::Command) {
    let words = std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|w| w.to_string_lossy().into_owned());

    finish(None, Some(shell_words::join(words)));
}

pub fn finish(exit: Option<i32>, exec: Option<String>) {
    let Some(call) = CALL.lock().ok().and_then(|mut c| c.take()) else {
        return;
    };

    let mut record = record(call, exit, exec);

    record.version.exe = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string());
    record.cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string());
    record.ppid = std::os::unix::process::parent_id();
    record.parent = std::fs::read_to_string(format!("/proc/{}/comm", record.ppid))
        .ok()
        .map(|s| s.trim().to_string());
    record.harness = harness()
        .and_then(|h| h.to_possible_value())
        .map(|v| v.get_name().to_string());
    record.session = lease::session();
    record.env = ENV
        .iter()
        .filter_map(|&name| Some((name, std::env::var(name).ok()?)))
        .collect();

    let _ = append(&registry::state_dir(), &record);
}

fn harness() -> Option<Harness> {
    let set = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty());

    if set("CLAUDECODE") {
        Some(Harness::Claude)
    } else if set("CODEX_THREAD_ID") || set("CODEX_SANDBOX") {
        Some(Harness::Codex)
    } else {
        None
    }
}

fn record(call: Call, exit: Option<i32>, exec: Option<String>) -> Record {
    let argv = redact(&call.argv, &call.secrets);
    let scrub = |text: Option<String>| {
        text.map(|mut text| {
            for (raw, safe) in call
                .argv
                .iter()
                .zip(&argv)
                .filter(|(raw, safe)| raw != safe)
            {
                text = text.replace(raw.as_str(), safe);
            }

            for secret in call.secrets.iter().filter(|s| s.chars().count() >= 4) {
                text = text.replace(secret.as_str(), &masked(secret));
            }

            text
        })
    };

    Record {
        at: call.at,
        ms: call.started.elapsed().as_millis() as u64,
        exit,
        version: Version {
            exe: None,
            pkg: env!("CARGO_PKG_VERSION"),
        },
        device: call.device,
        project: call.project,
        tree: call.tree,
        exec,
        out: scrub(call.out.text()),
        out_len: call.out.len,
        err: scrub(call.err.text()),
        err_len: call.err.len,
        piped: call.piped,
        spent: call.spent,
        argv,
        ..Record::default()
    }
}

fn masked(secret: &str) -> String {
    format!("<password, {} chars>", secret.chars().count())
}

fn redact(argv: &[String], secrets: &[String]) -> Vec<String> {
    argv.iter()
        .map(|arg| {
            if let Some(secret) = secrets.iter().find(|s| *s == arg) {
                return masked(secret);
            }

            if !secrets.iter().any(|s| arg.contains(s.as_str())) {
                return arg.clone();
            }

            match shell_words::split(arg) {
                Ok(words) => {
                    shell_words::join(words.iter().map(
                        |w| match secrets.iter().find(|s| *s == w) {
                            Some(secret) => masked(secret),
                            None => w.clone(),
                        },
                    ))
                }
                Err(_) => arg.clone(),
            }
        })
        .collect()
}

fn rotates(len: u64) -> bool {
    len > ROTATE_AT
}

fn append(dir: &Path, record: &Record) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(record)?;
    line.push(b'\n');

    std::fs::create_dir_all(dir)?;

    let path = dir.join("calls.jsonl");

    if std::fs::metadata(&path).is_ok_and(|m| rotates(m.len())) {
        std::fs::rename(&path, dir.join("calls.1.jsonl"))?;
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(&line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured(chunks: &[&str]) -> Capture {
        let mut capture = Capture::default();

        for chunk in chunks {
            capture.push(chunk);
        }

        capture
    }

    #[test]
    fn output_keeps_its_head_and_tail_and_says_what_was_cut() {
        let long = "x".repeat(HEAD + TAIL + 100);
        let lines: Vec<String> = (0..2000).map(|n| format!("{n}\n")).collect();
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let wide = "é".repeat(HEAD);

        for (chunks, len, cut) in [
            (vec![], 0, None),
            (vec!["changed    1 new\n"], 17, Some(0)),
            (vec![long.as_str()], long.len(), Some(100)),
            (
                lines.clone(),
                lines.concat().len(),
                Some(lines.concat().len() - HEAD - TAIL),
            ),
            (
                vec![wide.as_str()],
                wide.len(),
                Some(wide.len() - HEAD - TAIL),
            ),
        ] {
            let capture = captured(&chunks);
            let text = capture.text();
            let all = chunks.concat();

            assert_eq!(capture.len, len);

            match cut {
                None => assert_eq!(text, None),
                Some(0) => assert_eq!(text.as_deref(), Some(all.as_str())),
                Some(cut) => {
                    let text = text.unwrap();
                    let marker = format!("\n…[{cut} bytes cut]…\n");

                    assert!(text.contains(&marker), "{cut}");
                    assert!(all.starts_with(text.split(&marker).next().unwrap()));
                    assert!(all.ends_with(text.split(&marker).nth(1).unwrap()));
                    assert_eq!(text.len(), HEAD + TAIL + marker.len());
                }
            }
        }
    }

    #[test]
    fn a_password_is_replaced_by_its_length_wherever_it_was_typed() {
        let argv = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        let secrets = argv(&["hunter2"]);

        for (given, want) in [
            (
                argv(&["fill", "Password", "hunter2"]),
                argv(&["fill", "Password", "<password, 7 chars>"]),
            ),
            (
                argv(&["do", "tap Login", "fill Password hunter2", "tap Go"]),
                argv(&[
                    "do",
                    "tap Login",
                    "fill Password '<password, 7 chars>'",
                    "tap Go",
                ]),
            ),
            (argv(&["type", "hunter22"]), argv(&["type", "hunter22"])),
            (
                argv(&["fill", "Email", "me@x.io"]),
                argv(&["fill", "Email", "me@x.io"]),
            ),
        ] {
            assert_eq!(redact(&given, &secrets), want);
        }
    }

    #[test]
    fn a_record_is_one_line_with_the_password_scrubbed_from_its_output() {
        let mut call = Call {
            started: Instant::now(),
            at: 1_700_000_000_000,
            argv: vec!["do".into(), "fill Password hunter2".into()],
            quiet: false,
            out: Capture::default(),
            err: captured(&["  [1/1] fill Password hunter2\n"]),
            piped: 0,
            device: Some(Seen {
                id: "emulator-5554".into(),
                label: "pixel".into(),
                platform: "emu",
                host: None,
            }),
            project: None,
            tree: None,
            secrets: vec!["hunter2".into()],
            plain_focused: false,
            spent: BTreeMap::new(),
        };
        call.spent.insert("adb", Spent { n: 3, ms: 120 });

        let line = serde_json::to_string(&record(call, Some(0), None)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();

        assert!(!line.contains('\n') && !line.contains("hunter2"), "{line}");
        assert_eq!(value["argv"][1], "fill Password '<password, 7 chars>'");
        assert_eq!(
            value["err"],
            "  [1/1] fill Password '<password, 7 chars>'\n"
        );
        assert_eq!(value["device"]["label"], "pixel");
        assert_eq!(value["spent"]["adb"]["n"], 3);
        assert_eq!(value["exit"], 0);
        assert!(value.get("out").is_none() && value.get("project").is_none());
    }

    #[test]
    fn the_log_rotates_once_it_outgrows_its_cap() {
        for (len, want) in [(0, false), (ROTATE_AT, false), (ROTATE_AT + 1, true)] {
            assert_eq!(rotates(len), want, "{len}");
        }
    }
}
