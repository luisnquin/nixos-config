//! End-to-end proof that one upstream process serves every supported client revision.
//!
//! Each test drives a private `mcp-gatewayd` over its own socket, against a fake stdio server that
//! records one line per spawn. Nothing here touches the user's gateway.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const READ_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn different_client_revisions_share_one_backend() {
    let gateway = Gateway::start("2025-06-18");

    let mut newer = gateway.client();
    let newer_init = newer.initialize("2025-06-18");
    assert_eq!(newer_init["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(
        newer_init["result"]["serverInfo"]["title"],
        json!("Fake Server")
    );

    let mut older = gateway.client();
    let older_init = older.initialize("2025-03-26");
    assert_eq!(older_init["result"]["protocolVersion"], json!("2025-03-26"));
    assert_eq!(older_init["result"]["serverInfo"].get("title"), None);

    let newer_tools = newer.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(newer_tools["result"]["tools"][0]["title"], json!("Echo"));
    assert!(
        newer_tools["result"]["tools"][0]
            .get("outputSchema")
            .is_some()
    );

    let older_tools = older.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(older_tools["result"]["tools"][0]["name"], json!("echo"));
    assert_eq!(older_tools["result"]["tools"][0].get("title"), None);
    assert_eq!(older_tools["result"]["tools"][0].get("outputSchema"), None);

    assert_eq!(
        gateway.spawn_count(),
        1,
        "two revisions must share one backend"
    );

    let log = gateway.daemon_log();
    assert!(
        log.contains("gateway_started"),
        "startup log missing: {log}"
    );
    assert!(
        log.contains("protocol_bridged"),
        "divergence log missing: {log}"
    );
    assert!(
        log.contains("payload_translated"),
        "translation log missing: {log}"
    );
}

#[test]
fn same_revision_clients_share_one_backend() {
    let gateway = Gateway::start("2025-06-18");

    let mut first = gateway.client();
    first.initialize("2025-06-18");
    let mut second = gateway.client();
    second.initialize("2025-06-18");

    first.request(json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}));
    second.request(json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}));

    assert_eq!(gateway.spawn_count(), 1);
    assert!(!gateway.daemon_log().contains("protocol_bridged"));
}

#[test]
fn request_ids_stay_isolated_between_clients() {
    let gateway = Gateway::start("2025-06-18");

    let mut newer = gateway.client();
    newer.initialize("2025-06-18");
    let mut older = gateway.client();
    older.initialize("2025-03-26");

    // Both clients reuse id 1 concurrently, and one of them uses a string id.
    newer.send(&call("echo", "newer", json!(1)));
    older.send(&call("echo", "older", json!("shared-id")));

    let newer_response = newer.recv();
    let older_response = older.recv();

    assert_eq!(newer_response["id"], json!(1));
    assert_eq!(
        newer_response["result"]["content"][0]["text"],
        json!("newer")
    );
    assert_eq!(older_response["id"], json!("shared-id"));
    assert_eq!(
        older_response["result"]["content"][0]["text"],
        json!("older")
    );
    assert_eq!(gateway.spawn_count(), 1);
}

#[test]
fn unsupported_semantics_fail_visibly() {
    let gateway = Gateway::start("2025-06-18");

    let mut older = gateway.client();
    older.initialize("2025-03-26");

    let link = older.request(call("link", "", json!(1)));
    assert_eq!(link["id"], json!(1));
    assert!(
        link["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("resource_link")),
        "resource links must be refused, got {link}"
    );

    let structured = older.request(call("structured", "", json!(2)));
    assert!(
        structured["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("structuredContent")),
        "structured-only results must be refused, got {structured}"
    );

    let elicitation = older.request(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "elicitation/create",
        "params": {},
    }));
    assert_eq!(elicitation["error"]["code"], json!(-32601));

    // A 2025-06-18 client is served the very same payloads without complaint.
    let mut newer = gateway.client();
    newer.initialize("2025-06-18");
    let newer_link = newer.request(call("link", "", json!(1)));
    assert_eq!(
        newer_link["result"]["content"][0]["type"],
        json!("resource_link")
    );

    assert_eq!(gateway.spawn_count(), 1);
    let log = gateway.daemon_log();
    assert!(
        log.contains("contract_rejected"),
        "rejection log missing: {log}"
    );
}

