use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::Deserialize;
use tokio::process::Command;

use crate::model::Unix;

#[derive(Clone, Debug)]
pub struct Peer {
    pub hostname: String,
    pub ip: String,
    pub node_id: String,
    pub os: String,
    pub online: bool,
    pub last_seen: Option<Unix>,
}

impl Peer {
    pub fn is_android(&self) -> bool {
        self.os.eq_ignore_ascii_case("android")
    }
}

#[derive(Deserialize)]
struct StatusJson {
    #[serde(default, rename = "Peer")]
    peer: std::collections::HashMap<String, PeerJson>,
}

#[derive(Deserialize)]
struct PeerJson {
    #[serde(default, rename = "HostName")]
    host_name: String,
    #[serde(default, rename = "DNSName")]
    dns_name: String,
    #[serde(default, rename = "TailscaleIPs")]
    ips: Vec<String>,
    #[serde(default, rename = "OS")]
    os: String,
    #[serde(default, rename = "Online")]
    online: bool,
    #[serde(default, rename = "LastSeen")]
    last_seen: Option<String>,
    #[serde(default, rename = "ID")]
    id: String,
}

pub async fn peers() -> Result<Vec<Peer>> {
    let out = tokio::time::timeout(
        Duration::from_secs(8),
        Command::new("tailscale")
            .args(["status", "--json"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output(),
    )
    .await??;

    if !out.status.success() {
        bail!("tailscale status failed (is tailscaled up?)");
    }

    let status: StatusJson = serde_json::from_slice(&out.stdout)?;

    let mut peers: Vec<Peer> = status
        .peer
        .into_values()
        .map(|p| Peer {
            // HostName arrives capitalised as the device set it ("Watson");
            // the MagicDNS label is what actually resolves and what gets typed.
            hostname: dns_label(&p.dns_name).unwrap_or_else(|| p.host_name.to_lowercase()),
            ip: p
                .ips
                .iter()
                .find(|ip| ip.contains('.'))
                .cloned()
                .unwrap_or_default(),
            node_id: p.id,
            os: p.os,
            online: p.online,
            last_seen: p.last_seen.as_deref().and_then(parse_rfc3339),
        })
        .filter(|p| !p.ip.is_empty())
        .collect();

    peers.sort_by(|a, b| b.online.cmp(&a.online).then(a.hostname.cmp(&b.hostname)));

    Ok(peers)
}

fn dns_label(dns_name: &str) -> Option<String> {
    let label = dns_name.split('.').next()?.trim();

    (!label.is_empty()).then(|| label.to_lowercase())
}

fn parse_rfc3339(s: &str) -> Option<Unix> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
        // tailscale zeroes LastSeen for peers that have never been seen
        .filter(|t| *t > 0)
}
