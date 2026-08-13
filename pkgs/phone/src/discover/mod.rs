pub mod avahi;
pub mod sweep;
pub mod tailscale;

use std::collections::HashSet;

use crate::adb::{self, Server};
use crate::hosts::{self, Caps};
use crate::model::{
    discovered_id, Device, Endpoint, Pin, Platform, Reach, View, PLACEHOLDER_PREFIX,
};
use crate::registry::Registry;

/// The name discovery files tailnet peers under. Sources are named so two of
/// them can claim the same machine without colliding.
const TAILSCALE: &str = "tailscale";

/// Every adb server worth asking, this machine's first. adb has no federation
/// of its own — a server never talks to another server — so the fleet is
/// assembled client-side, one forward per enabled host.
///
/// Bringing tunnels up is part of surveying rather than a separate step: they
/// are daemonised and outlive the process, so all this usually does is confirm
/// the port an earlier run already opened.
async fn fleet(reg: &mut Registry) -> Vec<Server> {
    let states: Vec<_> = reg
        .hosts
        .iter()
        .filter(|h| h.enabled && h.caps.adb)
        .cloned()
        .collect();

    let mut tasks = tokio::task::JoinSet::new();

    for (i, mut state) in states.into_iter().enumerate() {
        tasks.spawn(async move {
            let port = hosts::adb_tunnel(&mut state).await.ok();

            (i, state.name, port)
        });
    }

    let mut up: Vec<(usize, String, u16)> = Vec::new();

    while let Some(Ok((i, name, port))) = tasks.join_next().await {
        if let Some(port) = port {
            up.push((i, name, port));
        }
    }

    up.sort_by_key(|(i, _, _)| *i);

    let mut out = vec![Server::Local];

    for (_, host, port) in up {
        reg.host_mut(&host).tunnel_port = Some(port);
        out.push(Server::Remote { host, port });
    }

    out
}

/// `adb devices` against every server at once, keeping each answer next to the
/// server that gave it: a serial is only unique within one server, and two macs
/// both running `emulator-5554` is the normal case, not an edge one.
async fn attached(fleet: &[Server]) -> Vec<(Server, Vec<adb::Attached>)> {
    let mut tasks = tokio::task::JoinSet::new();

    for (i, server) in fleet.iter().cloned().enumerate() {
        tasks.spawn(async move {
            let found = adb::devices(&server).await.unwrap_or_default();

            (i, server, found)
        });
    }

    let mut out: Vec<(usize, Server, Vec<adb::Attached>)> = Vec::new();

    while let Some(Ok(row)) = tasks.join_next().await {
        out.push(row);
    }

    out.sort_by_key(|(i, _, _)| *i);

    out.into_iter().map(|(_, s, d)| (s, d)).collect()
}

/// Devices that are only ever driven by running commands on their host: iPhones
/// behind a tunneld, and simulators, which have no socket to forward at all.
async fn hosted(enabled: &[(String, Caps)]) -> Vec<Device> {
    let mut tasks = tokio::task::JoinSet::new();

    for (host, caps) in enabled.iter().cloned() {
        if caps.tunneld {
            let host = host.clone();
            tasks.spawn(async move { crate::ios::devices(&host).await });
        }

        if caps.simctl {
            tasks.spawn(async move { crate::simctl::devices(&host).await });
        }
    }

    let mut out = Vec::new();

    while let Some(Ok(found)) = tasks.join_next().await {
        out.extend(found);
    }

    out.sort_by(|a, b| a.label.cmp(&b.label));

    out
}

