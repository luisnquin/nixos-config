use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Harness {
    Claude,
    Codex,
    Env,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub label: String,
    pub host: String,
    pub harness: Harness,
    pub pid: Option<u32>,
    pub start: Option<String>,
    pub shared: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Env {
    pub phone_agent: Option<String>,
    pub claudecode: bool,
    pub claude_session: Option<String>,
    pub claude_pid: Option<u32>,
    pub codex_thread: Option<String>,
    pub codex_session: Option<String>,
    pub codex_pid: Option<u32>,
    pub user: Option<String>,
}

impl Env {
    fn read() -> Self {
        let claudecode = var("CLAUDECODE").is_some();
        let codex_thread = var("CODEX_THREAD_ID");

        Env {
            phone_agent: var("PHONE_AGENT"),
            claudecode,
            claude_session: var("CLAUDE_CODE_SESSION_ID"),
            claude_pid: match claudecode {
                true => var("CLAUDE_PID")
                    .and_then(|p| p.parse().ok())
                    .or_else(|| ancestor("claude")),
                false => None,
            },
            codex_pid: codex_thread.as_ref().and_then(|_| ancestor("codex")),
            codex_thread,
            codex_session: var("CODEX_SESSION_ID"),
            user: var("USER"),
        }
    }
}

fn var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn me() -> &'static Agent {
    static ME: OnceLock<Agent> = OnceLock::new();

    ME.get_or_init(|| {
        let env = Env::read();
        let host = hostname();

        resolve(&env, &host, start)
    })
}

pub fn resolve(env: &Env, host: &str, start: impl Fn(u32) -> Option<String>) -> Agent {
    let mut agent = identify(env, host);

    agent.start = agent.pid.and_then(&start);

    if agent.start.is_none() {
        agent.pid = None;
    }

    agent
}

fn identify(env: &Env, host: &str) -> Agent {
    let of = |id: String, label: String, harness: Harness, pid: Option<u32>| Agent {
        id,
        label,
        host: host.to_string(),
        harness,
        pid,
        start: None,
        shared: false,
    };

    if let Some(id) = &env.phone_agent {
        let (harness, pid) = match (env.claudecode, &env.codex_thread) {
            (true, _) => (Harness::Claude, env.claude_pid),
            (false, Some(_)) => (Harness::Codex, env.codex_pid),
            (false, None) => (Harness::Env, None),
        };

        return of(id.clone(), short(id.rsplit('/').next().unwrap_or(id)), harness, pid);
    }

    if let Some(thread) = &env.codex_thread {
        let label = short(env.codex_session.as_deref().unwrap_or(thread));

        return of(format!("codex:{thread}"), label, Harness::Codex, env.codex_pid);
    }

    if let (true, Some(session)) = (env.claudecode, &env.claude_session) {
        let mut agent = of(session.clone(), short(session), Harness::Claude, env.claude_pid);
        agent.shared = true;

        return agent;
    }

    let user = format!("{}@{host}", env.user.as_deref().unwrap_or("someone"));

    of(user.clone(), user, Harness::User, None)
}

pub fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

pub fn hostname() -> String {
    std::process::Command::new("uname")
        .arg("-n")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

fn ancestor(name: &str) -> Option<u32> {
    let mut pid = std::process::id();

    for _ in 0..32 {
        let (parent, comm) = parent(pid)?;

        if comm.contains(name) {
            return Some(parent);
        }

        if parent <= 1 {
            return None;
        }

        pid = parent;
    }

    None
}

fn parent(pid: u32) -> Option<(u32, String)> {
    let ppid = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => after_comm(&stat)?.get(1)?.parse().ok()?,
        Err(_) => ps(pid, "ppid=")?.parse().ok()?,
    };

    let comm = match std::fs::read_to_string(format!("/proc/{ppid}/comm")) {
        Ok(comm) => comm.trim().to_string(),
        Err(_) => ps(ppid, "comm=")?,
    };

    Some((ppid, comm))
}

fn after_comm(stat: &str) -> Option<Vec<&str>> {
    Some(stat.rsplit_once(')')?.1.split_whitespace().collect())
}

fn ps(pid: u32, field: &str) -> Option<String> {
    let out = std::process::Command::new("ps")
        .args(["-o", field, "-p", &pid.to_string()])
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

pub fn start(pid: u32) -> Option<String> {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => Some(after_comm(&stat)?.get(19)?.to_string()),
        Err(_) => Some(ps(pid, "lstart=")?.split_whitespace().collect::<Vec<_>>().join("_")),
    }
}

pub fn alive(pid: u32, started: &str) -> bool {
    start(pid).is_some_and(|s| s == started)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Env {
        Env {
            user: Some("luis".into()),
            ..Env::default()
        }
    }

    fn started(_: u32) -> Option<String> {
        Some("123".into())
    }

    #[test]
    fn phone_agent_wins_over_everything() {
        let e = Env {
            phone_agent: Some("1f636eb3-8a91/a6a8204d5dbde1ddc".into()),
            claudecode: true,
            claude_session: Some("1f636eb3-8a91".into()),
            claude_pid: Some(42),
            codex_thread: Some("t1".into()),
            ..env()
        };
        let a = resolve(&e, "nyx", started);

        assert_eq!(a.id, "1f636eb3-8a91/a6a8204d5dbde1ddc");
        assert_eq!(a.label, "a6a8204d");
        assert_eq!((a.harness, a.pid, a.start.as_deref()), (Harness::Claude, Some(42), Some("123")));
        assert!(!a.shared);
    }

    #[test]
    fn a_hand_set_phone_agent_is_ttl_only() {
        let e = Env {
            phone_agent: Some("mine".into()),
            ..env()
        };
        let a = resolve(&e, "nyx", started);

        assert_eq!((a.id.as_str(), a.harness, a.pid), ("mine", Harness::Env, None));
    }

    #[test]
    fn codex_is_keyed_by_thread_and_labelled_by_session() {
        let e = Env {
            codex_thread: Some("thread-b".into()),
            codex_session: Some("0199aaaa-bbbb".into()),
            codex_pid: Some(7),
            ..env()
        };
        let a = resolve(&e, "nyx", started);

        assert_eq!(a.id, "codex:thread-b");
        assert_eq!(a.label, "0199aaaa");
        assert_eq!((a.harness, a.pid), (Harness::Codex, Some(7)));
    }

    #[test]
    fn claude_without_the_hook_shares_the_session() {
        let e = Env {
            claudecode: true,
            claude_session: Some("1f636eb3-8a91".into()),
            claude_pid: Some(42),
            ..env()
        };
        let a = resolve(&e, "nyx", started);

        assert_eq!((a.id.as_str(), a.label.as_str()), ("1f636eb3-8a91", "1f636eb3"));
        assert!(a.shared);
    }

    #[test]
    fn a_person_at_a_terminal_is_user_at_host() {
        let a = resolve(&env(), "nyx", started);

        assert_eq!((a.id.as_str(), a.harness, a.pid), ("luis@nyx", Harness::User, None));
    }

    #[test]
    fn a_pid_whose_start_cannot_be_read_is_dropped() {
        let e = Env {
            codex_thread: Some("t".into()),
            codex_pid: Some(7),
            ..env()
        };

        assert_eq!(resolve(&e, "nyx", |_| None).pid, None);
    }

    #[test]
    fn this_process_is_alive_and_a_wrong_start_is_not() {
        let me = std::process::id();
        let started = start(me).unwrap();

        assert!(alive(me, &started));
        assert!(!alive(me, "0"));
    }
}
