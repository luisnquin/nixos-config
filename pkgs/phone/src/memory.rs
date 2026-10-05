use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::adb::{self, Server};
use crate::lease::{Book, Standing};
use crate::model::{Device, Platform, View};
use crate::registry::Registry;
use crate::ssh::Where;
use crate::{actions, quoted, Refused};

const TIMEOUT: Duration = Duration::from_secs(25);

const GB: f64 = (1u64 << 30) as f64;

const DEFAULT_RAM: f64 = 2.0;

const READ: &str = r#"if [ -r /proc/meminfo ]; then
  echo "os: linux"
  cat /proc/meminfo
else
  echo "os: darwin"
  echo "memsize: $(sysctl -n hw.memsize)"
  echo "pressure: $(sysctl -n kern.memorystatus_vm_pressure_level 2>/dev/null)"
  vm_stat
fi || exit 1
[ $# -gt 0 ] || exit 0
home=$($SHELL -l -c 'printf %s "$ANDROID_AVD_HOME"' 2>/dev/null)
[ -n "$home" ] || home="${ANDROID_AVD_HOME:-${ANDROID_USER_HOME:-$HOME/.android}/avd}"
for avd; do
  dir=$(sed -n 's/^path=//p' "$home/$avd.ini" 2>/dev/null)
  [ -d "$dir" ] || dir="$home/$avd.avd"
  printf 'avd %s %s\n' "$avd" "$(sed -n 's/^hw\.ramSize *= *//p' "$dir/config.ini" 2>/dev/null | head -n 1)"
done
exit 0"#;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub reserve: f64,
    pub emulator_overhead: f64,
    pub simulator: f64,
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            reserve: 6.0,
            emulator_overhead: 0.5,
            simulator: 3.7,
        }
    }
}

impl Budget {
    pub fn label(&self) -> String {
        format!(
            "reserve {:.1} GB, an emulator its hw.ramSize + {:.1} GB, a simulator {:.1} GB",
            self.reserve, self.emulator_overhead, self.simulator
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Normal,
    Warn,
    Critical,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Normal => "normal",
            Level::Warn => "warn",
            Level::Critical => "critical",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memory {
    pub total: u64,
    pub available: u64,
    pub level: Level,
}

fn fields(text: &str) -> BTreeMap<&str, u64> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.rsplit_once(':')?;
            let value = value.split_whitespace().next()?.trim_end_matches('.');

            Some((key.trim().trim_matches('"'), value.parse().ok()?))
        })
        .collect()
}

fn linux(text: &str) -> Option<Memory> {
    let f = fields(text);
    let total = f.get("MemTotal")? * 1024;
    let available = f.get("MemAvailable")? * 1024;

    let level = match available * 100 / total.max(1) {
        0..5 => Level::Critical,
        5..10 => Level::Warn,
        _ => Level::Normal,
    };

    Some(Memory {
        total,
        available,
        level,
    })
}

fn darwin(text: &str) -> Option<Memory> {
    let f = fields(text);
    let page = text
        .split_once("page size of ")
        .and_then(|(_, rest)| rest.split_whitespace().next()?.parse::<u64>().ok())?;

    // file-backed already includes speculative pages and purgeable ones sit
    // inside anonymous memory, so this sum counts no page twice
    let pages = f.get("Pages free")?
        + f.get("File-backed pages")
            .or_else(|| f.get("Pages speculative"))
            .copied()
            .unwrap_or(0)
        + f.get("Pages purgeable").copied().unwrap_or(0);

    let level = match f.get("pressure") {
        Some(4) => Level::Critical,
        Some(2) => Level::Warn,
        _ => Level::Normal,
    };

    Some(Memory {
        total: *f.get("memsize")?,
        available: pages * page,
        level,
    })
}

fn ram_size(value: &str) -> Option<f64> {
    let value = value.trim().to_ascii_uppercase();
    let value = value.strip_suffix('B').unwrap_or(&value);

    let (number, scale) = match value.strip_suffix('G') {
        Some(n) => (n, 1.0),
        None => (value.strip_suffix('M').unwrap_or(value), 1.0 / 1024.0),
    };

    number
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| *n > 0.0)
        .map(|n| n * scale)
}

