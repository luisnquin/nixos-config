pub mod config;
pub mod control;
pub mod daemon;
pub mod driver;
pub mod effects;
pub mod machine;
pub mod runner;
pub mod sense;

use std::io::Write;

pub fn log(message: &str) {
    let mut err = std::io::stderr();
    let _ = writeln!(err, "outage: {message}");
}
