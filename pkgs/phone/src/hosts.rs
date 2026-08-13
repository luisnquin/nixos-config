use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::model::{now, Unix};
use crate::ssh;

/// The adb server's own port on every host that runs one.
pub const ADB_SERVER_PORT: u16 = 5037;

/// What a host was last seen to offer. Probed on enable rather than on every
/// survey: the probe is a whole ssh round trip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caps {
    #[serde(default)]
    pub adb: bool,
    #[serde(default)]
    pub simctl: bool,
    #[serde(default)]
    pub tunneld: bool,
}

impl Caps {
    pub fn any(self) -> bool {
        self.adb || self.simctl || self.tunneld
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();

        if self.adb {
            parts.push("adb");
        }

        if self.simctl {
            parts.push("simctl");
        }

        if self.tunneld {
            parts.push("tunneld");
        }

        if parts.is_empty() {
            "nothing to drive".into()
        } else {
            parts.join(" ")
        }
    }
}

/// The only thing `phone` stores about a host: which ones to look at and where
/// each adb forward landed. Everything else stays in ssh's config.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HostState {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub tunnel_port: Option<u16>,
    #[serde(default)]
    pub caps: Caps,
    #[serde(default)]
    pub probed: Option<Unix>,
}

impl HostState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// An ssh alias plus what ssh resolved it to.
#[derive(Clone, Debug)]
pub struct Host {
    pub name: String,
    pub target: String,
}

/// Mined only for the names worth resolving, because ssh has no "list every
/// host you know" mode. `ssh -G` does the resolving.
fn config_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        files.push(PathBuf::from(home).join(".ssh/config"));
    }

    files.push(PathBuf::from("/etc/ssh/ssh_config"));

    let mut out = Vec::new();

    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };

        out.push(file);

        // one level deep; nested includes are rare enough to skip
        for line in text.lines() {
            let Some(rest) = keyword(line, "include") else {
                continue;
            };

            for path in rest.split_whitespace() {
                let path = expand(path);

                if path.exists() {
                    out.push(path);
                }
            }
        }
    }

    out
}

fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => PathBuf::from(path),
        },
        None => PathBuf::from(path),
    }
}

/// Case-insensitive, and tolerates the `Key=value` form ssh also accepts.
fn keyword<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    let line = line.trim();
    let (key, rest) = line.split_once([' ', '\t', '='])?;

    key.eq_ignore_ascii_case(word).then(|| rest.trim())
}

/// Every literal alias named by a `Host` stanza. A wildcard is a rule about
/// hosts, not a host, and resolves to nothing anyone can connect to.
pub fn aliases() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for file in config_files() {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };

        for line in text.lines() {
            let Some(rest) = keyword(line, "host") else {
                continue;
            };

            for alias in rest.split_whitespace() {
                let literal = !alias.contains(['*', '?', '!']);

                if literal && !out.iter().any(|a| a == alias) {
                    out.push(alias.to_string());
                }
            }
        }
    }

    out
}

/// What ssh thinks `alias` means, as the `user@host:port` it would dial.
pub async fn resolve(alias: &str) -> Option<String> {
    let out = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new("ssh")
            .args(["-G", alias])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    target(&String::from_utf8_lossy(&out.stdout))
}

fn target(resolved: &str) -> Option<String> {
    let mut hostname = None;
    let mut port = None;
    let mut user = None;

    for line in resolved.lines() {
        match line.split_once(' ') {
            Some(("hostname", v)) if hostname.is_none() => hostname = Some(v.trim().to_string()),
            Some(("port", v)) if port.is_none() => port = Some(v.trim().to_string()),
            Some(("user", v)) if user.is_none() => user = Some(v.trim().to_string()),
            _ => {}
        }
    }

    let hostname = hostname.filter(|h| !h.is_empty())?;

    Some(format!(
        "{}@{hostname}:{}",
        user.unwrap_or_default(),
        port.unwrap_or_default()
    ))
}

/// Every ssh host worth offering, one row per machine. Aliases resolving to the
/// same `user@host:port` are one box, and surveying it twice doubles its
/// devices.
pub async fn discover() -> Vec<Host> {
    let mut tasks = tokio::task::JoinSet::new();

    for (i, alias) in aliases().into_iter().enumerate() {
        tasks.spawn(async move { (i, resolve(&alias).await, alias) });
    }

    let mut found: Vec<(usize, String, String)> = Vec::new();

    while let Some(Ok((i, target, alias))) = tasks.join_next().await {
        if let Some(target) = target {
            found.push((i, target, alias));
        }
    }

    // JoinSet finishes out of order; config order is what the user recognises
    found.sort_by_key(|(i, _, _)| *i);

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for (_, target, name) in found {
        if seen.insert(target.clone()) {
            out.push(Host { name, target });
        }
    }

    out
}

/// What `host` can drive, or `None` if it never answered — a host with no SDK
/// is one to stop asking about, an unreachable one is a route to go fix.
///
/// A non-interactive ssh reads no login profile, so the login shell is asked
/// for its own PATH.
pub async fn probe(host: &str) -> Option<Caps> {
    const SCRIPT: &str = r#"echo up
PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
command -v adb >/dev/null 2>&1 && echo adb
command -v xcrun >/dev/null 2>&1 && xcrun simctl help >/dev/null 2>&1 && echo simctl
curl -s --max-time 2 -o /dev/null http://127.0.0.1:49151/ && echo tunneld
exit 0"#;

    let text = ssh::text(host, SCRIPT, Duration::from_secs(25)).await;
    let said = |word: &str| text.lines().any(|l| l == word);

    said("up").then(|| Caps {
        adb: said("adb"),
        simctl: said("simctl"),
        tunneld: said("tunneld"),
    })
}

/// The local port the host's adb forward landed on, remembered so the next
/// `phone` process reuses the tunnel instead of opening a second one.
pub async fn adb_tunnel(state: &mut HostState) -> anyhow::Result<u16> {
    if let Some(port) = state.tunnel_port {
        if ssh::forward(&state.name, port, ADB_SERVER_PORT)
            .await
            .is_ok()
        {
            return Ok(port);
        }
    }

    // the remembered port was taken by something else in the meantime
    let port = ssh::free_port().await?;

    ssh::forward(&state.name, port, ADB_SERVER_PORT).await?;

    state.tunnel_port = Some(port);

    Ok(port)
}

pub fn stamp(state: &mut HostState, caps: Caps) {
    state.caps = caps;
    state.probed = Some(now());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_triple_ssh_resolved() {
        let resolved = "user luisnquin\nhostname rose\nport 22\naddkeystoagent false\n";

        assert_eq!(target(resolved).as_deref(), Some("luisnquin@rose:22"));
    }

    #[test]
    fn an_alias_with_no_hostname_resolves_to_nothing() {
        assert_eq!(target("user luisnquin\nport 22\n"), None);
    }

    #[test]
    fn keyword_accepts_both_separators() {
        assert_eq!(keyword("  Host rose", "host"), Some("rose"));
        assert_eq!(keyword("Include=/etc/ssh/x", "include"), Some("/etc/ssh/x"));
        assert_eq!(keyword("HostName rose.local", "host"), None);
    }
}
