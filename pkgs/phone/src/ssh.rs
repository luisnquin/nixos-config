use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use crate::discover::sweep;
use crate::registry::state_dir;

/// Multiplexed control sockets, so a survey's several commands per host share
/// one handshake. `%C` keys the socket on the tuple ssh resolved.
fn control_path() -> PathBuf {
    let dir = state_dir().join("ssh");
    let _ = std::fs::create_dir_all(&dir);

    dir.join("%C")
}

/// Connection hygiene only; where `host` is and how to reach it is ssh's.
pub fn command(host: &str) -> Command {
    let mut cmd = Command::new("ssh");

    cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"]);
    cmd.args(["-o", "ControlMaster=auto", "-o", "ControlPersist=60"]);
    cmd.arg("-o")
        .arg(format!("ControlPath={}", control_path().display()));
    cmd.arg(host);
    cmd.stdin(Stdio::null());

    cmd
}

fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// One remote command line running `script` with `args` bound to `$1`, `$2`, …
/// Positional rather than interpolated, so a device or bundle id is never read
/// as shell syntax.
///
/// Quoted into a single word because ssh has no argv: it joins everything after
/// the host with spaces and lets the remote shell split it again.
pub fn remote(script: &str, args: &[&str]) -> String {
    let mut out = format!("sh -c {} sh", quoted(script));

    for arg in args {
        out.push(' ');
        out.push_str(&quoted(arg));
    }

    out
}

pub fn script(host: &str, script: &str, args: &[&str]) -> Command {
    let mut cmd = command(host);

    cmd.arg(remote(script, args));

    cmd
}

/// Marks the line a remote run's exit code is written on.
const STATUS: &str = "phone-status:";

/// How a remote command ended, which is not always a number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// It ran, and exited with this.
    Code(i32),
    /// The status line arrived carrying something that is not a code. The
    /// command ran and its outcome is lost, which is not the same as never
    /// having reached it — this one is not worth retrying.
    Garbled(String),
    /// No status line at all: the session died before the command ended, or
    /// never opened. The one case where trying again is the right answer.
    Missing,
}

/// What a remote command did.
pub struct Ran {
    pub status: Status,
    /// Bytes, because some of what comes back this way is a PNG.
    pub stdout: Vec<u8>,
    /// Everything the remote said that was not the status line.
    pub said: String,
}

impl Ran {
    pub fn ok(&self) -> bool {
        self.status == Status::Code(0)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }
}

/// `script` on `host`, with the exit code carried back inside stderr rather
/// than read off the ssh process: not every session carries one. A brokered
/// session reports success whatever the command did, which would turn every
/// remote refusal into a silent success here.
pub async fn run(host: &str, script: &str, args: &[&str], limit: Duration) -> Result<Ran> {
    let out = self::script(host, &wrapped(script), args).output();
    let out = tokio::time::timeout(limit, out)
        .await
        .map_err(|_| anyhow!("{host} did not answer in time"))??;

    let (status, said) = outcome(&out.stderr);

    Ok(Ran {
        status,
        stdout: out.stdout,
        said,
    })
}

/// The status line runs after the script, which only happens if the script
/// leaves a shell behind to run it — and a script is free to end in `exit`, or
/// to hand its process to `exec`. A subshell confines both: they end it rather
/// than the shell holding the line, `$?` still reads what it ended with, and
/// stdout still flows through untouched.
///
/// Wrapping rather than documenting the constraint, because a caller that
/// breaks it is not told: a script that got no status back is indistinguishable
/// from a session that dropped, so the failure would read as a dead host.
fn wrapped(script: &str) -> String {
    format!("(\n{script}\n)\nprintf '{STATUS}%s\\n' \"$?\" >&2")
}

/// Splits a remote run's stderr into how the command ended and the reason it
/// printed.
fn outcome(stderr: &[u8]) -> (Status, String) {
    let stderr = String::from_utf8_lossy(stderr);
    let mut status = Status::Missing;
    let mut reason = Vec::new();

    for line in stderr.lines() {
        // a command whose last line of stderr had no newline on it leaves the
        // marker glued to the end of that line, so it is looked for anywhere
        match line.split_once(STATUS) {
            Some((before, value)) => {
                status = match value.trim().parse() {
                    Ok(code) => Status::Code(code),
                    Err(_) => Status::Garbled(value.trim().to_string()),
                };

                if !before.is_empty() {
                    reason.push(before.strip_prefix("phone: ").unwrap_or(before));
                }
            }
            None => reason.push(line.strip_prefix("phone: ").unwrap_or(line)),
        }
    }

    (status, reason.join("\n").trim().to_string())
}

