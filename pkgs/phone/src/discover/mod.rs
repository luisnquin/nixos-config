pub mod avahi;
pub mod sweep;
pub mod tailscale;

use std::collections::HashSet;

use crate::adb;
use crate::model::{
    Device, Endpoint, Pin, Platform, Reach, Tailnet, View, PLACEHOLDER_PREFIX,
};
use crate::registry::Registry;

/// Merge everything that can name a device — the adb server, the tailnet and
/// the registry — into one list keyed by stable device id.
///
/// Writes back what it learns: an attached device is the only moment where a
/// transport address and a hardware id are observable together, and that pairing
/// is exactly what later reconnects depend on.
pub async fn survey(reg: &mut Registry) -> Vec<View> {
    // the iPhone probe is an ssh round trip to the mac; overlap it with the
    // local adb call rather than paying for it in series.
    let (attached, peers, iphones) =
        tokio::join!(adb::devices(), tailscale::peers(), crate::ios::devices());

    let attached = attached.unwrap_or_default();
    let peers = peers.unwrap_or_default();

    let mut views: Vec<View> = Vec::new();
    let mut claimed: HashSet<String> = HashSet::new();

    for dev in &attached {
        let Some((device, reach)) = resolve_attached(reg, dev).await else {
            continue;
        };

        claimed.insert(device.id.clone());
        views.push(View { device, reach });
    }

    for peer in peers.iter().filter(|p| p.is_android()) {
        let known = reg
            .by_node_id(&peer.node_id)
            .or_else(|| reg.by_alias(&peer.hostname))
            .cloned();

        let mut device = known.unwrap_or_else(|| {
            // nothing has ever connected to it, so the tailnet node is the only
            // identity available until a first connect resolves the real one.
            let mut d = Device::new(
                format!("{PLACEHOLDER_PREFIX}{}", peer.node_id),
                peer.hostname.clone(),
                Platform::Android,
            );

            d.endpoints.push(Endpoint::new(peer.ip.clone(), 5555));
            d
        });

        device.tailnet = Some(Tailnet {
            hostname: peer.hostname.clone(),
            ip: peer.ip.clone(),
            node_id: peer.node_id.clone(),
        });

        reg.upsert(device.clone());

        if claimed.contains(&device.id) {
            // already listed as attached; the tailnet entry only refreshed its
            // address, it must not appear twice.
            continue;
        }

        claimed.insert(device.id.clone());

        views.push(View {
            device,
            reach: if peer.online {
                Reach::Online
            } else {
                Reach::Offline {
                    last_seen: peer.last_seen,
                }
            },
        });
    }

    // there is no adb transport to remember, but the row still has to outlive the
    // tunnel: it drops often enough that a device that is only listed while
    // reachable cannot be selected, queued for, or made the default.
    //
    // `last_connected` stays unset on purpose — a bare `phone connect` reaches
    // for the most recent device, and the iPhone has nothing to connect to.
    for device in iphones {
        claimed.insert(device.id.clone());

        let stored = reg.upsert(device).clone();

        views.push(View {
            device: stored,
            reach: Reach::Online,
        });
    }

    for device in &reg.devices {
        if claimed.contains(&device.id) {
            continue;
        }

        views.push(View {
            device: device.clone(),
            reach: Reach::Known,
        });
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

async fn resolve_attached(reg: &mut Registry, dev: &adb::Attached) -> Option<(Device, Reach)> {
    let platform = dev.platform();

    if dev.state != "device" {
        // an unauthorized transport answers no shell command, so its identity
        // cannot be read; fall back to the serial and let the user see why.
        let device = reg
            .by_alias(&dev.serial)
            .cloned()
            .unwrap_or_else(|| Device::new(dev.serial.clone(), dev.serial.clone(), platform));

        return Some((
            device,
            Reach::Unauthorized {
                serial: dev.serial.clone(),
            },
        ));
    }

    if platform == Platform::Emulator {
        let name = adb::avd_name(&dev.serial).await;
        let label = if name.is_empty() {
            dev.serial.clone()
        } else {
            name
        };

        let mut device = Device::new(dev.serial.clone(), label, Platform::Emulator);
        device.model = dev.model.clone();

        return Some((
            device,
            Reach::Attached {
                serial: dev.serial.clone(),
                wireless: false,
            },
        ));
    }

    let ident = adb::identity(&dev.serial).await;
    let id = ident.best_id().unwrap_or_else(|| dev.serial.clone());

    let mut device = reg
        .get(&id)
        .cloned()
        .unwrap_or_else(|| Device::new(id.clone(), String::new(), platform));

    // an attach over a tailnet address is the only moment the node id and the
    // hardware id are provably the same handset; fold the placeholder away
    // before it survives as a second row.
    if let Some((host, _)) = split_addr(&dev.serial) {
        if let Some(stale) = reg.placeholder_at(&host, &id).cloned() {
            device.absorb(&stale);
            reg.remove(&stale.id);
        }
    }

    device.platform = platform;

    if !ident.model.is_empty() {
        device.model = ident.model.clone();
    } else if !dev.model.is_empty() {
        device.model = dev.model.clone();
    }

    // labels are always derived, never user-set, so a tailnet hostname always
    // wins: it is the name that gets typed, and the model already has its own
    // column to sit in.
    let hostname = device
        .tailnet
        .as_ref()
        .map(|t| t.hostname.clone())
        .filter(|h| !h.is_empty());

    match hostname {
        Some(hostname) => device.label = hostname,
        None if device.label.is_empty() => {
            device.label = if device.model.is_empty() {
                id.clone()
            } else {
                device.model.clone()
            }
        }
        None => {}
    }

    device.add_alias(dev.serial.clone());

    if !ident.android_id.is_empty() {
        device.add_alias(format!("android_id:{}", ident.android_id));
    }

    if let Some((host, port)) = split_addr(&dev.serial) {
        let pin = if port == 5555 { Pin::Session } else { Pin::None };
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
