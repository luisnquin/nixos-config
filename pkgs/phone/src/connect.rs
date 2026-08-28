use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::sync::mpsc::UnboundedSender;

use crate::adb::{self, Server};
use crate::discover::{avahi, scoped, split_addr, sweep, tailscale};
use crate::model::{Device, Pin};
use crate::registry::Registry;

#[derive(Clone, Debug)]
pub enum Step {
    Try(String),
    Done(String),
    Fail(String),
    Note(String),
    Progress { done: usize, total: usize },
}

#[derive(Clone)]
pub struct Reporter(Option<UnboundedSender<Step>>);

impl Reporter {
    pub fn new(tx: UnboundedSender<Step>) -> Self {
        Self(Some(tx))
    }

    pub fn send(&self, step: Step) {
        if let Some(tx) = &self.0 {
            let _ = tx.send(step);
        }
    }

    pub fn try_(&self, msg: impl Into<String>) {
        self.send(Step::Try(msg.into()));
    }

    pub fn done(&self, msg: impl Into<String>) {
        self.send(Step::Done(msg.into()));
    }

    pub fn fail(&self, msg: impl Into<String>) {
        self.send(Step::Fail(msg.into()));
    }

    pub fn note(&self, msg: impl Into<String>) {
        self.send(Step::Note(msg.into()));
    }
}

#[derive(Clone, Debug)]
pub struct Opts {
    pub sweep: bool,
    pub range: RangeInclusive<u16>,
    pub concurrency: usize,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            sweep: true,
            range: sweep::EPHEMERAL,
            concurrency: sweep::Sweep::default().concurrency,
        }
    }
}

/// Walks every way of reaching `device`, cheapest first, and returns the adb
/// serial of the transport that came up.
pub async fn connect(
    reg: &mut Registry,
    server: &Server,
    device: &Device,
    opts: &Opts,
    rep: &Reporter,
) -> Result<String> {
    if !device.platform.is_adb() {
        bail!("{} is not an adb device", device.label);
    }

    if let Some(serial) = attached_serial(server, device).await {
        rep.done(format!("already attached as {serial}"));
        reg.touch(&device.id);
        reg.save()?;

        return Ok(serial);
    }

    if let Some(serial) = try_history(reg, server, device, rep).await? {
        return Ok(serial);
    }

    if let Some(serial) = try_avahi(reg, server, device, rep).await? {
        return Ok(serial);
    }

    if opts.sweep {
        if let Some(serial) = try_sweep(reg, server, device, opts, rep).await? {
            return Ok(serial);
        }
    }

    rep.note("plug it in over USB and run `phone device pin` to fix a port for next time");

    Err(anyhow!("no route to {}", device.label))
}

/// Compared through the scoped id, because a serial only identifies a device
/// within the server that reported it.
pub async fn attached_serial(server: &Server, device: &Device) -> Option<String> {
    let attached = adb::devices(server).await.ok()?;

    attached
        .into_iter()
        .filter(|a| a.state == "device")
        .find(|a| {
            let key = scoped(server, &a.serial);

            key == device.id
                || device.aliases.contains(&key)
                || device.endpoints.iter().any(|e| e.addr() == a.serial)
        })
        .map(|a| a.serial)
}

async fn try_history(
    reg: &mut Registry,
    server: &Server,
    device: &Device,
    rep: &Reporter,
) -> Result<Option<String>> {
    let ranked: Vec<(String, u16, Pin)> = device
        .ranked_endpoints()
        .into_iter()
        .map(|e| (e.host.clone(), e.port, e.pin))
        .collect();

    if ranked.is_empty() {
        return Ok(None);
    }

    rep.try_(format!("history: {} endpoint(s)", ranked.len()));

    // `adb connect` costs seconds per miss, a TCP probe one round trip
    let mut probes = tokio::task::JoinSet::new();

    for (i, (host, port, _)) in ranked.iter().enumerate() {
        let host = host.clone();
        let port = *port;

        probes.spawn(async move {
            (
                i,
                sweep::probe(&host, port, Duration::from_millis(700)).await,
            )
        });
    }

    let mut live = vec![false; ranked.len()];

    while let Some(Ok((i, hit))) = probes.join_next().await {
        live[i] = hit;
    }

    for (i, (host, port, pin)) in ranked.iter().enumerate() {
        if !live[i] {
            continue;
        }

        let addr = format!("{host}:{port}");

        rep.try_(format!("connect {addr}"));

        match adb::connect(server, &addr).await {
            Ok(()) => {
                remember(reg, device, host, *port, *pin);
                reg.save()?;
                rep.done(format!("connected {addr}"));

                return Ok(Some(addr));
            }
            Err(e) => rep.fail(format!("{addr}: {e}")),
        }
    }

    rep.fail("history: nothing answered");

    Ok(None)
}