#[test]
fn unknown_revisions_fail_closed_without_a_second_backend() {
    let gateway = Gateway::start("2025-06-18");

    let mut future = gateway.client();
    let response = future.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "2099-01-01", "capabilities": {}},
    }));
    let message = response["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("2099-01-01"), "got {response}");
    assert!(
        message.contains("compat.rs"),
        "error must be actionable: {response}"
    );
    assert_eq!(
        gateway.spawn_count(),
        0,
        "a refused revision must never start a backend"
    );

    let batch = future.request_raw("[{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}]");
    assert!(
        batch["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("batching")),
        "batches must be refused, got {batch}"
    );

    assert!(gateway.daemon_log().contains("protocol_rejected"));
}

#[test]
fn server_initiated_pings_are_answered_by_the_gateway() {
    let gateway = Gateway::start("2025-06-18");

    let mut client = gateway.client();
    client.initialize("2025-06-18");
    let response = client.request(call("server_ping", "", json!(1)));
    assert_eq!(
        response["result"]["content"][0]["text"],
        json!("ping-answered")
    );
}

#[test]
fn a_downlevel_upstream_serves_modern_clients() {
    let gateway = Gateway::start("2024-11-05");

    let mut newer = gateway.client();
    let init = newer.initialize("2025-06-18");
    assert_eq!(init["result"]["protocolVersion"], json!("2025-06-18"));

    let tools = newer.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(tools["result"]["tools"][0]["name"], json!("echo"));
    let echoed = newer.request(call("echo", "downlevel", json!(2)));
    assert_eq!(echoed["result"]["content"][0]["text"], json!("downlevel"));

    let mut older = gateway.client();
    let older_init = older.initialize("2025-03-26");
    assert_eq!(older_init["result"]["protocolVersion"], json!("2025-03-26"));

    assert_eq!(gateway.spawn_count(), 1);
    assert!(gateway.daemon_log().contains("protocol_downlevel"));
}

#[test]
fn a_precondition_appearing_heals_an_attached_client() {
    let gateway = Gateway::start_with(
        "2025-06-18",
        json!({
            "scope": "workspace",
            "requires": {"any_file_exists": ["encore.app"]},
        }),
    );

    let mut client = gateway.client();
    client.initialize("2025-06-18");
    let tools = client.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(tools["result"]["tools"], json!([]));

    std::fs::write(gateway.directory.join("encore.app"), "{}").expect("write precondition file");

    let tools = client.request(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert_eq!(
        tools["result"]["tools"][0]["name"],
        json!("echo"),
        "a client attached under the empty surface must reach the spawn, not an error"
    );
    assert_eq!(gateway.spawn_count(), 1);
}

#[test]
fn unmet_precondition_serves_an_empty_surface_without_spawning() {
    let gateway = Gateway::start_with(
        "2025-06-18",
        json!({
            "scope": "workspace",
            "requires": {"any_file_exists": ["encore.app"]},
        }),
    );

    let mut client = gateway.client();
    let init = client.initialize("2025-06-18");
    assert_eq!(init["result"]["capabilities"], json!({}));
    let tools = client.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(tools["result"]["tools"], json!([]));
    assert_eq!(
        gateway.spawn_count(),
        0,
        "an unmet precondition must never spawn"
    );
    assert!(gateway.daemon_log().contains("precondition_unmet"));

    // The workspace gaining the file heals the next handshake without a daemon restart.
    std::fs::write(gateway.directory.join("encore.app"), "{}").expect("write precondition file");
    let mut second = gateway.client();
    let init = second.initialize("2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["title"], json!("Fake Server"));
    let tools = second.request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(tools["result"]["tools"][0]["name"], json!("echo"));
    assert_eq!(gateway.spawn_count(), 1);
}

#[test]
fn idle_traffic_reaps_the_connection_and_use_respawns_it() {
    let gateway = Gateway::start_with(
        "2025-06-18",
        json!({"reap_when_idle": true, "idle_seconds": 1}),
    );

    let mut client = gateway.client();
    client.initialize("2025-06-18");
    let first = client.request(call("echo", "one", json!(1)));
    assert_eq!(first["result"]["content"][0]["text"], json!("one"));
    assert_eq!(gateway.spawn_count(), 1);

    gateway.wait_for_log("upstream_reaped");

    let mut late = gateway.client();
    late.initialize("2025-06-18");
    assert_eq!(
        gateway.spawn_count(),
        1,
        "a handshake alone must not revive a reaped backend"
    );

    let second = late.request(call("echo", "two", json!(1)));
    assert_eq!(second["result"]["content"][0]["text"], json!("two"));
    assert_eq!(
        gateway.spawn_count(),
        2,
        "first use after the reap respawns once"
    );
}

fn call(tool: &str, marker: &str, id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": tool, "arguments": {"marker": marker}},
    })
}

struct Gateway {
    directory: PathBuf,
    socket: PathBuf,
    spawn_log: PathBuf,
    daemon_log: PathBuf,
    child: Child,
}

impl Gateway {
    fn start(upstream_version: &str) -> Self {
        Self::start_with(upstream_version, json!({}))
    }

    /// Starts a gateway whose single `fake` server carries extra config fields on top of the
    /// defaults, e.g. `requires` or `reap_when_idle`.
    fn start_with(upstream_version: &str, overrides: Value) -> Self {
        let directory = scratch_directory();
        let socket = directory.join("gateway.sock");
        let spawn_log = directory.join("spawns.log");
        let daemon_log = directory.join("daemon.log");
        let config = directory.join("config.json");

        let mut server = json!({
            "command": fake_server().to_string_lossy(),
            "args": [spawn_log.to_string_lossy(), upstream_version],
            "scope": "global",
        });
        for (key, value) in overrides.as_object().expect("object overrides") {
            server[key] = value.clone();
        }

        std::fs::write(
            &config,
            serde_json::to_vec_pretty(&json!({"servers": {"fake": server}}))
                .expect("encode config"),
        )
        .expect("write config");

        let log = std::fs::File::create(&daemon_log).expect("create daemon log");
        let upstream_log = log.try_clone().expect("clone daemon log");
        let child = Command::new(env!("CARGO_BIN_EXE_mcp-gatewayd"))
            .arg("--config")
            .arg(&config)
            .arg("--socket")
            .arg(&socket)
            .env("RUST_LOG", "info")
            .stdin(Stdio::null())
            .stdout(upstream_log)
            .stderr(log)
            .spawn()
            .expect("spawn gateway daemon");

        let gateway = Self {
            directory,
            socket,
            spawn_log,
            daemon_log,
            child,
        };
        gateway.wait_until_listening();
        gateway
    }

    fn wait_until_listening(&self) {
        let deadline = Instant::now() + READ_TIMEOUT;
        while Instant::now() < deadline {
            if UnixStream::connect(&self.socket).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("gateway never bound {}", self.socket.display());
    }

    fn client(&self) -> Client {
        Client::connect(&self.socket, &self.directory)
    }

    fn spawn_count(&self) -> usize {
        std::fs::read_to_string(&self.spawn_log)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    }

    fn daemon_log(&self) -> String {
        std::fs::read_to_string(&self.daemon_log).unwrap_or_default()
    }

    fn wait_for_log(&self, needle: &str) {
        let deadline = Instant::now() + READ_TIMEOUT;
        while Instant::now() < deadline {
            if self.daemon_log().contains(needle) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("daemon log never contained {needle}: {}", self.daemon_log());
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct Client {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    fn connect(socket: &Path, workspace: &Path) -> Self {
        let stream = UnixStream::connect(socket).expect("connect to gateway");
        stream
            .set_read_timeout(Some(READ_TIMEOUT))
            .expect("set read timeout");
        let reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut client = Self {
            writer: stream,
            reader,
        };
        client.send_raw(
            &serde_json::to_string(&json!({"server": "fake", "workspace": workspace}))
                .expect("encode subscription"),
        );
        client
    }

    fn initialize(&mut self, version: &str) -> Value {
        let response = self.request(json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {"protocolVersion": version, "capabilities": {}},
        }));
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        response
    }

    fn request(&mut self, message: Value) -> Value {
        self.send(&message);
        self.recv()
    }

    fn request_raw(&mut self, message: &str) -> Value {
        self.send_raw(message);
        self.recv()
    }

    fn send(&mut self, message: &Value) {
        self.send_raw(&serde_json::to_string(message).expect("encode message"));
    }

    fn send_raw(&mut self, message: &str) {
        writeln!(self.writer, "{message}").expect("write message");
        self.writer.flush().expect("flush message");
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("read response");
        assert!(!line.is_empty(), "gateway closed the connection");
        serde_json::from_str(&line).expect("decode response")
    }
}

fn scratch_directory() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "nyx-mcp-gateway-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).expect("create scratch directory");
    directory
}

/// Compiles the fake server once per test binary, outside the cargo target graph so it can never
/// be installed alongside the real gateway binaries.
fn fake_server() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let directory = scratch_directory();
        let source = directory.join("fake_mcp_server.rs");
        let binary = directory.join("fake-mcp-server");
        std::fs::write(&source, include_str!("fixtures/fake_mcp_server.rs"))
            .expect("write fake server source");
        let status = Command::new("rustc")
            .arg("--edition=2021")
            .arg("-O")
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .status()
            .expect("run rustc");
        assert!(status.success(), "failed to compile the fake MCP server");
        binary
    })
}
