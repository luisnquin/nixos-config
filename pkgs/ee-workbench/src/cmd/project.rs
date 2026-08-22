use anyhow::{Result, bail};
use chrono::Utc;
use serde::Serialize;

use crate::cli::{Format, ProjectCommand};
use crate::cmd;
use crate::ids;
use crate::model::{Project, ProjectStatus};
use crate::store::{Checkouts, Workbench};

#[derive(Serialize)]
struct ProjectView {
    #[serde(flatten)]
    project: Project,
    experiments: usize,
    measurements: usize,
    /// Machine-local, from XDG state: never part of the repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    checkout: Option<String>,
}

pub fn run(command: ProjectCommand) -> Result<i32> {
    match command {
        ProjectCommand::New {
            slug,
            title,
            summary,
            tags,
            status,
        } => new(slug, title, summary, tags, status),
        ProjectCommand::List { status, format } => list(status, format),
        ProjectCommand::Show { slug, format } => show(slug, format),
        ProjectCommand::Link { slug, path, remove } => link(slug, path, remove),
    }
}

fn new(
    slug: String,
    title: Option<String>,
    summary: Option<String>,
    tags: Vec<String>,
    status: ProjectStatus,
) -> Result<i32> {
    ids::check_slug("project", &slug)?;

    let workbench = Workbench::open()?;
    let now = Utc::now();

    let project = Project {
        id: ids::event_id(now)?,
        title: title.unwrap_or_else(|| slug.clone()),
        slug,
        status,
        summary,
        tags,
        created_at: now,
    };

    let path = workbench.create_project(&project)?;

    println!("{}", cmd::relative(&workbench.root, &path));

    Ok(0)
}

fn list(status: Option<ProjectStatus>, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let projects: Vec<Project> = workbench
        .list_projects()?
        .into_iter()
        .filter(|project| status.is_none_or(|wanted| project.status == wanted))
        .collect();

    if format.json {
        cmd::emit_json(&projects)?;

        return Ok(0);
    }

    let rows = projects
        .iter()
        .map(|project| {
            vec![
                project.slug.clone(),
                format!("{:?}", project.status).to_lowercase(),
                cmd::stamp(project.created_at),
                project.title.clone(),
            ]
        })
        .collect::<Vec<_>>();

    cmd::print_table(&["SLUG", "STATUS", "CREATED", "TITLE"], &rows);

    Ok(0)
}

fn show(slug: String, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;
    let project = workbench.load_project(&slug)?;

    let experiments = workbench
        .list_experiments()?
        .into_iter()
        .filter(|experiment| experiment.project == slug)
        .count();

    let measurements = workbench
        .list_measurements()?
        .into_iter()
        .filter(|measurement| measurement.project == slug)
        .count();

    let checkout = Checkouts::load()?.paths.get(&project.id).cloned();

    let view = ProjectView {
        project,
        experiments,
        measurements,
        checkout,
    };

    if format.json {
        cmd::emit_json(&view)?;

        return Ok(0);
    }

    println!("project      {}", view.project.slug);
    println!("id           {}", view.project.id);
    println!("title        {}", view.project.title);
    println!(
        "status       {}",
        format!("{:?}", view.project.status).to_lowercase()
    );
    println!("created      {}", cmd::stamp(view.project.created_at));

    if let Some(summary) = &view.project.summary {
        println!("summary      {summary}");
    }

    if !view.project.tags.is_empty() {
        println!("tags         {}", view.project.tags.join(", "));
    }

    println!("experiments  {}", view.experiments);
    println!("measurements {}", view.measurements);
    println!("checkout     {}", cmd::dash(view.checkout.as_ref()));

    Ok(0)
}

fn link(slug: String, path: Option<String>, remove: bool) -> Result<i32> {
    let workbench = Workbench::open()?;
    let project = workbench.load_project(&slug)?;

    let mut checkouts = Checkouts::load()?;

    if remove {
        if checkouts.paths.remove(&project.id).is_none() {
            bail!("project {slug:?} has no checkout on this machine");
        }

        let stored = checkouts.save()?;

        println!("unlinked {slug} from {}", stored.display());

        return Ok(0);
    }

    let target = match path {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_dir()?,
    };

    let target = target.canonicalize()?;

    if !target.is_dir() {
        bail!("{} is not a directory", target.display());
    }

    checkouts
        .paths
        .insert(project.id.clone(), target.display().to_string());

    checkouts.save()?;

    println!("{} -> {}", slug, target.display());

    Ok(0)
}
