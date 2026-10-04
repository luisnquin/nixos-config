use std::path::PathBuf;
use std::time::Duration;

use crate::adb::{self, Attached, Server};
use crate::hosts::ADB_SERVER_PORT;
use crate::model::{is_transport_alias, Device, Platform, Reach, View, EMULATOR_SERIAL_PREFIX};
use crate::registry::Registry;
use crate::ssh;

pub async fn attached(reg: &Registry, want: &str) -> Option<View> {
    let device = only(reg, want)?;
    let held = device.transport.as_ref()?;
    let server = server_of(reg, held.host.as_deref()).await?;
    let (live, running) = listing(&server).await?;

    let row = live
        .into_iter()
        .find(|a| a.serial == held.serial && a.state == "device")?;

    if row.transport(held.host.as_deref()) != *held || !runs_as(device, &row.serial, &running) {
        return None;
    }

    let reach = Reach::Attached {
        wireless: row.is_wireless(),
        serial: row.serial,
    };

    Some(View::new(device.clone(), reach).on(server))
}

fn only<'a>(reg: &'a Registry, want: &str) -> Option<&'a Device> {
    let mut hits = reg.devices.iter().filter(|d| d.is(want));
    let device = hits.next()?;

    if hits.next().is_some() || !device.platform.is_adb() {
        return None;
    }

    let serial = &device.transport.as_ref()?.serial;

    (!is_transport_alias(want) || serial.eq_ignore_ascii_case(want)).then_some(device)
}

async fn server_of(reg: &Registry, host: Option<&str>) -> Option<Server> {
    let Some(host) = host else {
        return Some(Server::Local);
    };

    let state = reg.enabled_hosts().into_iter().find(|h| h.name == host)?;
    let port = state.tunnel_port.filter(|_| state.caps.adb)?;

    ssh::forward(host, port, ADB_SERVER_PORT).await.ok()?;

    Some(Server::Remote {
        host: host.to_string(),
        port,
    })
}

const INI: &str = "@@ini";

const LISTING: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
adb devices -l
for f in "${XDG_RUNTIME_DIR:-/nonexistent}"/avd/running/pid_*.ini "$HOME"/Library/Caches/TemporaryItems/avd/running/pid_*.ini; do
  [ -f "$f" ] && { echo "@@ini"; cat "$f"; }
done
exit 0"#;

async fn listing(server: &Server) -> Option<(Vec<Attached>, Vec<String>)> {
    let Server::Remote { host, .. } = server else {
        let live = adb::devices(server).await.ok()?;

        return Some((live, local_running()));
    };

    let out = tokio::time::timeout(
        Duration::from_secs(15),
        ssh::script(host, LISTING, &[]).output(),
    )
    .await
    .ok()?
    .ok()?;

    Some(split_listing(&String::from_utf8_lossy(&out.stdout)))
}

fn split_listing(text: &str) -> (Vec<Attached>, Vec<String>) {
    let text = text.replace('\r', "");
    let mut parts = text.split(INI);
    let live = adb::parse_devices(parts.next().unwrap_or_default());

    (live, parts.map(str::to_string).collect())
}

fn local_running() -> Vec<String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("avd/running"));
    let mac = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Caches/TemporaryItems/avd/running"));

    runtime
        .into_iter()
        .chain(mac)
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
        .collect()
}

fn runs_as(device: &Device, serial: &str, running: &[String]) -> bool {
    if device.platform != Platform::Emulator {
        return true;
    }

    let Some(port) = serial.strip_prefix(EMULATOR_SERIAL_PREFIX) else {
        return false;
    };

    let avds: Vec<&str> = running
        .iter()
        .filter(|ini| key(ini, "port.serial") == Some(port))
        .filter_map(|ini| key(ini, "avd.id"))
        .collect();

    !avds.is_empty() && avds.iter().all(|avd| *avd == device.label)
}

fn key<'a>(ini: &'a str, name: &str) -> Option<&'a str> {
    ini.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(k, _)| k.trim() == name)
        .map(|(_, v)| v.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transport;

    fn faraday() -> Device {
        let mut d = Device::new("58281FDCG001K5", "faraday", Platform::Android);

        d.add_alias("faraday:5555");
        d.transport = Some(Transport {
            serial: "faraday:5555".into(),
            id: "1".into(),
            ..Transport::default()
        });

        d
    }

    #[test]
    fn only_a_name_one_row_answers_to_skips_the_survey() {
        let mut reg = Registry::default();

        reg.devices.push(faraday());

        assert!(only(&reg, "faraday").is_some());
        assert!(only(&reg, "faraday:5555").is_some());

        let mut twin = faraday();

        twin.id = "other".into();
        reg.devices.push(twin);

        assert!(only(&reg, "faraday").is_none());
    }

    #[test]
    fn a_stale_transport_name_is_left_to_the_survey() {
        let mut reg = Registry::default();
        let mut d = faraday();

        d.add_alias("emulator-5554");
        reg.devices.push(d);

        assert!(only(&reg, "emulator-5554").is_none());

        let mut bare = faraday();

        bare.transport = None;
        reg.devices = vec![bare];

        assert!(only(&reg, "faraday").is_none());
    }

    #[test]
    fn a_host_listing_carries_its_running_files() {
        let text = "List of devices attached\r\n\
            emulator-5554 device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 device:emu64a transport_id:7\n\
            @@ini\navd.id=pixel_7-api36\nport.serial=5554\n\
            @@ini\navd.id=pixel_7-api36-b\nport.serial=5556\n";

        let (live, running) = split_listing(text);

        assert_eq!(live.len(), 1);
        assert_eq!(live[0].transport_id, "7");
        assert_eq!(running.len(), 2);

        let pixel = Device::new("pixel_7-api36", "pixel_7-api36", Platform::Emulator);
        let other = Device::new("pixel_7-api36-c", "pixel_7-api36-c", Platform::Emulator);

        assert!(runs_as(&pixel, "emulator-5554", &running));
        assert!(!runs_as(&other, "emulator-5554", &running));
        assert!(!runs_as(&pixel, "emulator-5558", &running));
    }
}
