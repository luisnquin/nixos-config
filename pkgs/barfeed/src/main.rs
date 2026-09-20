use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode};
use std::thread;
use std::time::Duration;

const TAILSCALE_SOCKET: &str = "/run/tailscale/tailscaled.sock";
const WHO: &str = "@who@";
const SSHD_CONFIG: &str = "/etc/ssh/sshd_config";
const NET_TCP: &str = "/proc/net/tcp";
const NET_TCP6: &str = "/proc/net/tcp6";
const TCP_ESTABLISHED: &str = "01";
const DEFAULT_SSH_PORT: u16 = 22;

fn main() -> ExitCode {
    let mut args = env::args();
    let executable = args.next().unwrap_or_else(|| "barfeed".to_owned());
    let invoked_as = Path::new(&executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("barfeed");

    let command = match invoked_as {
        "waybar-battery" => "battery".to_owned(),
        "waybar-ssh-solo" => "ssh-solo".to_owned(),
        "waybar-ssh-in" => "ssh-in".to_owned(),
        "waybar-ssh-out" => "ssh-out".to_owned(),
        "waybar-tailscale" => "tailscale".to_owned(),
        _ => match args.next() {
            Some(command) => command,
            None => {
                eprintln!("usage: barfeed <battery|ssh-solo|ssh-in|ssh-out|tailscale>");
                return ExitCode::FAILURE;
            }
        },
    };

    match command.as_str() {
        "battery" => battery(args),
        "ssh-solo" => {
            ssh_solo();
            ExitCode::SUCCESS
        }
        "ssh-in" => {
            ssh_in();
            ExitCode::SUCCESS
        }
        "ssh-out" => {
            ssh_out();
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

// Three modules cover two layouts. A single direction is a sibling of the
// stacked column rather than its lone survivor: GTK packs an only child at the
// top of a vertical box, so it would never sit on the bar's centre line.
// Whichever layout is not in use prints nothing and hide-empty-text drops it.
fn ssh_solo() {
    let scan = Scan::take();
    let inbound = scan.inbound();
    let outbound = scan.outbound();
    if !inbound.is_empty() && !outbound.is_empty() {
        print_json(json!({"text": ""}));
        return;
    }

    if !outbound.is_empty() {
        print_json(json!({
            "text": outbound_text(outbound.len()),
            "tooltip": tooltip("outbound", &outbound),
        }));
        return;
    }

    let count = inbound.len();
    print_json(json!({
        "text": inbound_text(count),
        "tooltip": tooltip("inbound", &inbound.sessions()),
    }));
}

fn ssh_in() {
    let scan = Scan::take();
    let inbound = scan.inbound();
    if inbound.is_empty() || scan.outbound().is_empty() {
        print_json(json!({"text": ""}));
        return;
    }

    let count = inbound.len();
    print_json(json!({
        "text": inbound_text(count),
        "tooltip": tooltip("inbound", &inbound.sessions()),
    }));
}

fn ssh_out() {
    let scan = Scan::take();
    let outbound = scan.outbound();
    if outbound.is_empty() || scan.inbound().is_empty() {
        print_json(json!({"text": ""}));
        return;
    }

    print_json(json!({
        "text": outbound_text(outbound.len()),
        "tooltip": tooltip("outbound", &outbound),
    }));
}

fn inbound_text(inbound: usize) -> String {
    let color = if inbound > 0 { "#b5e8e0" } else { "#6c7086" };
    format!("<span color=\"{color}\">\u{e9fd} {inbound}</span>")
}

fn outbound_text(outbound: usize) -> String {
    format!("<span color=\"#d8b4fe\">\u{e9fc} {outbound}</span>")
}

struct Session {
    kind: &'static str,
    peer: String,
    label: Option<String>,
}

fn tooltip(direction: &str, sessions: &[Session]) -> String {
    let mut lines = vec![format!("{} {direction}", sessions.len())];
    lines.extend(sessions.iter().map(|session| match &session.label {
        Some(label) => format!("{} · {} · {}", session.kind, label, session.peer),
        None => format!("{} · {}", session.kind, session.peer),
    }));
    escape_markup(&lines.join("\n"))
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

struct Scan {
    processes: Vec<Process>,
    connections: Vec<Connection>,
}

impl Scan {
    fn take() -> Self {
        Self {
            processes: session_processes(),
            connections: established_connections(),
        }
    }

    // Live sockets, not utmp: that table writes a row per tmux pane, and a dropped
    // client leaves a sshd-session process with no socket. Neither one is access.
    fn inbound(&self) -> Inbound<'_> {
        let ports = sshd_ports(&fs::read_to_string(SSHD_CONFIG).unwrap_or_default());

        Inbound {
            servers: self
                .processes
                .iter()
                .filter(|process| process.comm == "mosh-server")
                .collect(),
            sockets: self
                .connections
                .iter()
                .filter(|connection| ports.contains(&connection.local_port))
                .collect(),
        }
    }
}

// Counting an inbound session needs neither a fork nor a label, so `who` stays
// unspawned until a caller actually renders the tooltip.
struct Inbound<'a> {
    servers: Vec<&'a Process>,
    sockets: Vec<&'a Connection>,
}

impl Inbound<'_> {
    fn len(&self) -> usize {
        self.servers.len() + self.sockets.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn sessions(self) -> Vec<Session> {
        if self.is_empty() {
            return Vec::new();
        }

        let mut logins: Vec<Option<Login>> =
            who_entries(&who_output()).into_iter().map(Some).collect();

        let mosh: Vec<Session> = self
            .servers
            .into_iter()
            .map(|process| {
                let login = take_login(&mut logins, |login| {
                    mosh_pid(&login.host) == Some(process.pid)
                });
                Session {
                    kind: "mosh",
                    peer: login.as_ref().map_or_else(
                        || "unknown".to_owned(),
                        |login| host_address(&login.host).to_owned(),
                    ),
                    label: login.map(|login| format!("{}@{}", login.user, login.terminal)),
                }
            })
            .collect();

        let mut sessions: Vec<Session> = self
            .sockets
            .into_iter()
            .map(|connection| Session {
                kind: "ssh",
                label: claim_login(&mut logins, |login| {
                    ssh_login_matches(login, &connection.peer)
                }),
                peer: connection.peer.clone(),
            })
            .collect();

        sessions.extend(mosh);
        sessions
    }
}

impl Scan {
    fn outbound(&self) -> Vec<Session> {
        let Ok(uid) = fs::metadata("/proc/self").map(|metadata| metadata.uid()) else {
            return Vec::new();
        };
        let sockets = socket_index(&self.connections);

        self.processes
            .iter()
            .filter(|process| process.uid == uid)
            .filter_map(|process| match process.comm.as_str() {
                "ssh" => {
                    let arguments = arguments(&process.path);
                    Some(Session {
                        kind: if arguments.iter().any(|argument| argument == "-N") {
                            "tunnel"
                        } else {
                            "ssh"
                        },
                        peer: socket_peers(&process.path, &sockets)
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "multiplexed".to_owned()),
                        label: ssh_target(&arguments),
                    })
                }
                "mosh-client" => Some(Session {
                    kind: "mosh",
                    peer: mosh_client_peer(&arguments(&process.path))
                        .unwrap_or_else(|| "unknown".to_owned()),
                    label: None,
                }),
                _ => None,
            })
            .collect()
    }
}

fn who_output() -> String {
    Command::new(WHO)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

struct Login {
    user: String,
    terminal: String,
    host: String,
}

fn who_entries(output: &str) -> Vec<Login> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let user = fields.next()?.to_owned();
            let terminal = fields.next()?.to_owned();
            let open = line.find('(')?;
            let close = line.rfind(')')?;
            (open < close).then(|| Login {
                user,
                terminal,
                host: line[open + 1..close].to_owned(),
            })
        })
        .collect()
}

