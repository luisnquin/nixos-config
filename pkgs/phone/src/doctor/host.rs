use anyhow::Result;

use super::Check;
use crate::adb::{self, Server};
use crate::registry::{self, Registry};
use crate::{discover, hosts};

pub async fn checks(reg: &mut Registry, report: &mut impl FnMut(Check)) -> Result<()> {
    tools(report).await;
    ssh_hosts(reg, report).await
}

async fn tools(report: &mut impl FnMut(Check)) {
    let mut check = |ok: bool, name: &str, detail: String| report(Check::new(ok, name, detail));

    let adb_version = adb::run(&Server::Local, &["version"]).await;

    check(
        adb_version.as_ref().is_ok_and(|o| o.ok()),
        "adb",
        adb_version
            .as_ref()
            .ok()
            .and_then(|o| o.stdout.lines().next().map(str::to_string))
            .unwrap_or_else(|| "not on PATH".into()),
    );

    let attached = adb::devices(&Server::Local).await.unwrap_or_default();

    check(true, "transports", format!("{} attached", attached.len()));

    let key = registry::state_dir()
        .parent()
        .map(|_| dirs_adbkey())
        .unwrap_or_default();

    check(
        key.exists(),
        "adbkey",
        if key.exists() {
            key.display().to_string()
        } else {
            "missing; adb will generate one on first use".into()
        },
    );

    let peers = discover::tailscale::peers().await;

    match &peers {
        Ok(peers) => {
            let android = peers.iter().filter(|p| p.is_android()).count();
            let online = peers.iter().filter(|p| p.is_android() && p.online).count();

            check(
                true,
                "tailscale",
                format!("{android} android peer(s), {online} online"),
            );
        }
        Err(e) => check(false, "tailscale", e.to_string()),
    }

    // adb from nixpkgs is built without the bundled mDNS responder, so wireless
    // pairing depends entirely on the system's avahi
    let mdns = adb::run(&Server::Local, &["mdns", "check"]).await;
    let adb_mdns = mdns
        .as_ref()
        .is_ok_and(|o| !o.stderr.contains("not supported"));

    check(
        adb_mdns || which("avahi-browse"),
        "mdns",
        if adb_mdns {
            "adb has its own responder".into()
        } else if which("avahi-browse") {
            "via avahi-browse (adb has no responder)".into()
        } else {
            "no adb responder and no avahi-browse; pairing needs a manual addr".into()
        },
    );

    for tool in ["fzf", "scrcpy", "wl-copy", "notify-send"] {
        check(
            which(tool),
            tool,
            if which(tool) {
                "ok".into()
            } else {
                "not on PATH".into()
            },
        );
    }
}

async fn ssh_hosts(reg: &mut Registry, report: &mut impl FnMut(Check)) -> Result<()> {
    let mut check = |ok: bool, name: &str, detail: String| report(Check::new(ok, name, detail));

    let known = hosts::discover().await;
    reg.sync_hosts(&known.iter().map(|h| h.name.clone()).collect::<Vec<_>>());

    let enabled: Vec<String> = reg
        .enabled_hosts()
        .iter()
        .map(|h| format!("{} ({})", h.name, h.caps.label()))
        .collect();

    check(
        true,
        "ssh hosts",
        if enabled.is_empty() {
            format!(
                "{} in your ssh config, none enabled; `phone host enable NAME`",
                known.len()
            )
        } else {
            enabled.join(", ")
        },
    );

    for state in reg
        .hosts
        .iter()
        .filter(|h| h.enabled)
        .cloned()
        .collect::<Vec<_>>()
    {
        match hosts::probe(&state.name).await {
            Some(caps) if caps == state.caps => {
                check(true, &state.name, format!("still {}", caps.label()))
            }
            Some(caps) => check(
                false,
                &state.name,
                format!("now {} (was {})", caps.label(), state.caps.label()),
            ),
            None => check(false, &state.name, "unreachable over ssh".into()),
        }
    }

    reg.save()
}

fn dirs_adbkey() -> std::path::PathBuf {
    std::env::var_os("ANDROID_VENDOR_KEYS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".android/adbkey")
        })
}

fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}
