//! Runs the user-supplied `--preview` command. cliplenz stays agnostic about
//! the clipboard backend: it just feeds the focused line in and reads bytes out.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run `cmd` through `sh -c`, write `line` (+newline, matching a here-string)
/// to its stdin, and return its raw stdout.
pub fn run_preview(cmd: &str, line: &str) -> Result<Vec<u8>, String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn preview: {e}"))?;

    {
        let mut stdin = child.stdin.take().ok_or("preview stdin unavailable")?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|e| format!("write preview stdin: {e}"))?;
    }

    let out = child
        .wait_with_output()
        .map_err(|e| format!("wait preview: {e}"))?;
    if !out.status.success() {
        return Err(format!("preview command exited {}", out.status));
    }
    Ok(out.stdout)
}
