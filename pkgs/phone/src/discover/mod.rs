pub mod avahi;
pub mod sweep;
pub mod tailscale;

use std::collections::HashSet;

use crate::adb::{self, Server};
use crate::hosts::{self, HostState};
use crate::model::{
    discovered_id, Device, Endpoint, Pin, Platform, Reach, View, PLACEHOLDER_PREFIX,
};
use crate::registry::Registry;
use crate::ssh::Where;

/// The name discovery files tailnet peers under. Sources are named so two of
/// them can claim the same machine without colliding.
const TAILSCALE: &str = "tailscale";

const LOCAL: &str = "local";

/// Key prefix for an AVD read off a host's SDK. It is not a hardware id — the
/// row is rebuilt from the host on every survey and never stored, because an
/// AVD that is deleted should stop being offered rather than linger as a name
/// nothing can boot.
const AVD_PREFIX: &str = "avd:";

struct Attach {
    dev: adb::Attached,
    ident: adb::Identity,
}

#[derive(Default)]
struct Found {
    at: Option<String>,
    inventoried: bool,
    server: Option<(Server, Vec<Attach>)>,
    hosted: Vec<(Device, bool)>,
    avds: Vec<Device>,
    peers: Vec<tailscale::Peer>,
}

#[derive(Default)]
struct Findings {
    fleet: Vec<(usize, Server, Vec<Attach>)>,
    hosted: Vec<(Device, bool)>,
    avds: Vec<Device>,
    peers: Vec<tailscale::Peer>,
    inventoried: HashSet<Option<String>>,
}

impl Findings {
    fn absorb(&mut self, rank: usize, one: Found) {
        if one.inventoried {
            self.inventoried.insert(one.at.clone());
        }

        if let Some((server, rows)) = one.server {
            self.fleet.push((rank, server, rows));
            self.fleet.sort_by_key(|(rank, _, _)| *rank);
        }

        self.hosted.extend(one.hosted);
        self.peers.extend(one.peers);
        self.avds.extend(one.avds);

        self.hosted.sort_by(|a, b| a.0.label.cmp(&b.0.label));
        self.avds.sort_by(|a, b| a.label.cmp(&b.label));
    }
}

pub struct Snapshot {
    pub views: Vec<View>,
    pub pending: Vec<String>,
}

async fn probe_server(server: Server) -> (Server, Vec<Attach>) {
    let found = adb::devices(&server).await.unwrap_or_default();

    let mut tasks = tokio::task::JoinSet::new();

    for (i, dev) in found.into_iter().enumerate() {
        let server = server.clone();

        tasks.spawn(async move {
            let ident = if dev.state == "device" {
                adb::identity(&server, &dev.serial).await
            } else {
                adb::Identity::default()
            };

            (i, Attach { dev, ident })
        });
    }

    let mut rows: Vec<(usize, Attach)> = Vec::new();

    while let Some(Ok(row)) = tasks.join_next().await {
        rows.push(row);
    }

    rows.sort_by_key(|(i, _)| *i);

    (server, rows.into_iter().map(|(_, row)| row).collect())
}

fn avd_device(host: Option<&str>, name: String) -> Device {
    let id = match host {
        Some(host) => format!("{AVD_PREFIX}{host}/{name}"),
        None => format!("{AVD_PREFIX}{name}"),
    };

    let mut device = Device::new(id, name, Platform::Emulator);

    device.host = host.map(str::to_string);

    device
}

async fn scan_local() -> Found {
    let (server, sims, avds, peers) = tokio::join!(
        probe_server(Server::Local),
        crate::simctl::devices(&Where::Here),
        crate::avd::list(&Where::Here),
        tailscale::peers(),
    );

    Found {
        at: None,
        inventoried: sims.is_some(),
        server: Some(server),
        hosted: sims.unwrap_or_default(),
        avds: avds
            .into_iter()
            .map(|name| avd_device(None, name))
            .collect(),
        peers: peers.unwrap_or_default(),
    }
}

