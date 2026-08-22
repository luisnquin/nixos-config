use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// Wire version. The server is `ee-freecad-server`, a native FreeCAD module;
/// a mismatch here means one of the two binaries is stale, and both sides
/// refuse rather than misread a frame.
pub const PROTOCOL: u32 = 3;

/// NDJSON over the CAD socket: one JSON object per line in both directions.
/// The connection is stateful only in that the server keeps the FreeCAD
/// documents; requests themselves are independent.
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
}

impl Client {
    pub fn connect(socket: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket).with_context(|| {
            format!(
                "connecting to {}: is ee-freecad-server running?",
                socket.display()
            )
        })?;

        Ok(Self {
            reader: BufReader::new(stream.try_clone().context("cloning the cad socket")?),
            writer: stream,
            next_id: 1,
        })
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "id": id,
            "protocol": PROTOCOL,
            "method": method,
            "params": params,
        });

        writeln!(self.writer, "{request}").context("writing a cad request")?;
        self.writer.flush().context("flushing a cad request")?;

        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .context("reading a cad reply")?;
        if read == 0 {
            bail!("the cad session closed the connection during {method}");
        }

        let reply: Value = serde_json::from_str(line.trim())
            .with_context(|| format!("parsing the cad reply to {method}"))?;

        match reply.get("protocol").and_then(Value::as_u64) {
            Some(version) if version == u64::from(PROTOCOL) => {}
            Some(version) => bail!(
                "cad session speaks protocol {version}, this ee speaks {PROTOCOL}: rebuild both"
            ),
            None => bail!("cad reply to {method} carries no protocol version"),
        }

        if reply.get("ok").and_then(Value::as_bool) != Some(true) {
            let code = reply
                .pointer("/error/code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = reply
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("no message");
            bail!("{method} refused [{code}]: {message}");
        }

        Ok(reply.get("result").cloned().unwrap_or(Value::Null))
    }
}

/// One request on a fresh connection, for commands that do a single thing.
pub fn call(socket: &Path, method: &str, params: Value) -> Result<Value> {
    Client::connect(socket)?.call(method, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    /// A stand-in for the native server: enough to prove the client's framing,
    /// protocol guard and error mapping without linking FreeCAD.
    fn spawn_mock(socket: &Path, replies: Vec<String>) {
        let listener = UnixListener::bind(socket).unwrap();

        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let reader = BufReader::new(stream);

            for (line, reply) in reader.lines().zip(replies) {
                line.unwrap();
                writeln!(writer, "{reply}").unwrap();
                writer.flush().unwrap();
            }
        });
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ee-cad-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cad.sock")
    }

    #[test]
    fn a_result_comes_back_unwrapped() {
        let socket = scratch("ok");
        spawn_mock(
            &socket,
            vec![
                json!({ "ok": true, "protocol": PROTOCOL, "id": 1, "result": { "dof": 0 } })
                    .to_string(),
            ],
        );

        let result = call(&socket, "session.status", json!({})).unwrap();
        assert_eq!(result["dof"], json!(0));
    }

    #[test]
    fn a_refusal_keeps_its_code() {
        let socket = scratch("refusal");
        spawn_mock(
            &socket,
            vec![
                json!({
                    "ok": false,
                    "protocol": PROTOCOL,
                    "id": 1,
                    "error": { "code": "unknown-document", "message": "no active document" }
                })
                .to_string(),
            ],
        );

        let error = call(&socket, "document.inspect", json!({})).unwrap_err();
        assert!(error.to_string().contains("unknown-document"), "{error}");
        assert!(error.to_string().contains("no active document"), "{error}");
    }

    #[test]
    fn a_stale_server_is_refused() {
        let socket = scratch("stale");
        spawn_mock(
            &socket,
            vec![json!({ "ok": true, "protocol": 1, "id": 1, "result": {} }).to_string()],
        );

        let error = call(&socket, "session.status", json!({})).unwrap_err();
        assert!(error.to_string().contains("rebuild both"), "{error}");
    }

    #[test]
    fn a_missing_socket_names_the_server() {
        let socket = scratch("missing").with_file_name("absent.sock");

        let error = call(&socket, "session.status", json!({})).unwrap_err();
        assert!(
            format!("{error:#}").contains("ee-freecad-server"),
            "{error:#}"
        );
    }
}
