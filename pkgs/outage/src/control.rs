use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Request {
    Arm,
    Disarm,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub ok: bool,
    pub phase: String,
    pub detail: String,
}

impl Reply {
    pub fn new(ok: bool, phase: &str, detail: impl Into<String>) -> Self {
        Self {
            ok,
            phase: phase.to_string(),
            detail: detail.into(),
        }
    }
}

pub fn bind(path: &Path, group: &str) -> io::Result<UnixListener> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o660))?;
    if let Some(gid) = group_id(group) {
        chown_to_group(path, gid)?;
    } else {
        crate::log(&format!(
            "unknown control group '{group}', socket left root-only"
        ));
    }
    Ok(listener)
}

fn group_id(group: &str) -> Option<u32> {
    let name = std::ffi::CString::new(group).ok()?;
    let entry = unsafe { libc::getgrnam(name.as_ptr()) };
    if entry.is_null() {
        return None;
    }
    Some(unsafe { (*entry).gr_gid })
}

fn chown_to_group(path: &Path, gid: u32) -> io::Result<()> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let rc = unsafe { libc::chown(c_path.as_ptr(), u32::MAX, gid) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn user_id(user: &str) -> Option<u32> {
    let name = std::ffi::CString::new(user).ok()?;
    let entry = unsafe { libc::getpwnam(name.as_ptr()) };
    if entry.is_null() {
        return None;
    }
    Some(unsafe { (*entry).pw_uid })
}

const READ_TIMEOUT: Duration = Duration::from_secs(2);

pub fn read_request(stream: &UnixStream) -> io::Result<Request> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    serde_json::from_str(line.trim()).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

pub fn write_reply(mut stream: &UnixStream, reply: &Reply) -> io::Result<()> {
    let mut line = serde_json::to_string(reply)?;
    line.push('\n');
    stream.write_all(line.as_bytes())
}

pub fn request(path: &Path, req: Request, timeout: Duration) -> io::Result<Reply> {
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(timeout))?;
    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    (&stream).write_all(line.as_bytes())?;

    let mut response = String::new();
    BufReader::new(&stream).read_line(&mut response)?;
    serde_json::from_str(response.trim())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_use_stable_lowercase_names() {
        assert_eq!(serde_json::to_string(&Request::Arm).unwrap(), "\"arm\"");
        assert_eq!(
            serde_json::to_string(&Request::Disarm).unwrap(),
            "\"disarm\""
        );
        assert_eq!(
            serde_json::from_str::<Request>("\"status\"").unwrap(),
            Request::Status
        );
    }

    #[test]
    fn replies_round_trip() {
        let reply = Reply::new(true, "armed", "watching");
        let text = serde_json::to_string(&reply).unwrap();
        assert_eq!(serde_json::from_str::<Reply>(&text).unwrap(), reply);
    }

    #[test]
    fn a_round_trip_over_a_real_socket_works() {
        let dir = std::env::temp_dir().join(format!("outage-sock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("control.sock");

        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let req = read_request(&stream).unwrap();
            write_reply(&stream, &Reply::new(true, "disarmed", format!("{req:?}"))).unwrap();
        });

        let reply = request(&path, Request::Disarm, Duration::from_secs(5)).unwrap();
        assert!(reply.ok);
        assert_eq!(reply.phase, "disarmed");
        assert_eq!(reply.detail, "Disarm");

        server.join().unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
