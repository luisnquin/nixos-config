use anyhow::Result;
use chrono::Utc;
use serde::Serialize;

use crate::cli::{ExperimentCommand, Format};
use crate::cmd;
use crate::ids;
use crate::model::{Experiment, ExperimentStatus, Measurement};
use crate::store::Workbench;

#[derive(Serialize)]
struct ExperimentView {
    #[serde(flatten)]
    experiment: Experiment,
    measurements: Vec<Measurement>,
}

pub fn run(command: ExperimentCommand) -> Result<i32> {
    match command {
        ExperimentCommand::New {
            reference,
            title,
            hypothesis,
            tags,
            status,
        } => new(reference, title, hypothesis, tags, status),
        ExperimentCommand::List {
            project,
            status,
            format,
        } => list(project, status, format),
        ExperimentCommand::Show { reference, format } => show(reference, format),
        ExperimentCommand::Set {
            reference,
            status,
            title,
            hypothesis,
        } => set(reference, status, title, hypothesis),
    }
}

fn new(
    reference: String,
    title: Option<String>,
    hypothesis: Option<String>,
    tags: Vec<String>,
    status: ExperimentStatus,
) -> Result<i32> {
    let (project, slug) = ids::split_ref(&reference)?;

    let workbench = Workbench::open()?;

    workbench.load_project(&project)?;

    let now = Utc::now();

    let experiment = Experiment {
        title: title.unwrap_or_else(|| slug.clone()),
        slug,
        project,
        status,
        hypothesis,
        tags,
        created_at: now,
        updated_at: now,
    };

    let path = workbench.create_experiment(&experiment)?;

    println!("{}", cmd::relative(&workbench.root, &path));

    Ok(0)
}

fn list(project: Option<String>, status: Option<ExperimentStatus>, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let experiments: Vec<Experiment> = workbench
        .list_experiments()?
        .into_iter()
        .filter(|experiment| {
            project
                .as_ref()
                .is_none_or(|wanted| &experiment.project == wanted)
                && status.is_none_or(|wanted| experiment.status == wanted)
        })
        .collect();

    if format.json {
        cmd::emit_json(&experiments)?;

        return Ok(0);
    }

    let rows = experiments
        .iter()
        .map(|experiment| {
            vec![
                experiment.reference(),
                format!("{:?}", experiment.status).to_lowercase(),
                cmd::stamp(experiment.updated_at),
                experiment.title.clone(),
            ]
        })
        .collect::<Vec<_>>();

    cmd::print_table(&["EXPERIMENT", "STATUS", "UPDATED", "TITLE"], &rows);

    Ok(0)
}

fn show(reference: String, format: Format) -> Result<i32> {
    let (project, slug) = ids::split_ref(&reference)?;

    let workbench = Workbench::open()?;
    let experiment = workbench.load_experiment(&project, &slug)?;

    let measurements: Vec<Measurement> = workbench
        .list_measurements()?
        .into_iter()
        .filter(|measurement| {
            measurement.project == project && measurement.experiment.as_deref() == Some(&slug)
        })
        .collect();

    let view = ExperimentView {
        experiment,
        measurements,
    };

    if format.json {
        cmd::emit_json(&view)?;

        return Ok(0);
    }

    println!("experiment {}", view.experiment.reference());
    println!("title      {}", view.experiment.title);
    println!(
        "status     {}",
        format!("{:?}", view.experiment.status).to_lowercase()
    );
    println!("created    {}", cmd::stamp(view.experiment.created_at));
    println!("updated    {}", cmd::stamp(view.experiment.updated_at));

    if let Some(hypothesis) = &view.experiment.hypothesis {
        println!("hypothesis {hypothesis}");
    }

    if !view.experiment.tags.is_empty() {
        println!("tags       {}", view.experiment.tags.join(", "));
    }

    println!();

    let rows = view
        .measurements
        .iter()
        .map(|measurement| {
            vec![
                measurement.id.clone(),
                measurement.quantity.clone(),
                format!("{} {}", measurement.value, measurement.unit),
                cmd::dash(measurement.instrument.as_ref()),
                cmd::dash(measurement.note.as_ref()),
            ]
        })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        println!("no measurements");
    } else {
        cmd::print_table(&["EVENT", "QUANTITY", "VALUE", "INSTRUMENT", "NOTE"], &rows);
    }

    Ok(0)
}

fn set(
    reference: String,
    status: Option<ExperimentStatus>,
    title: Option<String>,
    hypothesis: Option<String>,
) -> Result<i32> {
    let (project, slug) = ids::split_ref(&reference)?;

    let workbench = Workbench::open()?;
    let mut experiment = workbench.load_experiment(&project, &slug)?;

    if status.is_none() && title.is_none() && hypothesis.is_none() {
        anyhow::bail!("nothing to set: pass --status, --title or --hypothesis");
    }

    if let Some(status) = status {
        experiment.status = status;
    }

    if let Some(title) = title {
        experiment.title = title;
    }

    if let Some(hypothesis) = hypothesis {
        experiment.hypothesis = Some(hypothesis);
    }

    experiment.updated_at = Utc::now();

    let path = workbench.save_experiment(&experiment)?;

    println!("{}", cmd::relative(&workbench.root, &path));

    Ok(0)
}
