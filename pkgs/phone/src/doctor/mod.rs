mod host;
pub mod motion;

use anyhow::{bail, Result};

use crate::project::{Project, FILE};
use crate::registry::Registry;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Ok,
    Warn,
    Fail,
}

pub struct Check {
    pub name: String,
    pub verdict: Verdict,
    pub detail: String,
}

impl Check {
    pub fn new(ok: bool, name: &str, detail: String) -> Self {
        Check {
            name: name.to_string(),
            verdict: match ok {
                true => Verdict::Ok,
                false => Verdict::Fail,
            },
            detail,
        }
    }

    pub fn warn(name: &str, detail: String) -> Self {
        Check {
            verdict: Verdict::Warn,
            ..Check::new(true, name, detail)
        }
    }

    fn print(&self) {
        let mark = match self.verdict {
            Verdict::Ok => '✓',
            Verdict::Warn => '!',
            Verdict::Fail => '✗',
        };

        println!("  {mark} {:<16} {}", self.name, self.detail);
    }
}

pub async fn run(reg: &mut Registry) -> Result<()> {
    let mut bad = 0;

    let mut report = |check: Check| {
        if check.verdict == Verdict::Fail {
            bad += 1;
        }
        check.print();
    };

    host::checks(reg, &mut report).await?;
    project().into_iter().for_each(&mut report);

    if bad > 0 {
        bail!("{bad} check(s) failed");
    }

    Ok(())
}

fn project() -> Vec<Check> {
    let project = match Project::here() {
        Ok(Some(project)) => project,
        Ok(None) => return Vec::new(),
        Err(e) => return vec![Check::new(false, FILE, format!("{e:#}"))],
    };

    let file = project.root.join(FILE);
    let found = motion::pins(&project.manifest, &file);

    match found.is_empty() {
        true => vec![Check::new(
            true,
            FILE,
            format!("{}, nothing known to break screen reads", file.display()),
        )],
        false => found,
    }
}