fn claim_login(logins: &mut [Option<Login>], matches: impl Fn(&Login) -> bool) -> Option<String> {
    take_login(logins, matches).map(|login| format!("{}@{}", login.user, login.terminal))
}

fn take_login(logins: &mut [Option<Login>], matches: impl Fn(&Login) -> bool) -> Option<Login> {
    let position = logins
        .iter()
        .position(|login| login.as_ref().is_some_and(&matches))?;
    logins[position].take()
}

fn ssh_login_matches(login: &Login, peer: &str) -> bool {
    mosh_pid(&login.host).is_none() && host_address(&login.host) == peer
}

fn host_address(host: &str) -> &str {
    host.split_whitespace().next().unwrap_or(host)
}

fn mosh_pid(host: &str) -> Option<u32> {
    let open = host.find('[')?;
    let close = host[open..].find(']')? + open;
    host[open + 1..close].parse().ok()
}

fn sshd_ports(config: &str) -> Vec<u16> {
    let ports: Vec<u16> = config
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("Port")
                .or_else(|| line.strip_prefix("port"))?;
            rest.split_whitespace().next()?.parse().ok()
        })
        .collect();

    if ports.is_empty() {
        vec![DEFAULT_SSH_PORT]
    } else {
        ports
    }
}

struct Connection {
    local_port: u16,
    peer: String,
    peer_port: u16,
    inode: u64,
}