async fn scan_host(mut state: HostState) -> (HostState, Found) {
    let name = state.name.clone();
    let caps = state.caps;

    let opened = if caps.adb {
        hosts::adb_tunnel(&mut state)
            .await
            .ok()
            .map(|port| Server::Remote {
                host: name.clone(),
                port,
            })
    } else {
        None
    };

    let (server, iphones, sims, avds) = tokio::join!(
        async {
            match opened {
                Some(server) => Some(probe_server(server).await),
                None => None,
            }
        },
        async {
            // a tunneld that lists an iPhone is listing one that is plugged in
            if caps.tunneld {
                crate::ios::devices(&name)
                    .await
                    .into_iter()
                    .map(|d| (d, true))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        },
        async {
            if caps.simctl {
                crate::simctl::devices(&Where::On(name.clone())).await
            } else {
                None
            }
        },
        async {
            if caps.adb || caps.emulator {
                crate::avd::list(&Where::On(name.clone())).await
            } else {
                Vec::new()
            }
        },
    );

    let inventoried = sims.is_some();

    let mut hosted = iphones;

    hosted.extend(sims.unwrap_or_default());

    let found = Found {
        at: Some(name.clone()),
        inventoried,
        server,
        hosted,
        avds: avds
            .into_iter()
            .map(|avd| avd_device(Some(&name), avd))
            .collect(),
        peers: Vec::new(),
    };

    (state, found)
}

pub async fn survey(reg: &mut Registry) -> Vec<View> {
    scan(reg, |_| {}).await
}

pub async fn scan(reg: &mut Registry, mut on: impl FnMut(Snapshot)) -> Vec<View> {
    let states: Vec<HostState> = reg.enabled_hosts().into_iter().cloned().collect();

    let mut pending: Vec<String> = std::iter::once(LOCAL.to_string())
        .chain(states.iter().map(|h| h.name.clone()))
        .collect();

    let mut found = Findings::default();
    let mut views = merge(reg, &found, pending.is_empty());

    on(Snapshot {
        views: views.clone(),
        pending: pending.clone(),
    });

    let mut tasks: tokio::task::JoinSet<(usize, String, Option<HostState>, Found)> =
        tokio::task::JoinSet::new();

    tasks.spawn(async { (0, LOCAL.to_string(), None, scan_local().await) });

    for (i, state) in states.into_iter().enumerate() {
        tasks.spawn(async move {
            let name = state.name.clone();
            let (state, found) = scan_host(state).await;

            (i + 1, name, Some(state), found)
        });
    }

    while let Some(joined) = tasks.join_next().await {
        let Ok((rank, name, state, one)) = joined else {
            continue;
        };

        if let Some(state) = state {
            reg.host_mut(&name).tunnel_port = state.tunnel_port;
        }

        found.absorb(rank, one);
        pending.retain(|p| p != &name);

        views = merge(reg, &found, pending.is_empty());

        on(Snapshot {
            views: views.clone(),
            pending: pending.clone(),
        });
    }

    views
}

/// Merges everything that can name a device into one list keyed by stable id,
/// writing back what it learns: an attached device is the only moment a
/// transport address and a hardware id are observable together, which is what
/// later reconnects depend on.
fn merge(reg: &mut Registry, found: &Findings, settled: bool) -> Vec<View> {
    let mut views: Vec<View> = Vec::new();
    let mut claimed: HashSet<String> = HashSet::new();

    for (_, server, rows) in &found.fleet {
        for row in rows {
            let Some((device, reach)) = resolve_attached(reg, server, row) else {
                continue;
            };

            // a forwarded emulator answers this machine's adb server and its
            // host's alike; the fleet is ordered by how far away the server is
            if !claimed.insert(device.id.clone()) {
                continue;
            }

            views.push(View::new(device, reach).on(server.clone()));
        }
    }

    for peer in found.peers.iter().filter(|p| p.is_android()) {
        let discovered = discovered_id(TAILSCALE, &peer.node_id);

        let known = reg
            .by_discovered_id(&discovered)
            .or_else(|| reg.by_alias(&peer.hostname))
            .cloned();

        let mut device = known.unwrap_or_else(|| {
            // the peer key is the only identity there is until a first connect
            let mut d = Device::new(
                format!("{PLACEHOLDER_PREFIX}{discovered}"),
                peer.hostname.clone(),
                Platform::Android,
            );

            d.endpoints.push(Endpoint::new(peer.ip.clone(), 5555));
            d
        });

        // the advertised name is the one that gets typed; model has its own column
        device.label = peer.hostname.clone();
        device.discovered_id = Some(discovered);
        device.merge_endpoint(Endpoint::new(peer.ip.clone(), 5555));
        device.add_alias(peer.hostname.clone());

        let stored = reg.upsert(device).clone();

        if claimed.contains(&stored.id) {
            // already listed as attached; this source only refreshed its address
            continue;
        }

        claimed.insert(stored.id.clone());

        views.push(View::new(
            stored,
            if peer.online {
                Reach::Online
            } else {
                Reach::Offline {
                    last_seen: peer.last_seen,
                }
            },
        ));
    }

    // no adb transport to remember, but the row must outlive the tunnel: it
    // drops often enough that a device listed only while reachable cannot be
    // selected or made the default. `last_connected` stays unset, since a bare
    // `phone device connect` reaches for the most recent device.
    for (device, booted) in &found.hosted {
        let device = device.clone();

        claimed.insert(device.id.clone());

        // only a running simulator is worth remembering: the row exists to
        // outlive a dropped tunnel, and one that was never started has nothing
        // to outlive. Storing the rest would also keep offering a simulator
        // long after it was deleted from the host.
        if !booted {
            views.push(View::new(device, Reach::Off));
            continue;
        }

        let stored = reg.upsert(device).clone();

        views.push(View::new(stored, Reach::Online));
    }

    // an AVD whose emulator is already running was listed above under the
    // hardware id that emulator reported, and the two are the same device
    let mut off: Vec<String> = Vec::new();

    for device in &found.avds {
        let mut device = device.clone();

        if views
            .iter()
            .any(|v| v.device.host == device.host && v.device.is(&device.label))
        {
            continue;
        }

        // the SDK names an AVD but says nothing about what it emulates, and the
        // row it supersedes was written while the thing was running and adb
        // could be asked
        if let Some(seen) = reg
            .devices
            .iter()
            .find(|d| d.platform == Platform::Emulator && d.is(&device.label))
        {
            device.model = seen.model.clone();
        }

        off.push(device.label.clone());
        claimed.insert(device.id.clone());

        views.push(View::new(device, Reach::Off));
    }

    // an alias learned above can reveal that a remembered row is a device this
    // survey already listed under a stronger key. Folding here rather than on
    // load means the survey that learns the alias is the one that stops showing
    // two rows for the one device.
    if settled {
        reg.fold_aliased(&claimed);

        let gone: Vec<String> = reg
            .devices
            .iter()
            .filter(|d| stale_sim(&claimed, &found.inventoried, d))
            .map(|d| d.id.clone())
            .collect();

        for id in gone {
            reg.remove(&id);
        }
    }

    for device in &reg.devices {
        if claimed.contains(&device.id) {
            continue;
        }

        if stale_sim(&claimed, &found.inventoried, device) {
            continue;
        }

        // a remembered emulator that the SDK just listed as bootable is that
        // AVD, filed under the hardware id it had while it ran. `off` says
        // where it is and how to start it, which `known` cannot
        if device.platform == Platform::Emulator && off.iter().any(|name| device.is(name)) {
            continue;
        }

        views.push(View::new(device.clone(), Reach::Known));
    }

    views.sort_by(|a, b| {
        a.reach
            .rank()
            .cmp(&b.reach.rank())
            .then_with(|| {
                b.device
                    .last_connected
                    .unwrap_or(0)
                    .cmp(&a.device.last_connected.unwrap_or(0))
            })
            .then_with(|| a.device.label.cmp(&b.device.label))
    });

    views
}

fn stale_sim(
    claimed: &HashSet<String>,
    inventoried: &HashSet<Option<String>>,
    device: &Device,
) -> bool {
    device.platform == Platform::Simulator
        && !claimed.contains(&device.id)
        && inventoried.contains(&device.host)
}

/// The id a serial is filed under. Hardware ids are globally unique; an adb
/// serial is only unique within its server, so it carries the host or a second
/// mac's `emulator-5554` overwrites the first one's row.
pub fn scoped(server: &Server, serial: &str) -> String {
    match server.host() {
        Some(host) => format!("{host}/{serial}"),
        None => serial.to_string(),
    }
}

fn resolve_attached(reg: &mut Registry, server: &Server, row: &Attach) -> Option<(Device, Reach)> {
    let Attach { dev, ident } = row;
    let key = scoped(server, &dev.serial);

    if dev.state != "device" {
        // An emulator on its way out keeps an `offline` row for a few seconds
        // after `emu kill`, and reading that as a device that is up is what
        // leaves a `down` followed straight away by an `up` with nothing to
        // start: the row claims the AVD, so the listing that would have called
        // it bootable never gets a say, and the run goes looking for a
        // transport to an emulator that no longer exists. Dropping it here
        // hands the answer to that listing, which says `off`.
        if dev.platform() == Platform::Emulator && !dev.is_authorizing() {
            return None;
        }

        let mut device = reg
            .by_alias(&key)
            .cloned()
            .unwrap_or_else(|| Device::new(key, dev.serial.clone(), dev.platform()));

        device.host = server.host().map(str::to_string);

        return Some((
            device,
            Reach::Unauthorized {
                serial: dev.serial.clone(),
            },
        ));
    }

    // the serial shape only says how this transport was opened: an emulator
    // reached over tcp has no `emulator-` serial and is an emulator regardless
    let platform = if ident.is_emulator() {
        Platform::Emulator
    } else {
        dev.platform()
    };

    // an emulator keyed by its serial is a different device per adb server, and
    // the one forwarded from a mac answers on both
    let id = ident.best_id().unwrap_or_else(|| key.clone());

    // a row already carrying this id as an alias is the same device under a
    // weaker key; filing it again would leave both standing
    let mut device = reg
        .by_alias(&id)
        .cloned()
        .unwrap_or_else(|| Device::new(id.clone(), String::new(), platform));

    // an attach over an advertised address is the only moment the key and the
    // hardware id are provably one handset; fold the placeholder away
    if let Some((host, _)) = split_addr(&dev.serial) {
        if let Some(stale) = reg.placeholder_at(&host, &device.id).cloned() {
            device.absorb(&stale);
            reg.remove(&stale.id);
        }
    }

    device.platform = platform;
    device.host = server.host().map(str::to_string);

    if !ident.model.is_empty() {
        device.model = ident.model.clone();
    } else if !dev.model.is_empty() {
        device.model = dev.model.clone();
    }

    // `adb devices -l` reports an emulator's system image as its model, the same
    // for all of them; the AVD name is what was typed to start this one
    if platform == Platform::Emulator && !ident.avd.is_empty() {
        device.label = ident.avd.clone();
    }

    if device.label.is_empty() {
        device.label = if device.model.is_empty() {
            device.id.clone()
        } else {
            device.model.clone()
        };
    }

    device.add_alias(key);

    if !ident.android_id.is_empty() {
        device.add_alias(format!("android_id:{}", ident.android_id));
    }

    if let Some((host, port)) = split_addr(&dev.serial) {
        let pin = if port == 5555 {
            Pin::Session
        } else {
            Pin::None
        };
        device.record_endpoint(&host, port, pin);
    }

    // a live transport is a connection; "never" while attached cannot be right
    device.last_connected = Some(crate::model::now());

    let stored = reg.upsert(device).clone();

    Some((
        stored,
        Reach::Attached {
            serial: dev.serial.clone(),
            wireless: dev.is_wireless(),
        },
    ))
}

pub fn split_addr(serial: &str) -> Option<(String, u16)> {
    let (host, port) = serial.rsplit_once(':')?;
    let port = port.parse().ok()?;

    Some((host.to_string(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_serial_is_only_unique_within_its_own_server() {
        let mac = Server::Remote {
            host: "mac".into(),
            port: 5038,
        };

        assert_eq!(scoped(&Server::Local, "emulator-5554"), "emulator-5554");
        assert_eq!(scoped(&mac, "emulator-5554"), "mac/emulator-5554");
    }

    #[test]
    fn an_unanswered_survey_still_lists_what_is_remembered() {
        let mut reg = Registry::default();

        reg.upsert(Device::new("udid", "iPhone 17", Platform::Ios));
        reg.upsert(Device::new("serial", "pixel-9", Platform::Android));

        let views = merge(&mut reg, &Findings::default(), false);

        assert_eq!(views.len(), 2);
        assert!(views.iter().all(|v| v.reach == Reach::Known));
    }

    fn recreated_on_mac() -> (Registry, Findings) {
        let mut reg = Registry::default();

        for udid in ["mac/dead", "mac/live"] {
            let mut sim = Device::new(udid, "iPhone 17", Platform::Simulator);

            sim.host = Some("mac".into());

            reg.upsert(sim);
        }

        let mut live = Device::new("mac/live", "iPhone 17", Platform::Simulator);

        live.host = Some("mac".into());

        let found = Findings {
            hosted: vec![(live, false)],
            ..Findings::default()
        };

        (reg, found)
    }

    #[test]
    fn a_simulator_the_host_no_longer_lists_stops_being_offered() {
        let (mut reg, mut found) = recreated_on_mac();

        found.inventoried.insert(Some("mac".into()));

        let views = merge(&mut reg, &found, true);

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].device.id, "mac/live");
        assert_eq!(reg.devices.len(), 1);
    }

    #[test]
    fn a_host_that_never_answered_keeps_its_remembered_simulators() {
        let (mut reg, found) = recreated_on_mac();

        let views = merge(&mut reg, &found, true);

        assert_eq!(views.len(), 2);
        assert_eq!(reg.devices.len(), 2);
    }
}
