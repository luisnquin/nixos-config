use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::actions;
use crate::lease::{self, Leases};
use crate::memory::Room;
use crate::model::{self, Device, Platform, Unix, View};
use crate::project::Project;
use crate::ssh::Where;

const TIMEOUT: Duration = Duration::from_secs(6);

pub const MARK: &str = "\n@usage";

const STAMP: &str = r#"state="${XDG_STATE_HOME:-$HOME/.local/state}/phone"
mkdir -p "$state" || exit 1
file="$state/usage.tsv"
tmp="$file.$$.tmp"
{ cat "$file" 2>/dev/null; } | awk -F '\t' -v OFS='\t' -v d="$1" -v t="$2" -v at="$3" '
NF < 4 { next }
$1 == d && $2 == t { $3 = at; $4 = $4 + 1; hit = 1 }
{ print }
END { if (!hit) print d, t, at, 1 }' > "$tmp" && mv "$tmp" "$file""#;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Use {
    pub at: Unix,
    pub count: u64,
}

impl Use {
    fn add(self, other: Use) -> Use {
        Use {
            at: self.at.max(other.at),
            count: self.count + other.count,
        }
    }

    pub fn label(&self) -> String {
        let ago = model::ago(self.at);

        format!(
            "{} · {}×",
            ago.strip_suffix(" ago").unwrap_or("now"),
            self.count
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct Usage {
    rows: BTreeMap<String, BTreeMap<String, Use>>,
}

impl Usage {
    pub fn parse(text: &str) -> Self {
        let mut usage = Usage::default();

        for line in text.lines() {
            let mut cols = line.split('\t');

            let (Some(device), Some(tree), Some(at), Some(count)) =
                (cols.next(), cols.next(), cols.next(), cols.next())
            else {
                continue;
            };

            let (Ok(at), Ok(count)) = (at.trim().parse(), count.trim().parse()) else {
                continue;
            };

            usage
                .rows
                .entry(device.to_string())
                .or_default()
                .insert(tree.to_string(), Use { at, count });
        }

        usage
    }

    pub fn of(&self, device: &Device, tree: Option<&str>) -> Driven {
        let Some(trees) = self.rows.get(&lease::key(device).id) else {
            return Driven::default();
        };

        Driven {
            project: tree.and_then(|tree| trees.get(tree)).copied(),
            all: trees.values().copied().reduce(Use::add),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Driven {
    pub project: Option<Use>,
    pub all: Option<Use>,
}

impl Driven {
    fn rank(&self) -> (u8, Reverse<u64>, Reverse<Unix>) {
        match (self.project, self.all) {
            (Some(mine), _) => (0, Reverse(mine.count), Reverse(mine.at)),
            (None, Some(all)) => (1, Reverse(all.count), Reverse(all.at)),
            (None, None) => (2, Reverse(0), Reverse(0)),
        }
    }

    pub fn label(&self) -> String {
        self.all
            .map(|all| all.label())
            .unwrap_or_else(|| "-".into())
    }
}

/// Device id -> how much it was driven, by the project at `tree` and by anyone.
pub fn driven(
    views: &[View],
    ledgers: &[(Where, Leases)],
    tree: Option<&str>,
) -> BTreeMap<String, Driven> {
    views
        .iter()
        .filter_map(|view| {
            let at = actions::where_of(&view.device);
            let (_, leases) = ledgers.iter().find(|(known, _)| *known == at)?;

            Some((view.device.id.clone(), leases.usage.of(&view.device, tree)))
        })
        .collect()
}

pub fn rank<T>(items: &mut [T], driven: impl Fn(&T) -> Driven) {
    items.sort_by_key(|item| driven(item).rank());
}

pub fn usual<'a>(
    views: &[&'a View],
    driven: &BTreeMap<String, Driven>,
) -> Option<(&'a View, bool)> {
    let used = |v: &View| driven.get(&v.device.id).copied().unwrap_or_default();

    let mut ranked = views.to_vec();
    rank(&mut ranked, |v| used(v));

    let top = *ranked.first()?;

    match used(top).rank().0 {
        0 => Some((top, true)),
        1 => Some((top, false)),
        _ => None,
    }
}

pub fn summary(
    at: &Where,
    views: &[View],
    driven: &BTreeMap<String, Driven>,
    room: Option<&Room>,
) -> String {
    let here: Vec<&View> = views
        .iter()
        .filter(|v| matches!(v.device.platform, Platform::Emulator | Platform::Simulator))
        .filter(|v| actions::where_of(&v.device) == *at)
        .collect();

    let mut parts = Vec::new();

    for platform in [Platform::Emulator, Platform::Simulator] {
        let of: Vec<&&View> = here
            .iter()
            .filter(|v| v.device.platform == platform)
            .collect();

        if of.is_empty() {
            continue;
        }

        let up = of.iter().filter(|v| actions::running(&v.reach)).count();

        parts.push(format!("{} {} ({up} up)", of.len(), platform.as_str()));
    }

    match usual(&here, driven) {
        Some((view, true)) => parts.push(format!("this project drives {}", view.device.label)),
        Some((view, false)) => parts.push(format!("most driven {}", view.device.label)),
        None => {}
    }

    if let Some(room) = room {
        parts.push(room.brief());
    }

    format!("{}: {}", at.label(), parts.join(" · "))
}

pub async fn stamp(device: &Device) {
    let tree = match Project::here().ok().flatten() {
        Some(project) => match lease::tree(&project).await {
            Ok(tree) => tree,
            Err(_) => return,
        },
        None => String::new(),
    };

    let at = model::now().to_string();
    let key = lease::key(device).id;

    let _ = actions::where_of(device)
        .exec(STAMP, &[&key, &tree, &at], TIMEOUT)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Reach;

    fn on_rose(id: &str, label: &str, platform: Platform, reach: Reach) -> View {
        let mut device = Device::new(id, label, platform);
        device.host = Some("rose".into());

        View::new(device, reach)
    }

    #[test]
    fn a_host_is_summed_up_with_what_this_project_drives_there() {
        let views = [
            on_rose("avd:rose/pixel", "pixel", Platform::Emulator, Reach::Off),
            on_rose("avd:rose/tab", "tab", Platform::Emulator, Reach::Off),
            on_rose("AAAA", "iPhone 17", Platform::Simulator, Reach::Online),
            View::new(
                Device::new("SERIAL", "handset", Platform::Android),
                Reach::Off,
            ),
        ];

        let once = Use { at: 1, count: 1 };
        let driven = BTreeMap::from([
            (
                "avd:rose/tab".to_string(),
                Driven {
                    project: None,
                    all: Some(Use { at: 1, count: 50 }),
                },
            ),
            (
                "avd:rose/pixel".to_string(),
                Driven {
                    project: Some(once),
                    all: Some(once),
                },
            ),
        ]);

        let rose = Where::On("rose".into());

        assert_eq!(
            summary(&rose, &views, &driven, None),
            "rose: 2 emu (0 up) · 1 sim (1 up) · this project drives pixel"
        );

        let theirs = BTreeMap::from([("avd:rose/tab".to_string(), driven["avd:rose/tab"])]);

        assert_eq!(
            summary(&rose, &views, &theirs, None),
            "rose: 2 emu (0 up) · 1 sim (1 up) · most driven tab"
        );
    }

    fn avd(name: &str) -> Device {
        let mut device = Device::new(format!("avd:rose/{name}"), name, Platform::Emulator);
        device.host = Some("rose".into());

        device
    }

    #[test]
    fn reads_every_project_and_sums_them_for_everyone() {
        let usage = Usage::parse(
            "avd:pixel\t/a\t100\t3\navd:pixel\t/b\t300\t2\nbroken line\navd:tab\t/b\tx\t1\n",
        );

        let driven = usage.of(&avd("pixel"), Some("/a"));

        assert_eq!(driven.project, Some(Use { at: 100, count: 3 }));
        assert_eq!(driven.all, Some(Use { at: 300, count: 5 }));
        assert_eq!(usage.of(&avd("tab"), Some("/b")), Driven::default());
        assert_eq!(usage.of(&avd("pixel"), None).project, None);
    }

    #[test]
    fn this_project_first_then_everyone_then_the_rest() {
        let used = |project: Option<u64>, all: Option<u64>| Driven {
            project: project.map(|count| Use { at: 1, count }),
            all: all.map(|count| Use { at: 1, count }),
        };

        let mut rows = vec![
            ("never", used(None, None)),
            ("theirs", used(None, Some(90))),
            ("mine-once", used(Some(1), Some(1))),
            ("also-never", used(None, None)),
            ("mine", used(Some(8), Some(9))),
        ];

        rank(&mut rows, |(_, driven)| *driven);

        assert_eq!(
            rows.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            ["mine", "mine-once", "theirs", "never", "also-never"]
        );
    }

    #[test]
    fn a_label_reads_as_how_long_ago_and_how_often() {
        let driven = Driven {
            project: None,
            all: Some(Use {
                at: model::now() - 7200,
                count: 41,
            }),
        };

        assert_eq!(driven.label(), "2h · 41×");
        assert_eq!(Driven::default().label(), "-");
    }

    #[tokio::test]
    async fn a_stamp_counts_up_and_keeps_projects_apart() {
        let dir = std::env::temp_dir()
            .join(format!("phone-usage-{}", std::process::id()))
            .display()
            .to_string();

        let stamp = |tree: &'static str, at: &'static str| {
            let dir = dir.clone();

            async move {
                Where::Here
                    .exec(
                        &format!("XDG_STATE_HOME='{dir}'\n{STAMP}"),
                        &["avd:pixel", tree, at],
                        TIMEOUT,
                    )
                    .await
                    .unwrap()
            }
        };

        assert!(stamp("/a", "100").await.ok());
        assert!(stamp("/a", "200").await.ok());
        assert!(stamp("", "300").await.ok());

        let text = std::fs::read_to_string(format!("{dir}/phone/usage.tsv")).unwrap();
        let driven = Usage::parse(&text).of(&avd("pixel"), Some("/a"));

        assert_eq!(driven.project, Some(Use { at: 200, count: 2 }));
        assert_eq!(driven.all, Some(Use { at: 300, count: 3 }));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
