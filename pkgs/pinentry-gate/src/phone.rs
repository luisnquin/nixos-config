//! The terminals that asked to be told: a pty marked under the runtime dir
//! gets the request as a private OSC and answers on the request's fifo.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use base64::Engine;
use rustix::fs::OFlags;

pub const OSC: u32 = 7771;

/// What may hold a marked pty: the ssh session the terminal came in on, or
/// the Mosh server or Dazzle engine that terminal left running, which lets go
/// of its ssh session the moment it is up and outlives every one that follows.
fn holds_a_pty(comm: &str) -> bool {
    comm.starts_with("sshd") || comm == "mosh-server" || comm == "dazzle-engine"
}

fn alive(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|comm| holds_a_pty(comm.trim()))
}

fn pts_number(name: &str) -> Option<u32> {
    name.strip_prefix("pts-")?.parse().ok()
}

/// Every pty a terminal marked whose ssh session is still alive. A mark whose
/// session is gone is dropped here, so the directory prunes itself.
pub fn marked(runtime: &Path) -> Vec<PathBuf> {
    marked_by(runtime, alive)
}

pub fn marked_by(runtime: &Path, alive: impl Fn(i32) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(runtime.join("tty")) else {
        return Vec::new();
    };
    let uid = rustix::process::getuid().as_raw();
    let mut names: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    let mut ptys = Vec::new();
    for entry in names {
        let Some(number) = entry.file_name().and_then(|n| n.to_str()).and_then(pts_number) else {
            continue;
        };
        let pid: i32 = std::fs::read_to_string(&entry).ok().and_then(|text| text.trim().parse().ok()).unwrap_or(-1);
        let pty = PathBuf::from(format!("/dev/pts/{number}"));
        let owned = std::fs::metadata(&pty).is_ok_and(|meta| meta.uid() == uid);
        if pid <= 0 || !alive(pid) || !owned {
            let _ = std::fs::remove_file(&entry);
            continue;
        }
        ptys.push(pty);
    }
    ptys
}

fn emit(pty: &Path, sequence: &[u8]) -> bool {
    let opened = OpenOptions::new()
        .write(true)
        .custom_flags((OFlags::NOCTTY | OFlags::NONBLOCK).bits() as i32)
        .open(pty);
    match opened {
        Ok(mut file) => file.write_all(sequence).is_ok(),
        Err(_) => false,
    }
}

/// The OSC, then a bell of its own. A terminal reads the OSC whole, but a
/// Mosh shell's pty ends at a server that ships a screen and not a byte
/// stream, and an OSC it does not know never leaves the host; the bell does,
/// and a phone that hears one asks the runtime directory what is open.
pub fn request_sequence(request_id: &str, payload_json: &str) -> Vec<u8> {
    let body = base64::engine::general_purpose::STANDARD.encode(payload_json.as_bytes());
    format!("\x1b]{OSC};pin;{request_id};{body}\x07\x07").into_bytes()
}

pub fn done_sequence(request_id: &str) -> Vec<u8> {
    format!("\x1b]{OSC};done;{request_id}\x07\x07").into_bytes()
}

pub fn notify(runtime: &Path, request_id: &str, payload_json: &str) -> Vec<PathBuf> {
    let sequence = request_sequence(request_id, payload_json);
    marked(runtime).into_iter().filter(|pty| emit(pty, &sequence)).collect()
}

pub fn done(ptys: &[PathBuf], request_id: &str) {
    let sequence = done_sequence(request_id);
    for pty in ptys {
        emit(pty, &sequence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags};
    use std::fs::File;
    use std::os::unix::io::OwnedFd;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pinentry-gate-phone-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tty")).unwrap();
        dir
    }

    fn openpty() -> (OwnedFd, File, String) {
        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
        grantpt(&master).unwrap();
        unlockpt(&master).unwrap();
        let name = ptsname(&master, Vec::new()).unwrap().into_string().unwrap();
        let slave = OpenOptions::new().read(true).write(true).custom_flags(OFlags::NOCTTY.bits() as i32).open(&name).unwrap();
        (master, slave, name)
    }

    fn read(master: &OwnedFd) -> Vec<u8> {
        let mut buf = vec![0u8; 4096];
        let n = rustix::io::read(master, &mut buf).unwrap();
        assert!(n > 0);
        buf.truncate(n);
        buf
    }

    #[test]
    fn an_ssh_session_a_mosh_server_or_a_dazzle_engine_holds_a_pty() {
        for comm in ["sshd", "sshd-session", "mosh-server", "dazzle-engine"] {
            assert!(holds_a_pty(comm), "{comm}");
        }
        for comm in ["dazzle", "mosh-client", "tmux: server", "zsh"] {
            assert!(!holds_a_pty(comm), "{comm}");
        }
    }

    #[test]
    fn stale_marks_are_pruned() {
        let dir = scratch("stale");
        std::fs::write(dir.join("tty/pts-9999"), "999999").unwrap();
        std::fs::write(dir.join("tty/junk"), "1").unwrap();
        assert!(marked_by(&dir, |_| false).is_empty());
        assert!(!dir.join("tty/pts-9999").exists());
        assert!(dir.join("tty/junk").exists());
    }

    #[test]
    fn a_live_mark_gets_the_request_and_the_done() {
        let dir = scratch("live");
        let (master, slave, name) = openpty();
        let number = name.rsplit('/').next().unwrap();
        std::fs::write(dir.join(format!("tty/pts-{number}")), std::process::id().to_string()).unwrap();
        let me = std::process::id() as i32;

        let told = marked_by(&dir, |pid| pid == me);
        assert_eq!(told, vec![PathBuf::from(&name)]);
        assert!(emit(&told[0], &request_sequence("abcd", r#"{"desc":"hi"}"#)));
        let raw = read(&master);
        let head = b"\x1b]7771;pin;abcd;";
        assert!(raw.starts_with(head));
        assert!(raw.ends_with(b"\x07\x07"));
        let body = &raw[head.len()..raw.len() - 2];
        let decoded = base64::engine::general_purpose::STANDARD.decode(body).unwrap();
        assert_eq!(decoded, br#"{"desc":"hi"}"#);

        done(&told, "abcd");
        assert_eq!(read(&master), b"\x1b]7771;done;abcd\x07\x07");
        drop((master, slave));
    }
}
