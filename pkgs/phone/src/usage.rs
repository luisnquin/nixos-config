use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::actions;
use crate::lease::{self, Book};
use crate::memory::Room;
use crate::model::{self, Device, Platform, Unix, View};
use crate::ssh::Where;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    books: &[Book],
    tree: Option<&str>,
) -> BTreeMap<String, Driven> {
    views
        .iter()
        .filter_map(|view| {
            let book = lease::book_of(books, &view.device)?;

            Some((view.device.id.clone(), book.usage.of(&view.device, tree)))
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::Reach;

    pub(crate) fn on_rose(id: &str, label: &str, platform: Platform, reach: Reach) -> View {
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

    pub fn avd(name: &str) -> Device {
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
}
