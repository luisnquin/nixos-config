mod collect;
mod model;
mod report;
mod store;
mod surface;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

use collect::Config;
use util::human;

#[derive(Parser)]
#[command(name = "heft", about = "where the disk space went, and what changed")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run every collector, write a snapshot and refresh the rollup.
    Scan {
        /// Also measure the nix store's hardlink dedup ratio. Walks the whole
        /// store, so it takes minutes rather than seconds.
        #[arg(long)]
        deep: bool,
        /// Suppress the report that a scan normally prints when it finishes.
        #[arg(long)]
        quiet: bool,
    },
    /// Print the diagnostic for the newest snapshot.
    Report {
        /// A date (2026-08-13), an offset from newest (-1), or a file path.
        #[arg(long)]
        snapshot: Option<String>,
    },
    /// Emit Waybar JSON from the cached rollup. Never scans.
    Waybar {
        /// Free space below which the module turns amber, in GiB.
        #[arg(long, default_value_t = 100)]
        warn_free: u64,
        /// Free space below which the module turns red, in GiB.
        #[arg(long, default_value_t = 50)]
        critical_free: u64,
    },
    /// Emit the full rollup as JSON for the eww panel. Never scans.
    Eww,
    /// Compare two snapshots.
    Diff {
        before: String,
        #[arg(default_value = "-0")]
        after: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Scan { deep, quiet } => scan(deep, quiet),
        Command::Report { snapshot } => print_report(snapshot),
        Command::Waybar {
            warn_free,
            critical_free,
        } => waybar(warn_free, critical_free),
        Command::Eww => eww(),
        Command::Diff { before, after } => diff(&before, &after),
    }
}

fn scan(deep: bool, quiet: bool) -> Result<()> {
    let config = Config::default();
    let carried = store::latest_snapshot().and_then(|s| s.nix_dedup_ratio);

    let snapshot = collect::census(&config, carried, deep)?;

    // The snapshot that was newest before this scan is what a delta compares
    // against — read it before saving, since same-day scans overwrite in place.
    let baseline = store::latest_snapshot().filter(|s| s.taken_at < snapshot.taken_at);

    store::save_snapshot(&snapshot)?;
    let rollup = store::build_rollup(&snapshot, baseline.as_ref());
    let path = store::save_rollup(&rollup)?;

    if quiet {
        return Ok(());
    }

    print!("{}", report::render(&snapshot, baseline.as_ref()));
    eprintln!("  rollup written to {}", path.display());
    if !deep && snapshot.nix_dedup_ratio.is_none() {
        eprintln!("  run `heft scan --deep` once to calibrate nix store sizes");
    }
    Ok(())
}

fn print_report(reference: Option<String>) -> Result<()> {
    let snapshot = match reference {
        Some(reference) => store::resolve(&reference)?,
        None => store::latest_snapshot()
            .ok_or_else(|| anyhow::anyhow!("no snapshots yet — run `heft scan`"))?,
    };
    let previous = store::previous_snapshot();
    print!("{}", report::render(&snapshot, previous.as_ref()));
    Ok(())
}

fn waybar(warn_free: u64, critical_free: u64) -> Result<()> {
    let gib = 1024 * 1024 * 1024;
    let thresholds = surface::Thresholds {
        warn_free: warn_free * gib,
        critical_free: critical_free * gib,
    };

    // A missing rollup is the normal state before the first scan. The bar must
    // still render something rather than leaving a gap.
    let output = match store::read_rollup() {
        Ok(rollup) => surface::waybar(&rollup, &thresholds),
        Err(_) => surface::WaybarOutput {
            text: "󰋊 …".into(),
            tooltip: "heft has not scanned yet".into(),
            class: "loading".into(),
            percentage: 0,
        },
    };

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

fn eww() -> Result<()> {
    match store::read_rollup() {
        Ok(rollup) => println!("{}", serde_json::to_string(&rollup)?),
        Err(_) => println!(
            "{}",
            serde_json::json!({
                "updated_label": "waiting for first scan",
                "filesystem": {"total": 0, "used": 0, "free": 0},
                "pct_used": 0.0,
                "domains": [],
                "movers": [],
                "reclaimable": 0,
                "actions": [],
                "newcomers": [],
            })
        ),
    }
    Ok(())
}

fn diff(before: &str, after: &str) -> Result<()> {
    let before = store::resolve(before)?;
    let after = store::resolve(after)?;
    print!("{}", report::render_diff(&before, &after));

    let delta = after.filesystem.used as i64 - before.filesystem.used as i64;
    if delta > 0 {
        eprintln!("  grew by {}", human(delta as u64));
    }
    Ok(())
}