/// Merge everything that can name a device — every adb server in the fleet,
/// every enabled host, the tailnet and the registry — into one list keyed by
/// stable device id.
///
/// Writes back what it learns: an attached device is the only moment where a
/// transport address and a hardware id are observable together, and that pairing
/// is exactly what later reconnects depend on.
pub async fn survey(reg: &mut Registry) -> Vec<View> {
    let fleet = fleet(reg).await;

    let enabled: Vec<(String, Caps)> = reg
        .enabled_hosts()
        .iter()
        .map(|h| (h.name.clone(), h.caps))
        .collect();

    let (fleet_devices, hosted_devices, peers) =
        tokio::join!(attached(&fleet), hosted(&enabled), tailscale::peers());

    let peers = peers.unwrap_or_default();

    let mut views: Vec<View> = Vec::new();
    let mut claimed: HashSet<String> = HashSet::new();

    for (server, found) in &fleet_devices {
        for dev in found {
            let Some((device, reach)) = resolve_attached(reg, server, dev).await else {
                continue;
            };

            claimed.insert(device.id.clone());
            views.push(View::new(device, reach).on(server.clone()));
        }
    }

    for peer in peers.iter().filter(|p| p.is_android()) {
        let discovered = discovered_id(TAILSCALE, &peer.node_id);

        let known = reg
            .by_discovered_id(&discovered)
            .or_else(|| reg.by_alias(&peer.hostname))
            .cloned();

        let mut device = known.unwrap_or_else(|| {
            // nothing has ever connected to it, so the peer key is the only
            // identity available until a first connect resolves the real one.
            let mut d = Device::new(
                format!("{PLACEHOLDER_PREFIX}{discovered}"),
                peer.hostname.clone(),
                Platform::Android,
            );

            d.endpoints.push(Endpoint::new(peer.ip.clone(), 5555));
            d
        });

        // the name a source advertises is the one that gets typed, and the
        // model already has its own column to sit in.
        device.label = peer.hostname.clone();
        device.discovered_id = Some(discovered);
        device.merge_endpoint(Endpoint::new(peer.ip.clone(), 5555));
        device.add_alias(peer.hostname.clone());

        let stored = reg.upsert(device).clone();

        if claimed.contains(&stored.id) {
            // already listed as attached; this source only refreshed its
            // address, it must not appear twice.
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

    // there is no adb transport to remember, but the row still has to outlive
    // the tunnel: it drops often enough that a device that is only listed while
    // reachable cannot be selected, queued for, or made the default.
    //
    // `last_connected` stays unset on purpose — a bare `phone connect` reaches
    // for the most recent device, and these have nothing to connect to.
    for device in hosted_devices {
        claimed.insert(device.id.clone());

        let stored = reg.upsert(device).clone();

        views.push(View::new(stored, Reach::Online));
    }

    for device in &reg.devices {
        if claimed.contains(&device.id) {
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

/// The id a serial is filed under. Hardware ids are globally unique, but an adb
/// serial is only unique within its own server, so anything keyed on one has to
/// carry the host it came from or a second mac's `emulator-5554` overwrites the
/// first one's row.
pub fn scoped(server: &Server, serial: &str) -> String {
    match server.host() {
        Some(host) => format!("{host}/{serial}"),
        None => serial.to_string(),
    }
}

async fn resolve_attached(
    reg: &mut Registry,
    server: &Server,
    dev: &adb::Attached,
) -> Option<(Device, Reach)> {
    let platform = dev.platform();
    let key = scoped(server, &dev.serial);

    if dev.state != "device" {
        // an unauthorized transport answers no shell command, so its identity
        // cannot be read; fall back to the serial and let the user see why.
        let mut device = reg
            .by_alias(&key)
            .cloned()
            .unwrap_or_else(|| Device::new(key, dev.serial.clone(), platform));

        device.host = server.host().map(str::to_string);

        return Some((
            device,
            Reach::Unauthorized {
                serial: dev.serial.clone(),
            },
        ));
    }

    if platform == Platform::Emulator {
        let name = adb::avd_name(server, &dev.serial).await;

        let label = if name.is_empty() { key.clone() } else { name };

        let mut device = Device::new(key, label, Platform::Emulator);

        device.model = dev.model.clone();
        device.host = server.host().map(str::to_string);
        device.add_alias(dev.serial.clone());

        let stored = reg.upsert(device).clone();

        return Some((
            stored,
            Reach::Attached {
                serial: dev.serial.clone(),
                wireless: false,
            },
        ));
    }

    let ident = adb::identity(server, &dev.serial).await;
    let id = ident.best_id().unwrap_or(key);

    let mut device = reg
        .get(&id)
        .cloned()
        .unwrap_or_else(|| Device::new(id.clone(), String::new(), platform));

    // an attach over an address some source advertised is the only moment its
    // key and the hardware id are provably the same handset; fold the
    // placeholder away before it survives as a second row.
    if let Some((host, _)) = split_addr(&dev.serial) {
        if let Some(stale) = reg.placeholder_at(&host, &id).cloned() {
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

    if device.label.is_empty() {
        device.label = if device.model.is_empty() {
            id.clone()
        } else {
            device.model.clone()
        };
    }

    device.add_alias(scoped(server, &dev.serial));

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

    // a live transport is a connection; leaving this at "never" while the
    // device is plainly attached is the one reading that cannot be right.
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
        let rose = Server::Remote {
            host: "rose".into(),
            port: 5038,
        };

        assert_eq!(scoped(&Server::Local, "emulator-5554"), "emulator-5554");
        assert_eq!(scoped(&rose, "emulator-5554"), "rose/emulator-5554");
    }
}
