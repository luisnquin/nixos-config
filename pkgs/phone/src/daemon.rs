use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::lease::{self, Book, Lock};
use crate::model::{self, Device, Platform, Unix};
use crate::registry;
use crate::ssh::{landed, Where};
use crate::{avd, simctl, stamps};

mod lifecycle;
mod memory;
pub use lifecycle::{ensure_used, start_due, used};

const OWNER: &str = "phone automatic cleanup";

#[derive(Serialize, Deserialize)]
struct Temporary {
    id: String,
    created: Unix,
}

fn directory() -> PathBuf {
    registry::state_dir().join("temporary")
}

fn path(id: &str) -> PathBuf {
    directory().join(format!("{}.json", stamps::hash(id.as_bytes())))
}

pub async fn track(at: &Where, id: &str) -> Result<()> {
    let body = serde_json::to_string(&Temporary {
        id: id.into(),
        created: model::now(),
    })?;
    let name = format!("{}.json", stamps::hash(id.as_bytes()));
    let script = r#"dir="${XDG_STATE_HOME:-$HOME/.local/state}/phone/temporary"
mkdir -p "$dir" || exit 1
tmp="$dir/$1.$$.tmp"
printf '%s' "$2" > "$tmp" && mv "$tmp" "$dir/$1""#;

    landed(
        at,
        "recording the temporary device",
        at.exec(script, &[&name, &body], Duration::from_secs(20))
            .await?,
    )
}

fn due(created: Unix, used: Option<Unix>, now: Unix, idle: Duration) -> bool {
    now.saturating_sub(created.max(used.unwrap_or(created))) >= idle.as_secs() as i64
}

pub async fn run(
    idle: Duration,
    interval: Duration,
    keep: &[String],
    adopt: &[String],
    timeout: Duration,
    dry_run: bool,
    scheduled: bool,
) -> Result<()> {
    if idle.is_zero() || interval.is_zero() {
        bail!("--idle and --interval must be greater than zero");
    }
    let _singleton = lifecycle::singleton()?;
    if scheduled && !lifecycle::scheduled() {
        return Ok(());
    }
    lifecycle::deadline(timeout)?;
    if !dry_run {
        lifecycle::cooldown(interval)?;
        release(None).await?;
    }
    let mut keep = keep.to_vec();
    keep.extend(crate::config::get().pools.values().flatten().cloned());
    eprintln!(
        "phone: cleanup pass started, max runtime {}s{}",
        timeout.as_secs(),
        if dry_run { ", preview only" } else { "" }
    );
    sweep(idle, &keep, adopt, dry_run).await
}

fn records() -> Result<Vec<Temporary>> {
    let entries = match fs::read_dir(directory()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut records = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            records.push(read_record(&entry.path())?);
        }
    }
    Ok(records)
}

fn pending(idle: Duration) -> Result<Vec<Temporary>> {
    Ok(records()?
        .into_iter()
        .filter(|record| due(record.created, None, model::now(), idle))
        .collect())
}

fn has_pending(idle: Duration) -> Result<bool> {
    Ok(!pending(idle)?.is_empty())
}

async fn inventory(records: &[Temporary], adopt: &[String]) -> Vec<(Device, bool)> {
    let sims = records
        .iter()
        .any(|record| !record.id.starts_with(model::AVD_PREFIX));
    let avds = records
        .iter()
        .any(|record| record.id.starts_with(model::AVD_PREFIX));
    let mut devices = Vec::new();
    if sims || !adopt.is_empty() {
        devices.extend(simctl::devices(&Where::Here).await.unwrap_or_default());
    }
    if avds || !adopt.is_empty() {
        for (name, _) in avd::list(&Where::Here).await {
            devices.push((
                Device::new(model::avd_id(None, &name), name, Platform::Emulator),
                false,
            ));
        }
    }
    devices
}

async fn sweep(idle: Duration, keep: &[String], adopt: &[String], dry_run: bool) -> Result<()> {
    let records = pending(idle)?;
    if records.is_empty() && adopt.is_empty() {
        return Ok(());
    }
    let devices = inventory(&records, adopt).await;
    let mut book = Book::fetch(&Where::Here).await?;
    if !dry_run {
        book = prune(book).await?;
        adopt_existing(&devices, adopt).await?;
    }
    let mut deleted = 0;
    for temporary in records {
        let Some((device, running)) = devices
            .iter()
            .find(|(device, _)| lease::key(device).id == temporary.id)
        else {
            continue;
        };
        if consider(&temporary, device, *running, &book, idle, keep, dry_run).await? {
            deleted += 1;
            if deleted == 3 {
                break;
            }
        }
    }
    Ok(())
}

async fn prune(book: Book) -> Result<Book> {
    let mut ledger = book.ledger.clone();
    ledger
        .leases
        .retain(|_, lease| book.judge(model::now()).valid(lease, true));
    if ledger == book.ledger {
        return Ok(book);
    }
    Ok(match book.commit(&ledger, &[]).await? {
        Ok(newer) | Err(newer) => newer,
    })
}

