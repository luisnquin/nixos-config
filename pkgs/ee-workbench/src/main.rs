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

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use store::Workbench;

fn main() {
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