fn established_connections() -> Vec<Connection> {
    let mut connections =
        parse_connections(&fs::read_to_string(NET_TCP).unwrap_or_default(), false);
    connections.extend(parse_connections(
        &fs::read_to_string(NET_TCP6).unwrap_or_default(),
        true,
    ));
    connections
}

fn parse_connections(table: &str, wide: bool) -> Vec<Connection> {
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if *fields.get(3)? != TCP_ESTABLISHED {
                return None;
            }
            let (_, local_port) = endpoint(fields.get(1)?, wide)?;
            let (peer, peer_port) = endpoint(fields.get(2)?, wide)?;
            Some(Connection {
                local_port,
                peer,
                peer_port,
                inode: fields.get(9)?.parse().ok()?,
            })
        })
        .collect()
}

fn endpoint(field: &str, wide: bool) -> Option<(String, u16)> {
    let (address, port) = field.split_once(':')?;
    Some((
        decode_address(address, wide)?,
        u16::from_str_radix(port, 16).ok()?,
    ))
}

fn decode_address(hex: &str, wide: bool) -> Option<String> {
    if !wide {
        let word = u32::from_str_radix(hex, 16).ok()?;
        return Some(Ipv4Addr::from(word.to_le_bytes()).to_string());
    }

    if hex.len() != 32 {
        return None;
    }
    let mut octets = [0u8; 16];
    for (index, group) in hex.as_bytes().chunks(8).enumerate() {
        let word = u32::from_str_radix(std::str::from_utf8(group).ok()?, 16).ok()?;
        octets[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }

    let address = Ipv6Addr::from(octets);
    Some(
        address
            .to_ipv4_mapped()
            .map_or_else(|| address.to_string(), |mapped| mapped.to_string()),
    )
}

struct Process {
    pid: u32,
    path: PathBuf,
    comm: String,
    uid: u32,
}

const SESSION_COMMS: [&str; 3] = ["mosh-server", "ssh", "mosh-client"];

fn session_processes() -> Vec<Process> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let pid = name.to_str()?.parse().ok()?;
            let path = entry.path();
            let comm = read_comm(&path.join("comm"))?;
            if !SESSION_COMMS.contains(&comm.as_str()) {
                return None;
            }
            Some(Process {
                pid,
                uid: fs::metadata(&path).ok()?.uid(),
                comm,
                path,
            })
        })
        .collect()
}

fn arguments(path: &Path) -> Vec<String> {
    fs::read(path.join("cmdline"))
        .unwrap_or_default()
        .split(|byte| *byte == 0)
        .filter(|argument| !argument.is_empty())
        .map(|argument| String::from_utf8_lossy(argument).into_owned())
        .collect()
}

fn socket_index(connections: &[Connection]) -> HashMap<u64, &Connection> {
    connections
        .iter()
        .map(|connection| (connection.inode, connection))
        .collect()
}

fn socket_peers(path: &Path, sockets: &HashMap<u64, &Connection>) -> Vec<String> {
    let Ok(descriptors) = fs::read_dir(path.join("fd")) else {
        return Vec::new();
    };

    descriptors
        .flatten()
        .filter_map(|descriptor| {
            let target = fs::read_link(descriptor.path()).ok()?;
            let inode = target
                .to_str()?
                .strip_prefix("socket:[")?
                .strip_suffix(']')?
                .parse::<u64>()
                .ok()?;
            let connection = sockets.get(&inode)?;
            Some(format!("{}:{}", connection.peer, connection.peer_port))
        })
        .collect()
}

fn ssh_target(arguments: &[String]) -> Option<String> {
    const VALUE_FLAGS: [&str; 17] = [
        "-o", "-L", "-R", "-D", "-p", "-i", "-l", "-F", "-W", "-J", "-b", "-c", "-m", "-Q", "-S",
        "-w", "-e",
    ];

    let mut skip = false;
    for argument in arguments.iter().skip(1) {
        if skip {
            skip = false;
        } else if VALUE_FLAGS.contains(&argument.as_str()) {
            skip = true;
        } else if !argument.starts_with('-') {
            return Some(argument.clone());
        }
    }
    None
}

