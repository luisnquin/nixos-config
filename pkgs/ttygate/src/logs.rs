use std::collections::VecDeque;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct LogBuffer {
    lines: VecDeque<String>,
    cap: usize,
}

impl LogBuffer {
    pub fn new(cap: usize) -> Self {
        LogBuffer {
            lines: VecDeque::with_capacity(cap),
            cap: cap.max(1),
        }
    }

    pub fn push(&mut self, line: String) {
        if self.lines.len() == self.cap {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// A window of at most `rows` lines whose bottom edge sits `from_bottom`
    /// lines above the newest line, oldest first. Powers the scrollable viewer.
    pub fn window(&self, from_bottom: usize, rows: usize) -> impl Iterator<Item = &str> {
        let end = self.lines.len().saturating_sub(from_bottom);
        let start = end.saturating_sub(rows);
        self.lines.iter().take(end).skip(start).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

// Streams `cmd` stdout over a channel. A spawn failure or dead pipe reports one
// line then goes quiet; it never panics and never blocks the login flow.
pub fn spawn_logs(cmd: Vec<String>) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel(128);

    tokio::spawn(async move {
        if cmd.is_empty() {
            return;
        }
        let mut command = Command::new(&cmd[0]);
        command
            .args(&cmd[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = tx.send(format!("// log source unavailable: {e}")).await;
                return;
            }
        };

        let Some(stdout) = child.stdout.take() else {
            return;
        };
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if tx.send(line).await.is_err() {
                break;
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest_past_capacity() {
        let mut b = LogBuffer::new(2);
        b.push("a".into());
        b.push("b".into());
        b.push("c".into());
        let got: Vec<_> = b.window(0, 5).collect();
        assert_eq!(got, vec!["b", "c"]);
    }

    #[test]
    fn window_returns_last_n_oldest_first() {
        let mut b = LogBuffer::new(10);
        for s in ["one", "two", "three"] {
            b.push(s.into());
        }
        let got: Vec<_> = b.window(0, 2).collect();
        assert_eq!(got, vec!["two", "three"]);
    }

    #[test]
    fn window_offset_from_bottom_skips_newest() {
        let mut b = LogBuffer::new(10);
        for s in ["one", "two", "three", "four"] {
            b.push(s.into());
        }
        // Two lines up from the bottom, a 2-row window shows one/two.
        let got: Vec<_> = b.window(2, 2).collect();
        assert_eq!(got, vec!["one", "two"]);
    }
}
