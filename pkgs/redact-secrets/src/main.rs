mod read_hook;
mod redact;

use std::env;
use std::ffi::OsString;
use std::io;
use std::os::fd::AsFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;

const USAGE: &str = "usage: redact-secrets [<command> | --read-hook]
  <command>     run it with the calling shell, secrets masked out of stdout and stderr
  --read-hook   Claude Code PreToolUse hook that denies Read on secret files
  (no args)     filter stdin to stdout";

fn shell() -> PathBuf {
    let parent = format!("/proc/{}/exe", std::os::unix::process::parent_id());
    if let Ok(exe) = std::fs::read_link(parent) {
        if exe
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with("sh"))
        {
            return exe;
        }
    }
    ["CLAUDE_CODE_SHELL", "SHELL"]
        .iter()
        .find_map(env::var_os)
        .map_or_else(|| PathBuf::from("/bin/sh"), PathBuf::from)
}

fn forward_signals(child: u32) -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT])?;
    let pid = Pid::from_raw(child as i32);
    thread::spawn(move || {
        for raw in signals.forever() {
            if let Ok(signal) = Signal::try_from(raw) {
                let _ = kill(pid, signal);
            }
        }
    });
    Ok(())
}

fn filter(input: Stdio, output: Stdio) -> io::Result<Child> {
    Command::new(env::current_exe()?)
        .stdin(input)
        .stdout(output)
        .spawn()
}

fn run(command: &OsString) -> io::Result<ExitCode> {
    let mut child = Command::new(shell())
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    forward_signals(child.id())?;

    // Separate processes rather than threads: a backgrounded job that keeps the
    // pipes open must not hold the exit back, and must keep a reader after we leave.
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let mut filters = [
        filter(stdout.into(), Stdio::inherit())?,
        filter(
            stderr.into(),
            io::stderr().as_fd().try_clone_to_owned()?.into(),
        )?,
    ];

    let status = child.wait()?;
    let deadline = Instant::now() + Duration::from_millis(500);
    for f in &mut filters {
        while f.try_wait()?.is_none() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
    }

    let code = status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0));
    Ok(ExitCode::from(code as u8))
}

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [] => match redact::stream(io::stdin().lock(), io::stdout().lock()) {
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(ExitCode::SUCCESS),
            other => other.map(|()| ExitCode::SUCCESS),
        },
        [flag] if flag == "--read-hook" => read_hook::run().map(|()| ExitCode::SUCCESS),
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        [command] => run(command),
        _ => {
            eprintln!("{USAGE}");
            Ok(ExitCode::from(2))
        }
    };
    result.unwrap_or_else(|err| {
        eprintln!("redact-secrets: {err}");
        ExitCode::from(1)
    })
}
