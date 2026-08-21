use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{self, Command, ExitCode};
use std::thread;
use std::time::Duration;

const TAILSCALE_SOCKET: &str = "/run/tailscale/tailscaled.sock";
const WHO: &str = "@who@";

fn main() -> ExitCode {
    let mut args = env::args();
    let executable = args.next().unwrap_or_else(|| "waytools".to_owned());
    let invoked_as = Path::new(&executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("waytools");

    let command = match invoked_as {
        "waybar-battery" => "battery".to_owned(),
        "waybar-ssh" => "ssh".to_owned(),
        "waybar-tailscale" => "tailscale".to_owned(),
        _ => match args.next() {
            Some(command) => command,
            None => {
                eprintln!("usage: waytools <battery|ssh|tailscale>");
                return ExitCode::FAILURE;
            }
        },
    };

    match command.as_str() {
        "battery" => battery(args),
        "ssh" => {
            ssh();
            ExitCode::SUCCESS
        }
        "tailscale" => tailscale(args),
        _ => {
            eprintln!("unknown waytool: {command}");
            ExitCode::FAILURE
        }
    }
}

fn battery(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut warning = 15;
    let mut critical = 5;

    while let Some(argument) = args.next() {
        let target = match argument.as_str() {
            "--warn" => &mut warning,
            "--critical" => &mut critical,
            _ => {
                eprintln!("unknown battery argument: {argument}");
                return ExitCode::FAILURE;
            }
        };

        let Some(value) = args.next().and_then(|value| value.parse::<u8>().ok()) else {
            eprintln!("{argument} requires an integer from 0 to 255");
            return ExitCode::FAILURE;
        };
        *target = value;
    }

    let Some((capacity, status, online)) = battery_state(Path::new("/sys/class/power_supply"))
    else {
        print_json(json!({"text": "?", "tooltip": "No battery", "class": "missing"}));
        return ExitCode::SUCCESS;
    };

    let class = battery_class(capacity, warning, critical);
    let mut icon = match capacity {
        0..=11 => "\u{f244}",
        12..=36 => "\u{f243}",
        37..=61 => "\u{f242}",
        62..=86 => "\u{f241}",
        _ => "\u{f240}",
    }
    .to_owned();

    if status == "Charging" || (status != "Full" && online) {
        icon.push_str(" \u{f0e7}");
    }

    print_json(json!({
        "text": format!(
            "<span size=\"7.5pt\">{capacity}%</span> <span size=\"10pt\">{icon}</span>"
        ),
        "tooltip": format!("{capacity}% · {status}"),
        "class": class,
    }));
    ExitCode::SUCCESS
}

fn battery_state(root: &Path) -> Option<(u8, String, bool)> {
    let entries = fs::read_dir(root).ok()?;
    let mut battery = None;
    let mut online = false;

    for entry in entries.flatten() {
        let path = entry.path();
        let kind = read_trimmed(path.join("type")).unwrap_or_default();
        match kind.as_str() {
            "Battery" if battery.is_none() => battery = Some(path),
            "Mains" | "USB" => {
                online |= read_trimmed(path.join("online")).as_deref() == Some("1");
            }
            _ => {}
        }
    }

    let battery = battery?;
    let capacity = read_trimmed(battery.join("capacity"))
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    let status = read_trimmed(battery.join("status")).unwrap_or_else(|| "Unknown".to_owned());
    Some((capacity, status, online))
}

fn battery_class(capacity: u8, warning: u8, critical: u8) -> &'static str {
    if capacity <= critical {
        "critical"
    } else if capacity <= warning {
        "warning"
    } else {
        "normal"
    }
}

fn ssh() {
    let inbound = inbound_sessions();
    let outbound = outbound_sessions();

    if inbound == 0 && outbound == 0 {
        print_json(json!({"text": "", "tooltip": ""}));
        return;
    }

    let mut text = "<span color=\"#cba6f7\" size=\"15pt\">󰣀</span>".to_owned();
    let mut tooltip = Vec::new();

    if inbound > 0 {
        text.push_str(&format!(
            " <span color=\"#b5e8e0\">\u{e9fd} {inbound}</span>"
        ));
        tooltip.push(format!("SSH inbound: {inbound}"));
    }
    if outbound > 0 {
        text.push_str(&format!(
            " <span color=\"#d8b4fe\">\u{e9fc} {outbound}</span>"
        ));
        tooltip.push(format!("SSH outbound: {outbound}"));
    }

    print_json(json!({"text": text, "tooltip": tooltip.join(" · ")}));
}

