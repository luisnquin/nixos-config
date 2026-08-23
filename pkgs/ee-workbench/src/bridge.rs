use std::fmt;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// Wire version. The server is `ee-freecad-server`, a native FreeCAD module;
/// a mismatch here means one of the two binaries is stale, and both sides
/// refuse rather than misread a frame.
pub const PROTOCOL: u32 = 4;

/// The three verbs that keep working across a build mismatch. Refusing every
/// method would strand a stale session holding unsaved work: you could neither
/// see what it has, write it out, nor retire it. Everything else refuses,
/// because a session started by an older generation does not behave the way the
/// caller's `ee` was written against, and answering anyway is how that goes
/// unnoticed.
const RESCUE: [&str; 3] = ["session.status", "document.save", "server.shutdown"];

/// The peer closed the connection without answering. Typed rather than a
/// string so a caller can tell it apart from a refusal the server actually
/// spoke, and decide whether replaying the request is safe.
#[derive(Debug)]
pub struct Hangup {
    pub method: String,
}

impl fmt::Display for Hangup {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "the cad session closed the connection during {}",
            self.method
        )
    }
}

impl std::error::Error for Hangup {}

/// True when the failure means nothing was listening any more, which is what a
/// session retiring on its idle deadline looks like from this side. A refusal,
/// a protocol mismatch or unparseable JSON all mean the request reached a live
/// server and must not be replayed.
pub fn is_disconnect(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        if cause.downcast_ref::<Hangup>().is_some() {
            return true;
        }

        cause.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                ErrorKind::ConnectionRefused
                    | ErrorKind::ConnectionReset
                    | ErrorKind::BrokenPipe
                    | ErrorKind::NotFound
            )
        })
    })
}

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
            return Err(anyhow::Error::new(Hangup {
                method: method.to_string(),
            }));
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

        // Drift the protocol version cannot see. `PROTOCOL` moves when the wire
        // shape changes, which is far rarer than a change in what a method
        // does; between those bumps two servers are indistinguishable to the
        // guard above. The reply names the build that answered, so a session
        // left listening by an older generation says so on the first request
        // rather than quietly serving last week's behaviour.
        if let Some(expected) = crate::spawn::expected_build() {
            let running = reply
                .get("build")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if running != expected && !RESCUE.contains(&method) {
                // Silence is drift too, and the loudest kind: a server old
                // enough not to report a build at all is older than everything
                // this check was written for.
                let named = if running.is_empty() {
                    "does not say which build it is".to_string()
                } else {
                    format!("is {running}")
                };
                bail!(
                    "the cad session on this socket {named}, but this ee is paired with \
                     {expected}: it was started by an older generation and still holds the \
                     socket. Save anything open, then `ee mechanical session stop`; the next \
                     command starts the right one."
                );
            }
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

    /// The build expectation is a process-wide environment variable and cargo
    /// runs test functions in parallel threads, so every test that reaches
    /// `call` serializes here rather than reading a neighbour's setting.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn exclusive() -> std::sync::MutexGuard<'static, ()> {
        ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ee-cad-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cad.sock")
    }

    #[test]
    fn a_result_comes_back_unwrapped() {
        let _exclusive = exclusive();
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
        let _exclusive = exclusive();
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
        let _exclusive = exclusive();
        let socket = scratch("stale");
        spawn_mock(
            &socket,
            vec![json!({ "ok": true, "protocol": 1, "id": 1, "result": {} }).to_string()],
        );

        let error = call(&socket, "session.status", json!({})).unwrap_err();
        assert!(error.to_string().contains("rebuild both"), "{error}");
    }

    #[test]
    fn a_hangup_is_told_apart_from_a_refusal() {
        let _exclusive = exclusive();
        // No replies at all: the mock accepts and then drops the connection,
        // which is exactly what a session retiring mid-call looks like.
        let vanishing = scratch("hangup");
        spawn_mock(&vanishing, vec![]);
        let error = call(&vanishing, "session.status", json!({})).unwrap_err();
        assert!(is_disconnect(&error), "{error:#}");

        let absent = scratch("gone").with_file_name("nothing.sock");
        let error = call(&absent, "session.status", json!({})).unwrap_err();
        assert!(is_disconnect(&error), "{error:#}");

        let live = scratch("live");
        spawn_mock(
            &live,
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
        let error = call(&live, "document.inspect", json!({})).unwrap_err();
        assert!(!is_disconnect(&error), "{error:#}");
    }

    /// The other tests' mocks answer without a `build`, so the guard skips them
    /// and this can set the process-wide variable while they run.
    #[test]
    fn a_session_from_another_build_is_refused_but_still_stoppable() {
        let _exclusive = exclusive();
        let reply = |tag: &str, method: &str, build: Option<&str>| {
            let socket = scratch(&format!("drift-{tag}-{}", method.replace('.', "-")));
            let mut envelope = json!({ "ok": true, "protocol": PROTOCOL, "id": 1, "result": {} });
            if let Some(build) = build {
                envelope["build"] = json!(build);
            }
            spawn_mock(&socket, vec![envelope.to_string()]);
            call(&socket, method, json!({}))
        };

        unsafe { std::env::set_var(crate::spawn::BUILD_ENV, "/nix/store/new-ee-freecad-server") };

        let named = reply(
            "named",
            "document.new",
            Some("/nix/store/old-ee-freecad-server"),
        );
        let error = named.unwrap_err();
        assert!(error.to_string().contains("older generation"), "{error}");
        assert!(!is_disconnect(&error), "a mismatch must not be retried");

        // A server predating the field says nothing at all, which is the drift
        // this whole check exists for and must not read as agreement.
        let error = reply("silent", "document.new", None).unwrap_err();
        assert!(error.to_string().contains("does not say"), "{error}");

        for rescue in RESCUE {
            assert!(
                reply("named", rescue, Some("/nix/store/old")).is_ok(),
                "{rescue} has to survive the drift"
            );
            assert!(reply("silent", rescue, None).is_ok(), "{rescue} likewise");
        }

        unsafe { std::env::remove_var(crate::spawn::BUILD_ENV) };
    }

    #[test]
    fn a_missing_socket_names_the_server() {
        let _exclusive = exclusive();
        let socket = scratch("missing").with_file_name("absent.sock");

        let error = call(&socket, "session.status", json!({})).unwrap_err();
        assert!(
            format!("{error:#}").contains("ee-freecad-server"),
            "{error:#}"
        );
    }
}