pub async fn output(mut cmd: Command, limit: Duration) -> Result<Vec<u8>> {
    let out = tokio::time::timeout(limit, cmd.stderr(Stdio::null()).output())
        .await
        .map_err(|_| anyhow!("ssh timed out"))??;

    Ok(out.stdout)
}

/// Empty string for anything that failed: discovery calls this per host and one
/// unreachable machine must not fail the survey.
pub async fn text(host: &str, remote: &str, limit: Duration) -> String {
    let mut cmd = command(host);
    cmd.arg(remote);

    match output(cmd, limit).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// A forward from `local` to `remote_port` on the host's loopback, reusing one
/// already up. `-f -N` daemonises so it outlives this process — every CLI
/// invocation is a fresh one, and a tunnel torn down on exit is rebuilt on every
/// command.
pub async fn forward(host: &str, local: u16, remote_port: u16) -> Result<()> {
    if sweep::probe("127.0.0.1", local, Duration::from_millis(400)).await {
        return Ok(());
    }

    let status = tokio::time::timeout(
        Duration::from_secs(15),
        command(host)
            .args(["-o", "ExitOnForwardFailure=yes"])
            .args(["-f", "-N", "-L"])
            .arg(format!("127.0.0.1:{local}:127.0.0.1:{remote_port}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| anyhow!("{host}: forward timed out"))??;

    if !status.success() {
        bail!("{host}: could not forward port {remote_port}");
    }

    Ok(())
}

/// A hint rather than a reservation: the bind is released immediately, so
/// callers keep the number and re-pick if it stops working.
pub async fn free_port() -> Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    drop(listener);

    Ok(port)
}

/// What whoever asked for this run calls the machine it is running on.
///
/// Unset for a command typed here, which is the ordinary case: a person reading
/// their own terminal wants "this machine", not their own hostname. A run handed
/// over by another machine sets it, so that every line it streams back names the
/// host the reader is not sitting at.
static CALLED: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Set once, by the side receiving a handed-over run, out of the name the
/// manifest arrived with. Later calls are ignored rather than an error: one run
/// is handed over once, and a second name would mean the first was wrong.
pub fn called(name: &str) {
    if !name.is_empty() {
        let _ = CALLED.set(name.to_string());
    }
}

/// A machine to run a shell command on. `Here` is not a host name: this process
/// already has a shell on this machine, and reaching it through ssh would need a
/// loopback config nothing else in the tool depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Where {
    #[default]
    Here,
    On(String),
}

impl Where {
    pub fn of(host: Option<&str>) -> Self {
        match host.filter(|h| !h.is_empty()) {
            Some(host) => Where::On(host.to_string()),
            None => Where::Here,
        }
    }

    pub fn host(&self) -> Option<&str> {
        match self {
            Where::Here => None,
            Where::On(host) => Some(host),
        }
    }

    /// What to call this machine in a line somebody reads. Not `host()`: that
    /// answers where to run something, and `Here` is the right answer to that
    /// even on a run whose output is read a network away.
    pub fn label(&self) -> &str {
        self.host()
            .unwrap_or_else(|| CALLED.get().map_or("this machine", String::as_str))
    }

    /// `script` with `args` bound to `$1`, `$2`, … either way, so a caller writes
    /// one script and never interpolates a name into it.
    pub fn run(&self, script: &str, args: &[&str]) -> Command {
        match self {
            Where::Here => {
                let mut cmd = Command::new("sh");

                cmd.arg("-c").arg(script).arg("sh").args(args);
                cmd.stdin(Stdio::null());

                cmd
            }
            Where::On(host) => self::script(host, script, args),
        }
    }

    pub async fn text(&self, script: &str, args: &[&str], limit: Duration) -> String {
        match output(self.run(script, args), limit).await {
            Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
            Err(_) => String::new(),
        }
    }

    /// `script`, with everything it printed kept. The local half reads the exit
    /// code off the process because there is one to read; the remote half reads
    /// it off the stream, for the reason `run` gives.
    pub async fn exec(&self, script: &str, args: &[&str], limit: Duration) -> Result<Ran> {
        let Where::Here = self else {
            return run(self.host().expect("not Here"), script, args, limit).await;
        };

        let out = tokio::time::timeout(limit, self.run(script, args).output())
            .await
            .map_err(|_| anyhow!("the command did not finish in {}s", limit.as_secs()))??;

        Ok(Ran {
            status: match out.status.code() {
                Some(code) => Status::Code(code),
                None => Status::Missing,
            },
            stdout: out.stdout,
            said: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }

    /// `script` with its output going straight to this terminal as it arrives.
    ///
    /// For the long ones — a dependency install, a gradle build — where a
    /// captured run means minutes of silence and then a wall of text. The exit
    /// code still comes off the stream rather than off ssh, so the status line
    /// has to be picked back out of the stderr being forwarded.
    /// `tag`, when there is one, is written in front of every line the command
    /// produces. Nothing needs it while one command runs at a time; two of them
    /// streaming into the same terminal do, because a gradle line and an
    /// xcodebuild line are otherwise indistinguishable. It costs a pipe on
    /// stdout, which is why the untagged case keeps inheriting it and whatever
    /// redraws its own progress there still can.
    pub async fn stream(&self, script: &str, args: &[&str], tag: Option<&str>) -> Result<Status> {
        let wrapped = wrapped(script);

        let mut cmd = match self {
            Where::Here => {
                let mut cmd = Command::new("sh");

                cmd.arg("-c").arg(&wrapped).arg("sh").args(args);
                cmd.stdin(Stdio::null());

                cmd
            }
            Where::On(host) => self::script(host, &wrapped, args),
        };

        let mut child = cmd
            .stdout(match tag {
                Some(_) => Stdio::piped(),
                None => Stdio::inherit(),
            })
            .stderr(Stdio::piped())
            .spawn()
            .context("starting the command")?;

        // its own task rather than a second arm of the loop below: a command
        // whose stdout pipe fills up while nothing is draining it stops dead
        let forwarding = child.stdout.take().map(|out| {
            let tag = tag.unwrap_or_default().to_string();

            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(out).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    println!("{tag}{line}");
                }
            })
        });

        let tag = tag.unwrap_or_default();
        let piped = child.stderr.take().expect("stderr is piped");
        let mut lines = tokio::io::BufReader::new(piped).lines();
        let mut status = Status::Missing;

        while let Some(line) = lines.next_line().await? {
            let Some((before, value)) = line.split_once(STATUS) else {
                eprintln!("{tag}{line}");
                continue;
            };

            status = match value.trim().parse() {
                Ok(code) => Status::Code(code),
                Err(_) => Status::Garbled(value.trim().to_string()),
            };

            if !before.is_empty() {
                eprintln!("{tag}{before}");
            }
        }

        child.wait().await?;

        if let Some(forwarding) = forwarding {
            forwarding.await.ok();
        }

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(script: &str) -> (Status, String, String) {
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(wrapped(script))
            .output()
            .expect("running sh");

        let (status, said) = outcome(&out.stderr);

        (
            status,
            String::from_utf8_lossy(&out.stdout).trim().to_string(),
            said,
        )
    }

    #[test]
    fn reads_the_remote_status_off_the_stream_that_carries_it() {
        assert_eq!(
            outcome(b"phone-status:0\n"),
            (Status::Code(0), String::new())
        );
        assert_eq!(
            outcome(b"phone: unknown key: wiggle\nphone-status:1\n"),
            (Status::Code(1), "unknown key: wiggle".to_string())
        );
    }

    /// A session that never reached the command leaves no status behind, and
    /// reporting that as a clean run would call every failure a success.
    #[test]
    fn tells_a_missing_status_apart_from_a_zero_one() {
        assert_eq!(outcome(b""), (Status::Missing, String::new()));
        assert_eq!(
            outcome(b"ssh: connect: host is down\n"),
            (Status::Missing, "ssh: connect: host is down".to_string())
        );
    }

    /// A command that ran and lost its outcome is not a session that never got
    /// there: only one of the two is worth trying again.
    #[test]
    fn tells_a_status_that_is_not_a_number_apart_from_no_status() {
        assert_eq!(
            outcome(b"phone-status:killed\n").0,
            Status::Garbled("killed".to_string())
        );
    }

    /// stderr with no newline on its last line arrives with the marker stuck to
    /// the end of it, and reading the status only at the start of a line would
    /// call that a dropped session.
    #[test]
    fn finds_the_status_behind_a_line_that_was_never_terminated() {
        assert_eq!(
            outcome(b"Password:phone-status:1\n"),
            (Status::Code(1), "Password:".to_string())
        );
    }

    /// The whole point of the subshell, and it only holds if a real shell
    /// behaves the way it is supposed to — so this runs one.
    #[test]
    fn a_script_that_ends_by_leaving_the_shell_still_reports_its_status() {
        assert_eq!(
            ran("echo out; exit 3"),
            (Status::Code(3), "out".into(), String::new())
        );
        assert_eq!(
            ran("exit 0"),
            (Status::Code(0), String::new(), String::new())
        );
    }

    /// `exec` replaces the shell with the command, which would take the status
    /// line with it. Inside a subshell it replaces only the subshell.
    #[test]
    fn a_script_that_hands_its_process_away_still_reports_its_status() {
        assert_eq!(
            ran("exec printf hi"),
            (Status::Code(0), "hi".into(), String::new())
        );
        assert_eq!(ran("exec false").0, Status::Code(1));
    }
}
