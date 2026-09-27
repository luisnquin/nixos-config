use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::model::{self, Device, Unix};
use crate::project::Project;
use crate::{actions, lease};

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
    use crate::model::Platform;
    use crate::ssh::Where;

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
