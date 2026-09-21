use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use outage::config::{Config, DEFAULT_CONFIG};
use outage::control::{self, Request};
use outage::daemon;

#[derive(Parser)]
#[command(
    name = "outage",
    about = "one-shot power-outage protocol",
    long_about = "Arms a one-shot protocol that, during a blackout, terminates the desktop \
session and sleeps between internet checks. Starts disarmed. Entry does not end it - that is \
where it starts doing its job. The internet coming back, a disarm or a reboot return it to \
disarmed, and it never re-arms itself."
)]
struct Cli {
    #[arg(long, default_value = DEFAULT_CONFIG, global = true)]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Watch for an outage. One shot: it does not re-arm itself")]
    Arm,
    #[command(
        about = "Stop watching, or end a running protocol. Clears the wake alarm, except where a suspend is already queued and the alarm is the only way back"
    )]
    Disarm,
    #[command(about = "Report the current phase")]
    Status,
    #[command(about = "Run the controller. Started by the system unit, not by hand")]
    Daemon,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = match Config::load(&cli.config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("outage: {}: {err}", cli.config.display());
            return ExitCode::FAILURE;
        }
    };

    match cli.command {
        Command::Daemon => match daemon::run(config) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("outage: {err}");
                ExitCode::FAILURE
            }
        },
        Command::Arm => talk(&config, Request::Arm),
        Command::Disarm => talk(&config, Request::Disarm),
        Command::Status => talk(&config, Request::Status),
    }
}

fn talk(config: &Config, request: Request) -> ExitCode {
    match control::request(&config.socket, request, config.control_timeout()) {
        Ok(reply) => {
            println!("{}: {}", reply.phase, reply.detail);
            if reply.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!(
                "outage: cannot reach the controller at {}: {err}",
                config.socket.display()
            );
            ExitCode::FAILURE
        }
    }
}
