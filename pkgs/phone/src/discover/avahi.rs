use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

pub const CONNECT_SERVICE: &str = "_adb-tls-connect._tcp";
pub const PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp";

#[derive(Clone, Debug)]
pub struct Service {
    pub name: String,
    pub host: String,
    pub port: u16,
}

impl Service {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Wireless debugging advertises itself as `adb-<hardware serial>-<suffix>`,
    /// which is the only place on the network where the transport address and
    /// the stable device id appear together.
    pub fn serial(&self) -> Option<&str> {
        let rest = self.name.strip_prefix("adb-")?;
        let (serial, _) = rest.rsplit_once('-')?;

        (!serial.is_empty()).then_some(serial)
    }
}

/// nixpkgs builds android-tools without the bundled mDNS responder, so
/// `adb mdns services` is unavailable and avahi (already running on this host)
/// stands in for it.
pub async fn browse(service: &str, wait: Duration) -> Vec<Service> {
    let out = tokio::time::timeout(
        wait + Duration::from_secs(2),
        Command::new("avahi-browse")
            .args(["-t", "-r", "-p", "-k", service])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output(),
    )
    .await;

    let Ok(Ok(out)) = out else {
        return Vec::new();
    };

    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(text: &str) -> Vec<Service> {
    let mut out = Vec::new();

    for line in text.lines() {
        // resolved records only; browse-only lines carry no address or port
        let fields: Vec<&str> = line.split(';').collect();

        if fields.first() != Some(&"=") || fields.len() < 9 {
            continue;
        }

        if fields[2] != "IPv4" {
            continue;
        }

        let Ok(port) = fields[8].parse::<u16>() else {
            continue;
        };

        out.push(Service {
            name: unescape(fields[3]),
            host: fields[7].to_string(),
            port,
        });
    }

    out
}

/// avahi escapes separators in the service name as `\<char>`.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolved_records_only() {
        let text = "+;wlan0;IPv4;adb-R58N70ABCDE-Kx9pQ;_adb-tls-connect._tcp;local\n\
                    =;wlan0;IPv4;adb-R58N70ABCDE-Kx9pQ;_adb-tls-connect._tcp;local;pixel.local;192.168.1.5;37419;\n\
                    =;wlan0;IPv6;adb-R58N70ABCDE-Kx9pQ;_adb-tls-connect._tcp;local;pixel.local;fe80::1;37419;\n";

        let found = parse(text);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].addr(), "192.168.1.5:37419");
        assert_eq!(found[0].serial(), Some("R58N70ABCDE"));
    }
}