async fn try_avahi(
    reg: &mut Registry,
    server: &Server,
    device: &Device,
    rep: &Reporter,
) -> Result<Option<String>> {
    rep.try_("mdns: browsing the local network");

    let services = avahi::browse(avahi::CONNECT_SERVICE, Duration::from_secs(3)).await;

    let hit = services.iter().find(|s| {
        s.serial()
            .is_some_and(|serial| serial == device.id || device.aliases.iter().any(|a| a == serial))
    });

    let Some(service) = hit else {
        rep.fail(if services.is_empty() {
            "mdns: nothing advertising wireless debugging".to_string()
        } else {
            format!("mdns: {} device(s), none matching", services.len())
        });

        return Ok(None);
    };

    let addr = service.addr();

    rep.try_(format!("connect {addr}"));

    match adb::connect(server, &addr).await {
        Ok(()) => {
            remember(reg, device, &service.host, service.port, Pin::None);
            reg.save()?;
            rep.done(format!("connected {addr}"));

            Ok(Some(addr))
        }
        Err(e) => {
            rep.fail(format!("{addr}: {e}"));

            Ok(None)
        }
    }
}

async fn try_sweep(
    reg: &mut Registry,
    server: &Server,
    device: &Device,
    opts: &Opts,
    rep: &Reporter,
) -> Result<Option<String>> {
    // sweeping an address a source reports down means waiting out the timeout
    // on every port in the range
    let peers = tailscale::peers().await.unwrap_or_default();

    let targets: Vec<String> = device
        .hosts()
        .into_iter()
        .filter(|host| !unreachable(&peers, host))
        .collect();

    if targets.is_empty() {
        rep.fail(format!("sweep: no address to scan for {}", device.label));

        return Ok(None);
    }

    for host in targets {
        rep.try_(format!(
            "sweep {host} ports {}-{}",
            opts.range.start(),
            opts.range.end()
        ));

        let scanner = sweep::Sweep {
            concurrency: opts.concurrency,
            ..Default::default()
        };

        let rep_progress = rep.clone();

        let open = scanner
            .scan(&host, opts.range.clone(), move |done, total| {
                rep_progress.send(Step::Progress { done, total });
            })
            .await?;

        if open.is_empty() {
            rep.fail(format!("sweep: no open ports on {host}"));

            continue;
        }

        rep.note(format!("sweep: {} candidate port(s)", open.len()));

        for port in open {
            let addr = format!("{host}:{port}");

            rep.try_(format!("connect {addr}"));

            if adb::connect(server, &addr).await.is_ok() {
                remember(reg, device, &host, port, Pin::None);
                reg.save()?;
                rep.done(format!("connected {addr}"));

                return Ok(Some(addr));
            }
        }
    }

    rep.fail("sweep: no candidate spoke adb");

    Ok(None)
}

/// An address nothing knows about is not ruled out: no source claiming it is
/// not the same as one claiming it is gone.
fn unreachable(peers: &[tailscale::Peer], host: &str) -> bool {
    peers.iter().any(|p| p.ip == host && !p.online)
}

fn routable(peers: &[tailscale::Peer], ip: &str) -> bool {
    peers.iter().any(|p| p.ip == ip && p.online)
}

fn remember(reg: &mut Registry, device: &Device, host: &str, port: u16, pin: Pin) {
    let entry = match reg.get_mut(&device.id) {
        Some(entry) => entry,
        None => {
            reg.upsert(device.clone());
            reg.get_mut(&device.id).expect("just upserted")
        }
    };

    entry.record_endpoint(host, port, pin);
    entry.last_connected = Some(crate::model::now());
}

