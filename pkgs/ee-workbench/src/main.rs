mod bridge;
mod cli;
mod cmd;
mod git;
mod ids;
mod model;
mod paths;
mod spawn;
mod store;
mod tui;

use std::ffi::c_int;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use store::Workbench;

/// Rust ignores SIGPIPE before `main`, so writing into a closed pipe returns
/// EPIPE and `println!` panics on it. `ee mechanical document inspect | head -1`
/// then prints a backtrace and exits 1, which to a caller is indistinguishable
/// from the command having failed. Restoring the default disposition makes the
/// process die on the signal the way every other tool in a pipeline does. It
/// covers `--json` too: `cmd::emit_json` is a `println!` as well, so one
/// disposition is both output paths.
///
/// Only ee's own output needs this. Children are already covered:
/// `std::process::Command` resets SIGPIPE to `SIG_DFL` between fork and exec,
/// because an ignored disposition is one of the few that survive `execve`.
///
/// Declared here rather than pulled from libc, matching the `flock` and
/// `setsid` bindings in `spawn`: `sighandler_t` is `size_t` on every platform
/// this builds for, and SIGPIPE is 13 on all of them.
const SIGPIPE: c_int = 13;
const SIG_DFL: usize = 0;

unsafe extern "C" {
    fn signal(signum: c_int, handler: usize) -> usize;
}

fn main() {
    unsafe { signal(SIGPIPE, SIG_DFL) };

    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("ee: {error:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    // No subcommand is the human path: the TUI is the default interface, and
    // the subcommands below are what agents and scripts drive.
    let Some(command) = cli.command else {
        return tui::run(Workbench::open()?);
    };

    match command {
        Command::Project { command } => cmd::project::run(command),
        Command::Inventory { command } => cmd::inventory::run(command),
        Command::Experiment { command } => cmd::experiment::run(command),
        Command::Measurement { command } => cmd::measurement::run(command),
        Command::Mechanical { command } => cmd::mechanical::run(command),
        Command::Repo { command } => cmd::repo::run(command),
        Command::Git { args } => git::passthrough(&paths::data_root(), &args),
    }
}
