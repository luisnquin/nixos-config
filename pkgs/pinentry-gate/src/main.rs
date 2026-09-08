//! A pinentry that asks on every surface at once.
//!
//! gpg-agent execs one pinentry and speaks Assuan to it on stdin/stdout. This
//! one fans a request out: a modal on the computer (a fullscreen terminal
//! window on the active graphical session, or a reserved virtual console
//! otherwise) and an escape sequence into every terminal that marked its pty.
//! The first decision wins and the others withdraw.

mod assuan;
mod broker;
mod config;
mod modal;
mod phone;
mod seat;
mod signals;

use std::io;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("modal") {
        std::process::exit(modal::main(&args[1..]));
    }
    // Anything else on the command line is gpg-agent's habit of passing
    // --display and friends; every decision arrives over Assuan.
    signals::install();
    let mut broker = broker::Broker::new(config::Config::load());
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut server = assuan::Server::new(|request| broker.ask(request), stdin.lock(), stdout.lock());
    if server.serve().is_err() {
        std::process::exit(1);
    }
}
