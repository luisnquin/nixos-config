use anyhow::{Result, bail};
use chrono::Utc;

use crate::cli::{Format, MeasurementCommand};
use crate::cmd;
use crate::ids;
use crate::model::Measurement;
use crate::store::Workbench;

pub fn run(command: MeasurementCommand) -> Result<i32> {
    match command {
        MeasurementCommand::Record {
            project,
            quantity,
            value,
            unit,
            experiment,
            instrument,
            note,
        } => record(project, quantity, value, unit, experiment, instrument, note),
        MeasurementCommand::List {
            project,
            experiment,
            quantity,
            limit,
            format,
        } => list(project, experiment, quantity, limit, format),
        MeasurementCommand::Show { id, format } => show(id, format),
    }
}

#[allow(clippy::too_many_arguments)]
fn record(
    project: String,
    quantity: String,
    value: f64,
    unit: String,
    experiment: Option<String>,
    instrument: Option<String>,
    note: Option<String>,
) -> Result<i32> {
    if !value.is_finite() {
        bail!("--value must be a finite number");
    }

    let workbench = Workbench::open()?;

    workbench.load_project(&project)?;

    if let Some(experiment) = &experiment {
        workbench.load_experiment(&project, experiment)?;
    }

    let now = Utc::now();

    let measurement = Measurement {
        id: ids::event_id(now)?,
        project,
        quantity,
        value,
        unit,
        experiment,
        instrument,
        note,
        at: now,
    };

    let (measurement, path) = workbench.append_measurement(measurement)?;

    println!("{}", cmd::relative(&workbench.root, &path));
    println!("{}", measurement.id);

    Ok(0)
}

fn list(
    project: Option<String>,
    experiment: Option<String>,
    quantity: Option<String>,
    limit: Option<usize>,
    format: Format,
) -> Result<i32> {
    let workbench = Workbench::open()?;

    let mut measurements = workbench.list_measurements()?;

    measurements.retain(|measurement| {
        project
            .as_ref()
            .is_none_or(|wanted| &measurement.project == wanted)
            && experiment
                .as_ref()
                .is_none_or(|wanted| measurement.experiment.as_ref() == Some(wanted))
            && quantity
                .as_ref()
                .is_none_or(|wanted| &measurement.quantity == wanted)
    });

    if let Some(limit) = limit {
        if limit == 0 {
            bail!("--limit must be greater than zero");
        }

        let skip = measurements.len().saturating_sub(limit);
        measurements.drain(..skip);
    }

    if format.json {
        cmd::emit_json(&measurements)?;

        return Ok(0);
    }

    let rows = measurements
        .iter()
        .map(|measurement| {
            vec![
                measurement.id.clone(),
                measurement.project.clone(),
                cmd::dash(measurement.experiment.as_ref()),
                measurement.quantity.clone(),
                format!("{} {}", measurement.value, measurement.unit),
                cmd::dash(measurement.note.as_ref()),
            ]
        })
        .collect::<Vec<_>>();

    cmd::print_table(
        &[
            "EVENT",
            "PROJECT",
            "EXPERIMENT",
            "QUANTITY",
            "VALUE",
            "NOTE",
        ],
        &rows,
    );

    Ok(0)
}

fn show(id: String, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let Some(measurement) = workbench
        .list_measurements()?
        .into_iter()
        .find(|measurement| measurement.id == id)
    else {
        bail!("unknown measurement {id:?}");
    };

    if format.json {
        cmd::emit_json(&measurement)?;

        return Ok(0);
    }

    println!("event      {}", measurement.id);
    println!("project    {}", measurement.project);
    println!("experiment {}", cmd::dash(measurement.experiment.as_ref()));
    println!("quantity   {}", measurement.quantity);
    println!("value      {} {}", measurement.value, measurement.unit);
    println!("instrument {}", cmd::dash(measurement.instrument.as_ref()));
    println!("note       {}", cmd::dash(measurement.note.as_ref()));
    println!("at         {}", cmd::stamp(measurement.at));

    Ok(0)
}
