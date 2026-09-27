use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::sync::mpsc::UnboundedSender;

use crate::adb::{self, Server};
use crate::discover::{avahi, scoped, split_addr, sweep, tailscale};
use crate::model::{Device, Pin, Platform, Reach, View};
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

pub async fn serial_of(server: &Server, device: &Device) -> Result<String> {
    attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!(unattached(device)))
}

pub fn unattached(device: &Device) -> String {
    let label = &device.label;
    let q = crate::quoted(label);

    match device.platform {
        Platform::Emulator => format!(
            "{label} is not attached: if it is running, `phone device connect {q}`; \
             if `phone device list` shows it off, `phone device boot {q}`"
        ),
        _ => format!(
            "{label} is not attached: plug it in over USB, or turn on wireless debugging \
             and `phone device connect {q}`"
        ),
    }
}

/// The survey already knows why a device has no transport, which a lookup made
/// later, from the device alone, cannot.
pub fn stranded(views: &[View], view: &View) -> Option<String> {
    let device = &view.device;

    if !device.platform.is_adb() || view.reach.is_attached() {
        return None;
    }

    let label = &device.label;
    let q = crate::quoted(label);

    let said = match (&view.reach, device.platform) {
        (Reach::Unauthorized { .. }, _) => format!(
            "{label} is attached but has not authorized this machine: accept the USB debugging prompt on its screen, then retry"
        ),
        (Reach::Online, _) => format!(
            "{label} is on the network with no adb transport: `phone device connect {q}`"
        ),
        (_, Platform::Emulator) => {
            let host = device.host.as_deref().unwrap_or("this machine");

            let id = &device.id;
            let avds: Vec<&View> = views
                .iter()
                .filter(|v| {
                    v.device.platform == Platform::Emulator
                        && v.device.host == device.host
                        && v.device.id != device.id
                        && v.reach != Reach::Known
                })
                .collect();

            if let Some(live) = avds.iter().find(|v| v.device.is(label)) {
                return Some(format!(
                    "{id} is a stale row of {label}, which {host} lists as {} ({}): \
                     `-t {}`, or `phone device forget {id}` drops the stale row",
                    live.device.id,
                    live.reach.label(),
                    live.device.id
                ));
            }

            let instead = match avds.is_empty() {
                true => format!("{host} lists no AVD at all, so it may be unreachable (`phone doctor`)"),
                false => format!(
                    "{host} has {}",
                    avds.iter()
                        .map(|v| crate::quoted(&v.device.label))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            };

            format!(
                "{label} is a remembered emulator that no AVD on {host} answers to now, likely a stale row: \
                 {instead}; `phone device forget {id}` drops it"
            )
        }
        _ => unattached(device),
    };

    Some(said)
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
    let serial = serial_of(server, device).await?;

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

    fn on_rose(id: &str, label: &str, reach: Reach) -> View {
        let mut device = Device::new(id, label, Platform::Emulator);

        device.host = Some("rose".into());

        View::new(device, reach)
    }

    #[test]
    fn a_remembered_emulator_names_the_avd_it_is_a_stale_row_of() {
        let stale = on_rose("android_id:2222bbbb", "pixel_7-api36", Reach::Known);
        let views = vec![
            on_rose("avd:rose/pixel_7-api36", "pixel_7-api36", Reach::Off),
            stale.clone(),
        ];

        assert_eq!(
            stranded(&views, &stale).unwrap(),
            "android_id:2222bbbb is a stale row of pixel_7-api36, which rose lists as \
             avd:rose/pixel_7-api36 (off): `-t avd:rose/pixel_7-api36`, or \
             `phone device forget android_id:2222bbbb` drops the stale row"
        );
    }

    #[test]
    fn a_remembered_emulator_no_avd_answers_to_lists_the_ones_that_do() {
        let gone = on_rose("android_id:9999", "nyx-remote-android", Reach::Known);
        let views = vec![
            on_rose("avd:rose/pixel_7-api36", "pixel_7-api36", Reach::Off),
            gone.clone(),
        ];

        let said = stranded(&views, &gone).unwrap();

        assert!(said.contains("no AVD on rose answers to now"));
        assert!(said.contains("rose has pixel_7-api36"));
        assert!(said.ends_with("`phone device forget android_id:9999` drops it"));

        let alone = stranded(std::slice::from_ref(&gone), &gone).unwrap();

        assert!(alone.contains("rose lists no AVD at all"));
    }

    #[test]
    fn every_missing_transport_names_its_next_step() {
        let phone = Device::new("serial", "pixel-9", Platform::Android);
        let listed = View::new(phone.clone(), Reach::Online);
        let prompt = View::new(
            phone.clone(),
            Reach::Unauthorized {
                serial: "serial".into(),
            },
        );
        let attached = View::new(
            phone.clone(),
            Reach::Attached {
                serial: "serial".into(),
                wireless: false,
            },
        );

        assert!(stranded(&[], &listed)
            .unwrap()
            .ends_with("`phone device connect pixel-9`"));
        assert!(stranded(&[], &prompt)
            .unwrap()
            .contains("accept the USB debugging prompt"));
        assert!(stranded(&[], &View::new(phone.clone(), Reach::Known))
            .unwrap()
            .contains("plug it in over USB"));
        assert_eq!(stranded(&[], &attached), None);
        assert_eq!(
            stranded(
                &[],
                &View::new(
                    Device::new("u", "iPad (A16)", Platform::Simulator),
                    Reach::Off
                )
            ),
            None
        );

        assert!(
            unattached(&Device::new("avd:x", "pixel_7-api36", Platform::Emulator))
                .contains("`phone device boot pixel_7-api36`")
        );
    }

    fn peer(ip: &str, online: bool) -> tailscale::Peer {
        tailscale::Peer {
            hostname: "peer-a".into(),
            ip: ip.into(),
            node_id: "nXXXX".into(),
            os: "android".into(),
            online,
            last_seen: None,
        }
    }

    #[test]
    fn an_offline_peer_is_not_worth_sweeping() {
        let peers = [peer("100.64.0.20", false)];

        assert!(!routable(&peers, "100.64.0.20"));
    }

    #[test]
    fn an_online_peer_is() {
        let peers = [peer("100.64.0.20", true)];

        assert!(routable(&peers, "100.64.0.20"));
    }

    #[test]
    fn an_address_the_tailnet_no_longer_lists_is_not() {
        let peers = [peer("100.64.0.10", true)];

        assert!(!routable(&peers, "100.64.0.20"));
    }
}