async fn adopt_existing(devices: &[(Device, bool)], names: &[String]) -> Result<()> {
    for name in names {
        let Some((device, _)) = devices.iter().find(|(device, _)| device.is(name)) else {
            continue;
        };
        let id = lease::key(device).id;
        if !path(&id).exists() {
            track(&Where::Here, &id).await?;
        }
    }
    Ok(())
}

fn read_record(file: &std::path::Path) -> Result<Temporary> {
    let temporary: Temporary = serde_json::from_slice(&fs::read(file)?)
        .with_context(|| format!("reading {}", file.display()))?;
    if path(&temporary.id) != file {
        bail!("temporary device record has a mismatched id");
    }
    Ok(temporary)
}

async fn eligible(
    temporary: &Temporary,
    device: &Device,
    running: bool,
    book: &Book,
    idle: Duration,
    keep: &[String],
) -> Result<bool> {
    let last = book.usage.of(device, None).all.map(|used| used.at);
    if running
        || keep.iter().any(|name| device.is(name))
        || !due(temporary.created, last, model::now(), idle)
    {
        return Ok(false);
    }
    Ok(device.platform != Platform::Emulator || !avd::in_use(&device.label).await?)
}

async fn consider(
    temporary: &Temporary,
    device: &Device,
    running: bool,
    book: &Book,
    idle: Duration,
    keep: &[String],
    dry_run: bool,
) -> Result<bool> {
    if !eligible(temporary, device, running, book, idle, keep).await? {
        return Ok(false);
    }
    if dry_run {
        if !blocked(book, device) {
            eprintln!(
                "phone: would delete {} after {}h without use",
                device.label,
                idle.as_secs() / 3600
            );
        }
        return Ok(false);
    }
    if !reserve(device, temporary, idle).await? {
        return Ok(false);
    }
    let deleted = retire(device).await;
    release(Some(&temporary.id)).await?;
    match deleted {
        Ok(true) => {
            fs::remove_file(path(&temporary.id))?;
            eprintln!(
                "phone: deleted temporary {} at {}",
                device.label,
                model::now()
            );
            return Ok(true);
        }
        Ok(false) => {}
        Err(error) => eprintln!("phone: could not retire {}: {error:#}", device.label),
    }
    Ok(false)
}

fn blocked(book: &Book, device: &Device) -> bool {
    let key = lease::key(device);
    book.ledger.locks.contains_key(&key.id)
        || book
            .ledger
            .get(&key)
            .is_some_and(|(_, lease)| book.judge(model::now()).valid(lease, true))
}

async fn reserve(device: &Device, temporary: &Temporary, idle: Duration) -> Result<bool> {
    for _ in 0..6 {
        let book = Book::fetch(&Where::Here).await?;
        let last = book.usage.of(device, None).all.map(|used| used.at);
        if blocked(&book, device) || !due(temporary.created, last, model::now(), idle) {
            return Ok(false);
        }
        let mut ledger = book.ledger.clone();
        ledger.locks.insert(
            temporary.id.clone(),
            Lock {
                by: OWNER.into(),
                host: book.host.clone(),
                since: model::now(),
            },
        );
        if book.commit(&ledger, &[]).await?.is_ok() {
            return Ok(true);
        }
    }
    bail!("leases kept changing during cleanup");
}

async fn release(id: Option<&str>) -> Result<()> {
    for _ in 0..6 {
        let book = Book::fetch(&Where::Here).await?;
        let mut ledger = book.ledger.clone();
        ledger.locks.retain(|key, lock| {
            lock.by != OWNER || lock.host != book.host || id.is_some_and(|id| key != id)
        });
        if ledger == book.ledger || book.commit(&ledger, &[]).await?.is_ok() {
            return Ok(());
        }
    }
    bail!("leases kept changing while releasing cleanup reservations");
}

async fn retire(device: &Device) -> Result<bool> {
    match device.platform {
        Platform::Emulator => {
            if avd::in_use(&device.label).await? {
                return Ok(false);
            }
            avd::delete(&Where::Here, &device.label).await?;
        }
        Platform::Simulator => {
            let devices = simctl::devices(&Where::Here)
                .await
                .context("could not verify simulator state")?;
            let Some((_, running)) = devices.iter().find(|(current, _)| current.id == device.id)
            else {
                return Ok(false);
            };
            if *running {
                return Ok(false);
            }
            simctl::delete(&Where::Here, &device.id).await?;
        }
        _ => bail!("cleanup only accepts virtual devices"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_use_and_future_timestamps_delay_retirement() {
        let idle = Duration::from_secs(100);
        assert!(!due(0, Some(151), 200, idle));
        assert!(!due(201, None, 200, idle));
        assert!(!due(151, None, 200, idle));
        assert!(due(0, Some(100), 200, idle));
    }
}
