//! Minimal stdio MCP server used by the gateway integration tests.
//!
//! Compiled with plain `rustc` at test time so it never becomes an installed binary. Requests are
//! inspected with substring scans instead of a JSON parser: the gateway serialises maps in sorted
//! key order, so the first `"id":` occurrence is always the top-level request id.

use std::io::{BufRead, Write};

fn main() {
    let mut args = std::env::args().skip(1);
    let spawn_log = args.next().expect("spawn log path");
    let version = args.next().unwrap_or_else(|| "2025-06-18".to_owned());

    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&spawn_log)
        .expect("open spawn log");
    writeln!(log, "{}", std::process::id()).expect("record spawn");
    log.flush().expect("flush spawn log");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut lines = stdin.lock().lines();
    while let Some(line) = lines.next() {
        let Ok(line) = line else { break };
        let Some(method) = string_field(&line, "method") else {
            continue;
        };
        let id = raw_id(&line);
        // Each arm yields the response payload without its enclosing braces.
        let payload = match method {
            // Sends a ping to the client and withholds the tool result until it is answered,
            // mirroring servers that use ping as a link keepalive.
            "tools/call" if string_field(&line, "name") == Some("server_ping") => {
                writeln!(stdout, "{{\"jsonrpc\":\"2.0\",\"id\":9990,\"method\":\"ping\"}}")
                    .expect("write ping");
                stdout.flush().expect("flush ping");
                let answer = lines
                    .next()
                    .and_then(Result::ok)
                    .unwrap_or_default();
                let text = if answer.contains("9990") && answer.contains("\"result\"") {
                    "ping-answered"
                } else {
                    "ping-refused"
                };
                format!("\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":\"{text}\"}}]}}")
            }
            "initialize" => format!(
                "\"result\":{{\"protocolVersion\":\"{version}\",\
                 \"capabilities\":{{\"tools\":{{}},\"prompts\":{{}}}},\
                 \"serverInfo\":{{\"name\":\"fake\",\"version\":\"0\",\"title\":\"Fake Server\"}}}}"
            ),
            "ping" => "\"result\":{}".to_owned(),
            "tools/list" => "\"result\":{\"tools\":[\
                 {\"name\":\"echo\",\"title\":\"Echo\",\"inputSchema\":{\"type\":\"object\"},\
                 \"outputSchema\":{\"type\":\"object\"}}]}"
                .to_owned(),
            "tools/call" => tool_payload(&line),
            _ => "\"error\":{\"code\":-32601,\"message\":\"unknown method\"}".to_owned(),
        };
        let Some(id) = id else { continue };
        writeln!(stdout, "{{\"jsonrpc\":\"2.0\",\"id\":{id},{payload}}}").expect("write response");
        stdout.flush().expect("flush response");
    }
}

fn tool_payload(line: &str) -> String {
    match string_field(line, "name") {
        Some("link") => "\"result\":{\"content\":[{\"type\":\"resource_link\",\
                         \"uri\":\"file:///x\",\"name\":\"x\"}]}"
            .to_owned(),
        Some("structured") => {
            "\"result\":{\"content\":[],\"structuredContent\":{\"ok\":true}}".to_owned()
        }
        _ => {
            let marker = string_field(line, "marker").unwrap_or("none");
            format!("\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":\"{marker}\"}}]}}")
        }
    }
}

fn string_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn raw_id(line: &str) -> Option<&str> {
    let start = line.find("\"id\":")? + "\"id\":".len();
    let rest = &line[start..];
    let end = rest.find([',', '}'])?;
    Some(rest[..end].trim())
}