fn inbound_sessions() -> usize {
    Command::new(WHO)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| count_inbound(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or(0)
}

fn count_inbound(output: &str) -> usize {
    output
        .lines()
        .filter(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            fields
                .get(1)
                .is_some_and(|terminal| terminal.starts_with("pts/"))
                && fields.last().is_some_and(|host| host.starts_with('('))
        })
        .count()
}

fn outbound_sessions() -> usize {
    let Ok(uid) = fs::metadata("/proc/self").map(|metadata| metadata.uid()) else {
        return 0;
    };
    let Ok(processes) = fs::read_dir("/proc") else {
        return 0;
    };

    processes
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.bytes().all(|byte| byte.is_ascii_digit()))
        })
        .filter(|entry| {
            fs::metadata(entry.path()).is_ok_and(|metadata| metadata.uid() == uid)
                && read_trimmed(entry.path().join("comm")).as_deref() == Some("ssh")
        })
        .count()
}

fn tailscale(args: impl Iterator<Item = String>) -> ExitCode {
    let mut once = false;
    for argument in args {
        if argument == "--once" {
            once = true;
        } else {
            eprintln!("unknown tailscale argument: {argument}");
            return ExitCode::FAILURE;
        }
    }

    loop {
        match tailscale_status() {
            Ok((online, total)) => connected_tailscale(online, total),
            Err(error) => {
                eprintln!("Tailscale LocalAPI error: {error}");
                disconnected_tailscale();
            }
        }

        if once {
            return ExitCode::SUCCESS;
        }
        thread::sleep(Duration::from_secs(5));
    }
}

fn tailscale_status() -> Result<(usize, usize), String> {
    let mut socket = UnixStream::connect(TAILSCALE_SOCKET).map_err(|error| error.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    socket
        .write_all(b"GET /localapi/v0/status HTTP/1.0\r\nHost: local-tailscaled.sock\r\n\r\n")
        .map_err(|error| error.to_string())?;

    let mut response = Vec::new();
    socket
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    parse_tailscale_response(&response)
}

fn parse_tailscale_response(response: &[u8]) -> Result<(usize, usize), String> {
    let body_start = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .ok_or_else(|| "invalid HTTP response".to_owned())?;
    let headers = &response[..body_start];
    if !headers.starts_with(b"HTTP/1.0 200 ") && !headers.starts_with(b"HTTP/1.1 200 ") {
        return Err("LocalAPI returned a non-success response".to_owned());
    }

    let status: Value =
        serde_json::from_slice(&response[body_start..]).map_err(|error| error.to_string())?;
    let peers = status.get("Peer").and_then(Value::as_object);
    let total = peers.map_or(0, |peers| peers.len());
    let online = peers.map_or(0, |peers| {
        peers
            .values()
            .filter(|peer| peer.get("Online").and_then(Value::as_bool) == Some(true))
            .count()
    });
    Ok((online, total))
}

fn connected_tailscale(online: usize, total: usize) {
    print_json(json!({
        "text": format!("\u{e9ff} {online}/{total}"),
        "tooltip": format!("{online}/{total}"),
    }));
}

fn disconnected_tailscale() {
    print_json(json!({"text": "\u{e9ff} off", "class": "disconnected"}));
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn print_json(value: Value) {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    if let Err(error) = writeln!(output, "{value}").and_then(|()| output.flush()) {
        if error.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("failed to write Waybar status: {error}");
        }
        process::exit(u8::from(error.kind() != io::ErrorKind::BrokenPipe).into());
    }
}

#[cfg(test)]
mod tests {
    use super::{battery_class, count_inbound, parse_tailscale_response};

    #[test]
    fn counts_only_remote_pseudo_terminal_sessions() {
        let output = "luis pts/1 2026-08-21 10:00 (100.64.0.1)\nluis tty1 2026-08-21 09:00\n";
        assert_eq!(count_inbound(output), 1);
    }

    #[test]
    fn classifies_battery_thresholds() {
        assert_eq!(battery_class(5, 15, 5), "critical");
        assert_eq!(battery_class(15, 15, 5), "warning");
        assert_eq!(battery_class(16, 15, 5), "normal");
    }

    #[test]
    fn counts_online_tailscale_peers() {
        let response = b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"Peer\":{\"a\":{\"Online\":true},\"b\":{\"Online\":false}}}";
        assert_eq!(parse_tailscale_response(response).unwrap(), (1, 2));
    }
}
