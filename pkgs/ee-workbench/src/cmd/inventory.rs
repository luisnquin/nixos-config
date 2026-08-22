use anyhow::{Result, bail};
use chrono::Utc;
use serde::Serialize;

use crate::cli::{Format, InventoryCommand, PartCommand};
use crate::cmd;
use crate::ids;
use crate::model::{Part, Stock, StockEvent};
use crate::store::Workbench;

#[derive(Serialize)]
struct PartView {
    #[serde(flatten)]
    part: Part,
    on_hand: i64,
    events: usize,
}

pub fn run(command: InventoryCommand) -> Result<i32> {
    match command {
        InventoryCommand::Part { command } => part(command),
        InventoryCommand::Receive {
            part,
            qty,
            location,
            note,
        } => movement(part, i64::from(qty), location, None, note),
        InventoryCommand::Consume {
            part,
            qty,
            project,
            note,
        } => movement(part, -i64::from(qty), None, project, note),
        InventoryCommand::Stock { part, format } => stock(part, format),
        InventoryCommand::Events {
            part,
            limit,
            format,
        } => events(part, limit, format),
    }
}

fn part(command: PartCommand) -> Result<i32> {
    match command {
        PartCommand::Add {
            slug,
            name,
            kind,
            package,
            mpn,
            datasheet,
            tags,
        } => {
            ids::check_slug("part", &slug)?;

            let workbench = Workbench::open()?;

            let part = Part {
                name: name.unwrap_or_else(|| slug.clone()),
                slug,
                kind,
                package,
                mpn,
                datasheet,
                tags,
                created_at: Utc::now(),
            };

            let path = workbench.create_part(&part)?;

            println!("{}", cmd::relative(&workbench.root, &path));

            Ok(0)
        }
        PartCommand::List { format } => {
            let workbench = Workbench::open()?;
            let parts = workbench.list_parts()?;

            if format.json {
                cmd::emit_json(&parts)?;

                return Ok(0);
            }

            let rows = parts
                .iter()
                .map(|part| {
                    vec![
                        part.slug.clone(),
                        cmd::dash(part.kind.as_ref()),
                        cmd::dash(part.package.as_ref()),
                        cmd::dash(part.mpn.as_ref()),
                        part.name.clone(),
                    ]
                })
                .collect::<Vec<_>>();

            cmd::print_table(&["SLUG", "KIND", "PACKAGE", "MPN", "NAME"], &rows);

            Ok(0)
        }
        PartCommand::Show { slug, format } => {
            let workbench = Workbench::open()?;
            let part = workbench.load_part(&slug)?;

            let ledger = workbench
                .list_stock_events()?
                .into_iter()
                .filter(|event| event.part == slug)
                .collect::<Vec<_>>();

            let view = PartView {
                part,
                on_hand: ledger.iter().map(|event| event.delta).sum(),
                events: ledger.len(),
            };

            if format.json {
                cmd::emit_json(&view)?;

                return Ok(0);
            }

            println!("part      {}", view.part.slug);
            println!("name      {}", view.part.name);
            println!("kind      {}", cmd::dash(view.part.kind.as_ref()));
            println!("package   {}", cmd::dash(view.part.package.as_ref()));
            println!("mpn       {}", cmd::dash(view.part.mpn.as_ref()));
            println!("datasheet {}", cmd::dash(view.part.datasheet.as_ref()));

            if !view.part.tags.is_empty() {
                println!("tags      {}", view.part.tags.join(", "));
            }

            println!("on hand   {}", view.on_hand);
            println!("events    {}", view.events);

            Ok(0)
        }
    }
}

fn movement(
    part: String,
    delta: i64,
    location: Option<String>,
    project: Option<String>,
    note: Option<String>,
) -> Result<i32> {
    let workbench = Workbench::open()?;

    // A movement against an undefined part would be an orphan in the ledger,
    // and the ledger is append-only: there is no cleaning it up later.
    workbench.load_part(&part)?;

    if let Some(project) = &project {
        workbench.load_project(project)?;
    }

    let now = Utc::now();

    let event = StockEvent {
        id: ids::event_id(now)?,
        part: part.clone(),
        delta,
        location,
        project,
        note,
        at: now,
    };

    let (event, path) = workbench.append_stock_event(event)?;

    let on_hand: i64 = workbench
        .list_stock_events()?
        .iter()
        .filter(|other| other.part == part)
        .map(|other| other.delta)
        .sum();

    if on_hand < 0 {
        eprintln!("warning: {part} is now at {on_hand}; the ledger is missing arrivals");
    }

    println!(
        "{} ({} on hand)",
        cmd::relative(&workbench.root, &path),
        on_hand
    );
    println!("{}", event.id);

    Ok(0)
}

fn stock(part: Option<String>, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let mut stock = workbench.stock()?;

    if let Some(part) = &part {
        workbench.load_part(part)?;
        stock.retain(|entry| &entry.part == part);
    }

    if format.json {
        cmd::emit_json(&stock)?;

        return Ok(0);
    }

    let rows = stock
        .iter()
        .map(|entry: &Stock| {
            vec![
                entry.part.clone(),
                entry.on_hand.to_string(),
                entry.events.to_string(),
                entry
                    .last_event_at
                    .map(cmd::stamp)
                    .unwrap_or_else(|| "-".into()),
                entry.name.clone(),
            ]
        })
        .collect::<Vec<_>>();

    cmd::print_table(&["PART", "ON HAND", "EVENTS", "LAST", "NAME"], &rows);

    Ok(0)
}

fn events(part: Option<String>, limit: Option<usize>, format: Format) -> Result<i32> {
    let workbench = Workbench::open()?;

    let mut events = workbench.list_stock_events()?;

    if let Some(part) = &part {
        events.retain(|event| &event.part == part);
    }

    if let Some(limit) = limit {
        if limit == 0 {
            bail!("--limit must be greater than zero");
        }

        let skip = events.len().saturating_sub(limit);
        events.drain(..skip);
    }

    if format.json {
        cmd::emit_json(&events)?;

        return Ok(0);
    }

    let rows = events
        .iter()
        .map(|event| {
            vec![
                event.id.clone(),
                event.part.clone(),
                format!("{:+}", event.delta),
                cmd::dash(event.location.as_ref()),
                cmd::dash(event.project.as_ref()),
                cmd::dash(event.note.as_ref()),
            ]
        })
        .collect::<Vec<_>>();

    cmd::print_table(
        &["EVENT", "PART", "DELTA", "LOCATION", "PROJECT", "NOTE"],
        &rows,
    );

    Ok(0)
}