fn mosh_client_peer(arguments: &[String]) -> Option<String> {
    let port = arguments.last()?;
    let address = arguments.get(arguments.len().checked_sub(2)?)?;
    port.parse::<u16>().ok()?;
    Some(format!("{address}:{port}"))
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

fn read_comm(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut buffer = [0u8; 32];
    let read = file.read(&mut buffer).ok()?;
    Some(std::str::from_utf8(&buffer[..read]).ok()?.trim().to_owned())
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
    use super::{
        battery_class, decode_address, host_address, mosh_client_peer, mosh_pid, parse_connections,
        parse_tailscale_response, ssh_login_matches, ssh_target, sshd_ports, who_entries,
        DEFAULT_SSH_PORT,
    };

    const TCP_TABLE: &str = concat!(
        "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n",
        "   0: 0A004064:0165 14004064:C3C0 01 00000000:00000000 00:00000000 00000000     0        0 9036000 1 0000 20 0 0 10 -1\n",
        "   1: 0A004064:C8F8 1E004064:0016 01 00000000:00000000 00:00000000 00000000  1000        0 9036506 1 0000 20 0 0 10 -1\n",
        "   2: 00000000:0165 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 15470 1 0000 10 0 0 10 -1\n",
    );

    #[test]
    fn reads_established_connections_only() {
        let connections = parse_connections(TCP_TABLE, false);
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].local_port, 357);
        assert_eq!(connections[0].peer, "100.64.0.20");
        assert_eq!(connections[1].peer_port, 22);
        assert_eq!(connections[1].inode, 9036506);
    }

    #[test]
    fn decodes_byte_swapped_addresses() {
        assert_eq!(decode_address("0A004064", false).unwrap(), "100.64.0.10");
        assert_eq!(
            decode_address("5C117AFD0000E0A10000000002000100", true).unwrap(),
            "fd7a:115c:a1e0::1:2"
        );
        assert_eq!(
            decode_address("0000000000000000FFFF00001E004064", true).unwrap(),
            "100.64.0.30"
        );
    }

    #[test]
    fn reads_the_ports_sshd_listens_on() {
        assert_eq!(
            sshd_ports("Port 357\nPasswordAuthentication no\n"),
            vec![357]
        );
        assert_eq!(sshd_ports("  Port 22\nPort 2222\n"), vec![22, 2222]);
        assert_eq!(sshd_ports("PortForwarding yes\n"), vec![DEFAULT_SSH_PORT]);
    }

    #[test]
    fn separates_logins_from_their_source() {
        let output = concat!(
            "dev pts/3 2026-09-20 15:41 (100.64.0.20 via mosh [1672032])\n",
            "dev pts/6 2026-09-20 15:38 (tmux(1449615).%7)\n",
            "dev tty1  2026-09-20 09:00\n",
        );
        let logins = who_entries(output);
        assert_eq!(logins.len(), 2);
        assert_eq!(host_address(&logins[0].host), "100.64.0.20");
        assert_eq!(mosh_pid(&logins[0].host), Some(1672032));
        assert_eq!(mosh_pid(&logins[1].host), None);
    }

    #[test]
    fn a_mosh_login_is_not_an_ssh_session() {
        let output = concat!(
            "dev pts/3 2026-09-20 15:41 (100.64.0.20 via mosh [1672032])\n",
            "dev pts/5 2026-09-20 15:44 (100.64.0.20)\n",
        );
        let logins = who_entries(output);
        assert!(!ssh_login_matches(&logins[0], "100.64.0.20"));
        assert!(ssh_login_matches(&logins[1], "100.64.0.20"));
        assert!(!ssh_login_matches(&logins[1], "100.64.0.30"));
    }

    #[test]
    fn keeps_a_port_with_a_trailing_comment() {
        assert_eq!(sshd_ports("Port 357 # not 22\n"), vec![357]);
    }

    #[test]
    fn finds_the_target_behind_the_flags() {
        let arguments: Vec<String> = [
            "ssh",
            "-o",
            "ControlMaster=auto",
            "mac",
            "-f",
            "-N",
            "-L",
            "127.0.0.1:34889:127.0.0.1:5037",
        ]
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect();
        assert_eq!(ssh_target(&arguments).as_deref(), Some("mac"));
    }

    #[test]
    fn reads_the_mosh_client_endpoint() {
        let arguments: Vec<String> = ["mosh-client", "-#", "key", "100.64.0.30", "60001"]
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect();
        assert_eq!(
            mosh_client_peer(&arguments).as_deref(),
            Some("100.64.0.30:60001")
        );
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
