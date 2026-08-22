use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;

use crate::cli::{Format, RepoCommand};
use crate::cmd;
use crate::git;
use crate::paths;
use crate::store::Workbench;

#[derive(Serialize)]
struct Counts {
    projects: usize,
    parts: usize,
    stock_events: usize,
    experiments: usize,
    measurements: usize,
}

/// Every XDG root the workbench touches. Only `data` is authoritative; the
/// rest are where derived, machine-local and transient state belong.
#[derive(Serialize)]
struct Roots {
    data: String,
    config: String,
    cache: String,
    state: String,
    runtime: String,
}

#[derive(Serialize)]
struct Status {
    roots: Roots,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    repository: bool,
    /// `git status --porcelain` lines: what a commit would sweep up.
    dirty: Vec<String>,
    counts: Counts,
}

#[derive(Serialize)]
struct Problem {
    kind: String,
    subject: String,
    message: String,
}

#[derive(Serialize)]
struct CheckReport {
    root: String,
    counts: Counts,
    problems: Vec<Problem>,
}

pub fn run(command: RepoCommand) -> Result<i32> {
    match command {
        RepoCommand::Init { path } => init(path),
        RepoCommand::Status { format } => status(format),
        RepoCommand::Path => {
            println!("{}", paths::data_root().display());

            Ok(0)
        }
        RepoCommand::Check { format } => check(format),
    }
}

fn counts(workbench: &Workbench) -> Result<Counts> {
    Ok(Counts {
        projects: workbench.list_projects()?.len(),
        parts: workbench.list_parts()?.len(),
        stock_events: workbench.list_stock_events()?.len(),
        experiments: workbench.list_experiments()?.len(),
        measurements: workbench.list_measurements()?.len(),
    })
}

fn init(path: Option<String>) -> Result<i32> {
    let root = path.map(PathBuf::from).unwrap_or_else(paths::data_root);
    let outcome = Workbench::init(root, Utc::now())?;

    println!(
        "{} {}",
        if outcome.created {
            "initialized"
        } else {
            "already initialized"
        },
        outcome.root.display()
    );

    if outcome.git_initialized {
        println!("git repository created (nothing committed)");
    }

    Ok(0)
}

fn status(format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;
    let dirty = git::porcelain(&workbench.root)?;

    let status = Status {
        roots: Roots {
            data: workbench.root.display().to_string(),
            config: paths::config_dir().display().to_string(),
            cache: paths::cache_dir().display().to_string(),
            state: paths::state_dir().display().to_string(),
            runtime: paths::runtime_dir().display().to_string(),
        },
        branch: git::head_branch(&workbench.root)?,
        repository: dirty.is_some(),
        dirty: dirty.unwrap_or_default(),
        counts: counts(&workbench)?,
    };

    if format.json {
        cmd::emit_json(&status)?;

        return Ok(0);
    }

    println!("data         {}", status.roots.data);
    println!("config       {}", status.roots.config);
    println!("cache        {}", status.roots.cache);
    println!("state        {}", status.roots.state);
    println!("runtime      {}", status.roots.runtime);
    println!("branch       {}", cmd::dash(status.branch.as_ref()));
    println!("projects     {}", status.counts.projects);
    println!("parts        {}", status.counts.parts);
    println!("stock events {}", status.counts.stock_events);
    println!("experiments  {}", status.counts.experiments);
    println!("measurements {}", status.counts.measurements);

    if !status.repository {
        println!("git          absent");
    } else if status.dirty.is_empty() {
        println!("git          clean");
    } else {
        println!("git          {} uncommitted paths", status.dirty.len());

        for line in &status.dirty {
            println!("             {line}");
        }
    }

    Ok(0)
}