/// Restart adbd on a fixed port so later reconnects skip discovery. Lasts until
/// reboot, or forever if root can set the persistent property.
pub async fn pin(
    reg: &mut Registry,
    server: &Server,
    device: &Device,
    port: u16,
    rep: &Reporter,
) -> Result<()> {
    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    rep.try_(format!("adb tcpip {port}"));
    adb::tcpip(server, &serial, port).await?;

    // adbd drops every transport while it rebinds
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let host = host_for(reg, server, device, &serial).await?;
    let addr = format!("{host}:{port}");

    rep.try_(format!("connect {addr}"));
    adb::connect(server, &addr).await?;

    let persistent = adb::run_timeout(
        server,
        &[
            "-s",
            &addr,
            "shell",
            "su",
            "-c",
            &format!("setprop persist.adb.tcp.port {port}"),
        ],
        Duration::from_secs(6),
    )
    .await
    .map(|o| o.ok())
    .unwrap_or(false);

    let pin_kind = if persistent {
        rep.note("persisted across reboots via persist.adb.tcp.port");
        Pin::Persistent
    } else {
        rep.note("holds until the device reboots (no root)");
        Pin::Session
    };

    remember(reg, device, &host, port, pin_kind);
    reg.save()?;
    rep.done(format!("pinned {addr}"));

    Ok(())
}

/// Where to dial once adbd rebinds. An address a discovery source advertises
/// beats whatever the LAN hands out, because it stays valid from anywhere.
async fn host_for(
    reg: &Registry,
    server: &Server,
    device: &Device,
    serial: &str,
) -> Result<String> {
    let mut known = device.hosts();

    if let Some(stored) = reg.get(&device.id) {
        for host in stored.hosts() {
            if !known.contains(&host) {
                known.push(host);
            }
        }
    }

    if let Ok(peers) = tailscale::peers().await {
        if let Some(host) = known.into_iter().find(|h| routable(&peers, h)) {
            return Ok(host);
        }
    }

    if let Some((host, _)) = split_addr(serial) {
        return Ok(host);
    }

    let out = adb::run_timeout(
        server,
        &["-s", serial, "shell", "ip", "route", "get", "1.1.1.1"],
        Duration::from_secs(6),
    )
    .await?;

    out.stdout
        .split_whitespace()
        .skip_while(|w| *w != "src")
        .nth(1)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("could not work out the device's own address"))
}

/// Wireless debugging advertises pairing on a different ephemeral port than
/// connect, so the code alone is never enough to reach it.
pub async fn pair(
    server: &Server,
    addr: Option<&str>,
    code: &str,
    rep: &Reporter,
) -> Result<String> {
    let addr = match addr {
        Some(addr) => addr.to_string(),
        None => {
            rep.try_("mdns: looking for a device in pairing mode");

            let found = avahi::browse(avahi::PAIRING_SERVICE, Duration::from_secs(5)).await;

            let service = found.into_iter().next().ok_or_else(|| {
                anyhow!(
                    "no device is advertising pairing; open Wireless debugging > \
                     Pair device with pairing code, and stay on the same network"
                )
            })?;

            rep.note(format!("found {} at {}", service.name, service.addr()));

            service.addr()
        }
    };

    rep.try_(format!("pair {addr}"));
    adb::pair(server, &addr, code).await?;
    rep.done(format!("paired with {addr}"));

    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(ip: &str, online: bool) -> tailscale::Peer {
        tailscale::Peer {
            hostname: "moriarty".into(),
            ip: ip.into(),
            node_id: "nXXXX".into(),
            os: "android".into(),
            online,
            last_seen: None,
        }
    }

    #[test]
    fn an_offline_peer_is_not_worth_sweeping() {
        let peers = [peer("100.75.85.98", false)];

        assert!(!routable(&peers, "100.75.85.98"));
    }

    #[test]
    fn an_online_peer_is() {
        let peers = [peer("100.75.85.98", true)];

        assert!(routable(&peers, "100.75.85.98"));
    }

    #[test]
    fn an_address_the_tailnet_no_longer_lists_is_not() {
        let peers = [peer("100.127.25.101", true)];

        assert!(!routable(&peers, "100.75.85.98"));
    }
}
