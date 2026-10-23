use std::fs::{self, File};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::lease::{self, Book, Lock};
use crate::model::{self, Device, Platform, Unix};
use crate::registry;
use crate::ssh::{landed, Where};
use crate::{avd, simctl, stamps};

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
    once: bool,
    dry_run: bool,
) -> Result<()> {
    if idle.is_zero() || interval.is_zero() {
        bail!("--idle and --interval must be greater than zero");
    }

    fs::create_dir_all(registry::state_dir())?;
    let singleton = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(registry::state_dir().join("daemon.lock"))?;
    singleton
        .try_lock()
        .context("another phone daemon is already running for this state directory")?;

    if !dry_run {
        release(None).await?;
    }
    eprintln!(
        "phone: maintenance started, idle {}h, interval {}s{}",
        idle.as_secs() / 3600,
        interval.as_secs(),
        if dry_run { ", preview only" } else { "" }
    );

    loop {
        if let Err(error) = sweep(idle, keep, adopt, dry_run).await {
            if once {
                return Err(error);
            }
            eprintln!("phone: cleanup failed: {error:#}");
        }
        if once {
            return Ok(());
        }
        tokio::time::sleep(interval).await;
    }
}

async fn inventory() -> Vec<(Device, bool)> {
    let mut devices = simctl::devices(&Where::Here).await.unwrap_or_default();
    for (name, _) in avd::list(&Where::Here).await {
        devices.push((
            Device::new(model::avd_id(None, &name), name, Platform::Emulator),
            false,
        ));
    }
    devices
}

async fn sweep(idle: Duration, keep: &[String], adopt: &[String], dry_run: bool) -> Result<()> {
    let devices = inventory().await;
    let mut book = Book::fetch(&Where::Here).await?;
    if !dry_run {
        book = prune(book).await?;
        adopt_existing(&devices, adopt).await?;
    }

    let entries = match fs::read_dir(directory()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        if entry.path().extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let temporary = read_record(&entry.path())?;
        let Some((device, running)) = devices
            .iter()
            .find(|(device, _)| lease::key(device).id == temporary.id)
        else {
            continue;
        };
        consider(&temporary, device, *running, &book, idle, keep, dry_run).await?;
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
) -> Result<()> {
    if !eligible(temporary, device, running, book, idle, keep).await? {
        return Ok(());
    }
    if dry_run {
        if !blocked(book, device) {
            eprintln!(
                "phone: would delete {} after {}h without use",
                device.label,
                idle.as_secs() / 3600
            );
        }
        return Ok(());
    }
    if !reserve(device, temporary, idle).await? {
        return Ok(());
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
        }
        Ok(false) => {}
        Err(error) => eprintln!("phone: could not retire {}: {error:#}", device.label),
    }
    Ok(())
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