fn parse(text: &str) -> Option<(Memory, BTreeMap<String, f64>)> {
    let (head, rest) = text.split_once('\n')?;

    let memory = match head.trim() {
        "os: linux" => linux(rest)?,
        "os: darwin" => darwin(rest)?,
        _ => return None,
    };

    let avds = rest
        .lines()
        .filter_map(|line| {
            let mut words = line.strip_prefix("avd ")?.split_whitespace();
            let name = words.next()?;
            let ram = words.next().and_then(ram_size).unwrap_or(DEFAULT_RAM);

            Some((name.to_string(), ram))
        })
        .collect();

    Some((memory, avds))
}

async fn read(at: &Where, avds: &[&str]) -> Result<(Memory, BTreeMap<String, f64>)> {
    let ran = at.exec(READ, avds, TIMEOUT).await?;

    if !ran.ok() {
        bail!("reading memory on {}: {}", at.label(), ran.said);
    }

    match parse(&String::from_utf8_lossy(&ran.stdout)) {
        Some(read) => Ok(read),
        None => bail!("{} printed memory figures this does not read", at.label()),
    }
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / GB
}

#[derive(Clone, Debug)]
pub struct Tenant {
    pub label: String,
    pub gb: f64,
    pub holder: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fit {
    Room(String),
    Full(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct Room {
    pub at: Where,
    pub memory: Memory,
    pub budget: Budget,
    pub tenants: Vec<Tenant>,
    pub emulator: Option<f64>,
    pub simulators: bool,
    ram: BTreeMap<String, f64>,
}

impl Room {
    fn cost(&self, device: &Device) -> f64 {
        match device.platform {
            Platform::Simulator => self.budget.simulator,
            _ => {
                self.ram.get(&device.label).copied().unwrap_or(DEFAULT_RAM)
                    + self.budget.emulator_overhead
            }
        }
    }

    pub fn committed(&self) -> f64 {
        self.tenants.iter().map(|t| t.gb).sum()
    }

    pub fn left(&self) -> f64 {
        gb(self.memory.total) - self.budget.reserve - self.committed()
    }

    fn pressed(&self) -> bool {
        self.memory.level != Level::Normal
    }

    pub fn line(&self) -> String {
        let tenants = match self.tenants.len() {
            0 => "nothing running".to_string(),
            n => format!(
                "{:.1} committed by {n} device{} ({})",
                self.committed(),
                if n == 1 { "" } else { "s" },
                self.tenants
                    .iter()
                    .map(|t| format!(
                        "{} {:.1} {}",
                        t.label,
                        t.gb,
                        match &t.holder {
                            Some(holder) => format!("held by {holder}"),
                            None => "unheld".to_string(),
                        }
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };

        format!(
            "{}: {:.0} GB, {:.1} reserved, {tenants}, {:.1} left; pressure {}, {:.1} GB available",
            self.at.label(),
            gb(self.memory.total),
            self.budget.reserve,
            self.left().max(0.0),
            self.memory.level.as_str(),
            gb(self.memory.available)
        )
    }

    fn idle(&self) -> impl Iterator<Item = String> + '_ {
        self.tenants
            .iter()
            .filter(|t| t.holder.is_none())
            .map(|idle| {
                format!(
                    "{} is running and nobody holds it: use it with -t {} before booting another",
                    idle.label,
                    quoted(&idle.label)
                )
            })
    }

    pub fn fit(&self, device: &Device) -> Fit {
        let need = self.cost(device);

        match self.admits(need) {
            None => Fit::Room(format!(
                "{} has room for it ({:.1} GB left, it needs {need:.1})",
                self.at.label(),
                self.left()
            )),
            Some(why) => Fit::Full(std::iter::once(why).chain(self.idle()).collect()),
        }
    }

    pub fn invite(&self) -> Vec<String> {
        let mut out = self.advice();

        out.extend(self.boots());

        out
    }

    pub fn advice(&self) -> Vec<String> {
        let host = self.at.label();
        let mut out = Vec::new();

        if self.pressed() {
            out.push(format!(
                "{host} is under memory pressure ({}): boot nothing there until it clears; a starved device takes taps and never acts on them",
                self.memory.level.as_str()
            ));
        }

        out.extend(self.idle());

        out
    }

    fn boots(&self) -> Option<String> {
        let host = self.at.label();

        if self.pressed() {
            return None;
        }

        let fits: Vec<(usize, String)> = self
            .fits()
            .into_iter()
            .map(|(platform, n)| match platform {
                Platform::Simulator => count(n, "simulator"),
                _ => count(n, "emulator"),
            })
            .collect();

        match fits.iter().any(|(n, _)| *n > 0) {
            true => Some(format!(
                "you may boot {} on {host}",
                fits.iter()
                    .filter(|(n, _)| *n > 0)
                    .map(|(_, said)| said.as_str())
                    .collect::<Vec<_>>()
                    .join(" or ")
            )),
            false if !fits.is_empty() => Some(format!(
                "no room on {host} for another device: wait for a `phone down`, or ask a holder above to release theirs"
            )),
            false => None,
        }
    }

    fn fits(&self) -> Vec<(Platform, usize)> {
        let left = self.left().max(0.0);
        let mut fits = Vec::new();

        if let Some(cost) = self.emulator {
            fits.push((Platform::Emulator, (left / cost) as usize));
        }

        if self.simulators {
            fits.push((Platform::Simulator, (left / self.budget.simulator) as usize));
        }

        fits
    }

    pub fn room_for(&self, platform: Platform) -> Option<usize> {
        let n = self.fits().into_iter().find(|(p, _)| *p == platform)?.1;

        Some(if self.pressed() { 0 } else { n })
    }

    pub fn brief(&self) -> String {
        if self.pressed() {
            return format!(
                "memory pressure {}, boot nothing",
                self.memory.level.as_str()
            );
        }

        let fits: Vec<String> = self
            .fits()
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(platform, n)| format!("{n} {}", platform.as_str()))
            .collect();

        match fits.is_empty() {
            true => "no room to boot".to_string(),
            false => format!("room to boot {}", fits.join(" or ")),
        }
    }

    fn admits(&self, need: f64) -> Option<String> {
        if self.pressed() {
            return Some(format!(
                "{} is under memory pressure ({})",
                self.at.label(),
                self.memory.level.as_str()
            ));
        }

        (need > self.left()).then(|| {
            format!(
                "no room on {}: this needs {need:.1} GB and {:.1} is left",
                self.at.label(),
                self.left().max(0.0)
            )
        })
    }
}

fn count(n: usize, what: &str) -> (usize, String) {
    let more = match n {
        1 => format!("1 more {what}"),
        n => format!("{n} more {what}s"),
    };

    (n, more)
}

pub fn budget(reg: &Registry, at: &Where) -> Budget {
    at.host()
        .and_then(|host| reg.hosts.iter().find(|h| h.name == host))
        .and_then(|h| h.budget)
        .unwrap_or_default()
}

fn hosted(view: &View) -> bool {
    matches!(
        view.device.platform,
        Platform::Emulator | Platform::Simulator
    )
}

pub async fn rooms(reg: &Registry, views: &[View], hosts: &[Where]) -> Vec<(Where, Result<Room>)> {
    let reading = hosts.iter().map(|at| async move {
        let mine: Vec<&View> = views
            .iter()
            .filter(|v| hosted(v) && actions::where_of(&v.device) == *at)
            .collect();

        (at.clone(), room(budget(reg, at), at, &mine).await)
    });

    futures_util::future::join_all(reading).await
}

pub fn hosts_of<'a>(views: impl IntoIterator<Item = &'a View>) -> Vec<Where> {
    let mut out: Vec<Where> = Vec::new();

    for view in views.into_iter().filter(|v| hosted(v)) {
        let at = actions::where_of(&view.device);

        if !out.contains(&at) {
            out.push(at);
        }
    }

    out
}

async fn room(budget: Budget, at: &Where, views: &[&View]) -> Result<Room> {
    let avds: Vec<&str> = views
        .iter()
        .filter(|v| v.device.platform == Platform::Emulator)
        .map(|v| v.device.label.as_str())
        .collect();

    let (read, book) = tokio::join!(read(at, &avds), Book::fetch(at));
    let (memory, ram) = read?;
    let book = book.ok();

    let mut room = Room {
        at: at.clone(),
        memory,
        budget,
        tenants: Vec::new(),
        emulator: None,
        simulators: false,
        ram,
    };

    for view in views {
        let device = &view.device;

        match (actions::running(&view.reach), device.platform) {
            (true, _) => room.tenants.push(Tenant {
                label: device.label.clone(),
                gb: room.cost(device),
                holder: book.as_ref().and_then(|b| held(b, device)),
            }),
            (false, Platform::Simulator) => room.simulators = true,
            (false, _) => {
                let cost = room.cost(device);

                room.emulator = Some(room.emulator.map_or(cost, |had| had.max(cost)));
            }
        }
    }

    Ok(room)
}

fn held(book: &Book, device: &Device) -> Option<String> {
    match book.standing(device, true) {
        Standing::Mine(lease) | Standing::Held(lease) => Some(lease.describe(crate::model::now())),
        Standing::Free(_) => None,
    }
}

pub async fn report(reg: &Registry, views: &[View], hosts: &[Where]) -> Vec<String> {
    let rooms = rooms(reg, views, hosts).await;

    lines(&rooms, Room::invite)
}

pub fn lines(rooms: &[(Where, Result<Room>)], after: fn(&Room) -> Vec<String>) -> Vec<String> {
    let mut out = Vec::new();

    for (at, room) in rooms {
        match room {
            Ok(room) => {
                out.push(room.line());
                out.extend(after(room));
            }
            Err(e) => out.push(format!("{}: memory unread ({e:#})", at.label())),
        }
    }

    out
}

pub async fn fit(reg: &Registry, views: &[View], device: &Device) -> Option<Fit> {
    let at = actions::where_of(device);
    let (_, room) = rooms(reg, views, &[at]).await.pop()?;

    room.ok().map(|room| room.fit(device))
}

pub async fn admit(reg: &Registry, views: &[View], booting: &[&Device], over: bool) -> Result<()> {
    let hosts = hosts_of(
        views
            .iter()
            .filter(|v| booting.iter().any(|d| d.id == v.device.id)),
    );

    for (at, room) in rooms(reg, views, &hosts).await {
        let room = match room {
            Ok(room) => room,
            Err(e) => {
                eprintln!("phone: {e:#}; booting without a memory check");
                continue;
            }
        };

        let here: Vec<&&Device> = booting
            .iter()
            .filter(|d| actions::where_of(d) == at)
            .collect();

        let need: f64 = here.iter().map(|d| room.cost(d)).sum();

        eprintln!("phone: {}", room.line());

        let Some(why) = room.admits(need) else {
            continue;
        };

        let names = here
            .iter()
            .map(|d| d.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        if over {
            eprintln!("phone: {why}; booting {names} anyway (--over-budget)");
            continue;
        }

        let mut said = vec![format!("{names} not booted: {why}")];

        said.extend(room.invite());
        said.push("`--over-budget` boots it anyway; ask before using it".to_string());

        return Err(Refused(said.join("\n")).into());
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guest {
    pub ram: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub paged_in: u64,
}

/// Pages read back from swap in the second sampled: swap in use stays after a
/// shortage has passed, pages coming back mean the guest is short now.
const THRASHING: u64 = 256;

impl Guest {
    pub fn heavy(&self) -> bool {
        self.paged_in >= THRASHING
    }
}

fn guest_of(text: &str) -> Option<Guest> {
    let f = fields(text);
    let swap_total = f.get("SwapTotal")? * 1024;

    Some(Guest {
        ram: f.get("MemTotal")? * 1024,
        swap_total,
        swap_used: swap_total.saturating_sub(f.get("SwapFree")? * 1024),
        paged_in: match text
            .lines()
            .filter_map(|l| l.strip_prefix("pswpin ")?.trim().parse::<u64>().ok())
            .collect::<Vec<_>>()[..]
        {
            [first, second] => second.saturating_sub(first),
            _ => 0,
        },
    })
}

pub async fn guest(server: &Server, serial: &str) -> Option<Guest> {
    let out = adb::run_timeout(
        server,
        &[
            "-s",
            serial,
            "shell",
            "cat /proc/meminfo; grep pswpin /proc/vmstat; sleep 1; grep pswpin /proc/vmstat",
        ],
        Duration::from_secs(7),
    )
    .await
    .ok()?;

    guest_of(&out.stdout)
}

pub fn swapping(label: &str, guest: &Guest) -> String {
    format!(
        "{label} is thrashing: {:.1} MB/s paged back in from swap, {:.1} of {:.1} GB of guest swap in use on {:.1} GB of RAM; it takes taps and may never act on them",
        guest.paged_in as f64 * 4096.0 / 1e6,
        gb(guest.swap_used),
        gb(guest.swap_total),
        gb(guest.ram)
    )
}

pub async fn strain(server: &Server, device: &Device, serial: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();

    if device.platform == Platform::Emulator {
        if let Some(guest) = match serial {
            Some(serial) => guest(server, serial).await,
            None => None,
        } {
            if guest.heavy() {
                out.push(swapping(&device.label, &guest));
            }
        }
    }

    if matches!(device.platform, Platform::Emulator | Platform::Simulator) {
        let at = actions::where_of(device);

        if let Ok((memory, _)) = read(&at, &[]).await {
            if memory.level != Level::Normal {
                out.push(format!(
                    "{} is under memory pressure ({}, {:.1} GB available): its devices take taps and may never act on them",
                    at.label(),
                    memory.level.as_str(),
                    gb(memory.available)
                ));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const VM_STAT: &str = r#"os: darwin
memsize: 17179869184
pressure: 1
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                    70064.
Pages active:                                 300123.
Pages inactive:                               349855.
Pages speculative:                             11798.
Pages throttled:                                   0.
Pages wired down:                             150850.
Pages purgeable:                                3814.
"Translation faults":                       42024016.
File-backed pages:                            286700.
Anonymous pages:                              375076.
Pages stored in compressor:                   347303.
Pages occupied by compressor:                 132435.
Swapouts:                                          0.
avd pixel_7-api36 2G
avd pixel_7-api36-b
"#;

    const MEMINFO: &str = "os: linux
MemTotal:       16303284 kB
MemFree:          412000 kB
MemAvailable:    1200000 kB
SwapTotal:             0 kB
SwapFree:              0 kB
";

    #[test]
    fn reads_what_a_mac_can_hand_back_without_compressing() {
        let (memory, avds) = parse(VM_STAT).unwrap();

        assert_eq!(memory.total, 17_179_869_184);
        assert_eq!(memory.available, (70_064 + 286_700 + 3_814) * 16_384);
        assert_eq!(memory.level, Level::Normal);
        assert_eq!(avds.get("pixel_7-api36"), Some(&2.0));
        assert_eq!(avds.get("pixel_7-api36-b"), Some(&DEFAULT_RAM));
    }

    #[test]
    fn takes_the_mac_kernel_pressure_verdict_as_it_is() {
        let warn = VM_STAT.replace("pressure: 1", "pressure: 2");
        let critical = VM_STAT.replace("pressure: 1", "pressure: 4");

        assert_eq!(parse(&warn).unwrap().0.level, Level::Warn);
        assert_eq!(parse(&critical).unwrap().0.level, Level::Critical);
    }

    #[test]
    fn reads_linux_available_memory_and_calls_a_sliver_of_it_pressure() {
        let (memory, avds) = parse(MEMINFO).unwrap();

        assert_eq!(memory.total, 16_303_284 * 1024);
        assert_eq!(memory.available, 1_200_000 * 1024);
        assert_eq!(memory.level, Level::Warn);
        assert!(avds.is_empty());
    }

    #[test]
    fn refuses_output_it_does_not_recognise() {
        assert!(parse("").is_none());
        assert!(parse("os: plan9\nfree: 3\n").is_none());
        assert!(parse("os: darwin\nPages free: 3.\n").is_none());
    }

    #[test]
    fn reads_ram_size_in_every_unit_the_emulator_takes() {
        assert_eq!(ram_size("2G"), Some(2.0));
        assert_eq!(ram_size("2GB"), Some(2.0));
        assert_eq!(ram_size("2048"), Some(2.0));
        assert_eq!(ram_size("1536M"), Some(1.5));
        assert_eq!(ram_size(" 3072 mb "), Some(3.0));
        assert_eq!(ram_size(""), None);
        assert_eq!(ram_size("lots"), None);
    }

    #[test]
    fn a_guest_paging_swap_back_in_is_heavy() {
        let text = "MemTotal: 2014000 kB\nSwapTotal: 1510000 kB\nSwapFree: 250000 kB\npswpin 1000\npswpin 3000\n";
        let guest = guest_of(text).unwrap();

        assert_eq!(guest.swap_used, 1_260_000 * 1024);
        assert_eq!(guest.paged_in, 2000);
        assert!(guest.heavy());

        let settled = "MemTotal: 2014000 kB\nSwapTotal: 1510000 kB\nSwapFree: 250000 kB\npswpin 1000\npswpin 1004\n";
        let none = "MemTotal: 2014000 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n";

        assert!(!guest_of(settled).unwrap().heavy());
        assert!(!guest_of(none).unwrap().heavy());
    }

    fn room(tenants: &[(&str, f64, Option<&str>)], level: Level) -> Room {
        Room {
            at: Where::On("rose".into()),
            memory: Memory {
                total: 16 << 30,
                available: 3 << 30,
                level,
            },
            budget: Budget::default(),
            tenants: tenants
                .iter()
                .map(|(label, gb, holder)| Tenant {
                    label: label.to_string(),
                    gb: *gb,
                    holder: holder.map(|p| format!("{p} (claude on nyx, idle 3m)")),
                })
                .collect(),
            emulator: Some(2.5),
            simulators: true,
            ram: BTreeMap::new(),
        }
    }

    #[test]
    fn invites_as_many_devices_as_the_budget_has_room_for() {
        let room = room(&[("iPhone 17", 3.7, Some("dazzle"))], Level::Normal);

        assert!((room.left() - 6.3).abs() < 1e-9);
        assert!(room.admits(2.5).is_none());
        assert!(room.line().starts_with(
            "rose: 16 GB, 6.0 reserved, 3.7 committed by 1 device (iPhone 17 3.7 held by dazzle ("
        ));
        assert_eq!(
            room.invite(),
            vec!["you may boot 2 more emulators or 1 more simulator on rose"]
        );
    }

    #[test]
    fn points_at_an_unheld_device_before_offering_to_boot_one() {
        let room = room(
            &[
                ("iPhone 17", 3.7, Some("dazzle")),
                ("pixel_7-api36", 2.5, None),
            ],
            Level::Normal,
        );

        assert_eq!(
            room.invite()[0],
            "pixel_7-api36 is running and nobody holds it: use it with -t pixel_7-api36 before booting another"
        );
    }

    #[test]
    fn a_full_budget_refuses_and_says_to_wait() {
        let room = room(
            &[
                ("iPhone 17", 3.7, Some("dazzle")),
                ("pixel_7-api36", 2.5, Some("hotline")),
                ("pixel_7-api36-b", 2.5, Some("clipz")),
            ],
            Level::Normal,
        );

        assert!(room.admits(2.5).unwrap().starts_with("no room on rose"));
        assert_eq!(
            room.invite(),
            vec!["no room on rose for another device: wait for a `phone down`, or ask a holder above to release theirs"]
        );
    }

    #[test]
    fn says_whether_one_more_device_fits_and_what_to_use_when_it_does_not() {
        let avd = Device::new(
            "avd:rose/pixel_7-api36-c",
            "pixel_7-api36-c",
            Platform::Emulator,
        );

        let roomy = room(&[("iPhone 17", 3.7, Some("dazzle"))], Level::Normal);

        assert_eq!(
            roomy.fit(&avd),
            Fit::Room("rose has room for it (6.3 GB left, it needs 2.5)".into())
        );

        let full = room(
            &[
                ("iPhone 17", 3.7, Some("dazzle")),
                ("pixel_7-api36", 2.5, None),
                ("pixel_7-api36-b", 2.5, Some("clipz")),
            ],
            Level::Normal,
        );

        let Fit::Full(said) = full.fit(&avd) else {
            panic!("a full host has no room");
        };

        assert!(said[0].starts_with("no room on rose: this needs 2.5 GB"));
        assert!(said[1].starts_with("pixel_7-api36 is running and nobody holds it"));
    }

    #[test]
    fn the_brief_form_carries_the_same_figures_as_the_invitation() {
        let open = room(&[("iPhone 17", 3.7, Some("dazzle"))], Level::Normal);

        assert_eq!(open.brief(), "room to boot 2 emu or 1 sim");
        assert_eq!(open.room_for(Platform::Emulator), Some(2));
        assert_eq!(open.advice(), Vec::<String>::new());

        let full = room(
            &[("a", 5.0, Some("x")), ("b", 5.0, Some("y"))],
            Level::Normal,
        );

        assert_eq!(full.brief(), "no room to boot");

        let pressed = room(&[], Level::Warn);

        assert_eq!(pressed.brief(), "memory pressure warn, boot nothing");
        assert_eq!(pressed.room_for(Platform::Simulator), Some(0));
    }

    #[test]
    fn pressure_vetoes_what_the_budget_would_allow() {
        let room = room(&[], Level::Warn);

        assert_eq!(
            room.admits(2.5).as_deref(),
            Some("rose is under memory pressure (warn)")
        );
        assert_eq!(room.invite().len(), 1);
        assert!(room.invite()[0].starts_with("rose is under memory pressure (warn)"));
    }
}