/// Reads every record and reports what the file layout alone cannot enforce:
/// dangling references, slugs that disagree with their own path, and stock
/// that the ledger drove below zero.
fn check(format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let projects = workbench.list_projects()?;
    let parts = workbench.list_parts()?;
    let events = workbench.list_stock_events()?;
    let experiments = workbench.list_experiments()?;
    let measurements = workbench.list_measurements()?;

    let mut problems = Vec::new();

    let known_project = |slug: &str| projects.iter().any(|project| project.slug == slug);
    let known_part = |slug: &str| parts.iter().any(|part| part.slug == slug);

    for project in &projects {
        if !workbench.project_path(&project.slug).is_file() {
            problems.push(Problem {
                kind: "path-mismatch".into(),
                subject: project.slug.clone(),
                message: "project slug does not match its directory".into(),
            });
        }
    }

    for part in &parts {
        if !workbench.part_path(&part.slug).is_file() {
            problems.push(Problem {
                kind: "path-mismatch".into(),
                subject: part.slug.clone(),
                message: "part slug does not match its file name".into(),
            });
        }
    }

    for event in &events {
        if !workbench.stock_event_path(event.at, &event.id).is_file() {
            problems.push(Problem {
                kind: "path-mismatch".into(),
                subject: event.id.clone(),
                message: "stock event id does not match its file name".into(),
            });
        }

        if !known_part(&event.part) {
            problems.push(Problem {
                kind: "dangling-part".into(),
                subject: event.id.clone(),
                message: format!("stock event references unknown part {:?}", event.part),
            });
        }

        if let Some(project) = &event.project
            && !known_project(project)
        {
            problems.push(Problem {
                kind: "dangling-project".into(),
                subject: event.id.clone(),
                message: format!("stock event references unknown project {project:?}"),
            });
        }
    }

    for experiment in &experiments {
        if !workbench
            .experiment_path(&experiment.project, &experiment.slug)
            .is_file()
        {
            problems.push(Problem {
                kind: "path-mismatch".into(),
                subject: experiment.reference(),
                message: "experiment slug or project does not match its directory".into(),
            });
        }

        if !known_project(&experiment.project) {
            problems.push(Problem {
                kind: "dangling-project".into(),
                subject: experiment.reference(),
                message: format!(
                    "experiment references unknown project {:?}",
                    experiment.project
                ),
            });
        }
    }

    for measurement in &measurements {
        if !workbench
            .measurement_path(&measurement.project, &measurement.id)
            .is_file()
        {
            problems.push(Problem {
                kind: "path-mismatch".into(),
                subject: measurement.id.clone(),
                message: "measurement id or project does not match its file name".into(),
            });
        }

        if !known_project(&measurement.project) {
            problems.push(Problem {
                kind: "dangling-project".into(),
                subject: measurement.id.clone(),
                message: format!(
                    "measurement references unknown project {:?}",
                    measurement.project
                ),
            });
        }

        if let Some(slug) = &measurement.experiment
            && !experiments
                .iter()
                .any(|other| other.project == measurement.project && &other.slug == slug)
        {
            problems.push(Problem {
                kind: "dangling-experiment".into(),
                subject: measurement.id.clone(),
                message: format!("measurement references unknown experiment {slug:?}"),
            });
        }
    }

    for entry in workbench.stock()? {
        if entry.on_hand < 0 {
            problems.push(Problem {
                kind: "negative-stock".into(),
                subject: entry.part.clone(),
                message: format!("ledger sums to {}", entry.on_hand),
            });
        }
    }

    let report = CheckReport {
        root: workbench.root.display().to_string(),
        counts: counts(&workbench)?,
        problems,
    };

    if format.json {
        cmd::emit_json(&report)?;
    } else if report.problems.is_empty() {
        println!(
            "ok: {} projects, {} parts, {} stock events, {} experiments, {} measurements",
            report.counts.projects,
            report.counts.parts,
            report.counts.stock_events,
            report.counts.experiments,
            report.counts.measurements
        );
    } else {
        for problem in &report.problems {
            println!(
                "{}: {} — {}",
                problem.kind, problem.subject, problem.message
            );
        }
    }

    Ok(i32::from(!report.problems.is_empty()))
}
