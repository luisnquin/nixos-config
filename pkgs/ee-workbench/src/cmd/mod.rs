pub mod experiment;
pub mod inventory;
pub mod measurement;
pub mod mechanical;
pub mod project;
pub mod repo;

use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

pub fn emit_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}

/// Paths are reported relative to the data root: what the operator sees is
/// what `ee git add` takes.
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M").to_string()
}

pub fn dash(value: Option<&String>) -> String {
    value.cloned().unwrap_or_else(|| "-".to_string())
}

/// Left-aligned columns sized to their contents; the last column is not
/// padded, so output stays clean under `cut` and `awk`.
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(|cell| cell.chars().count())
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    let line = |cells: &[String]| {
        let mut out = String::new();

        for (index, cell) in cells.iter().enumerate() {
            if index + 1 == cells.len() {
                out.push_str(cell);
            } else {
                out.push_str(&format!("{cell:<width$}  ", width = widths[index]));
            }
        }

        println!("{}", out.trim_end());
    };

    line(&headers.iter().map(|h| h.to_string()).collect::<Vec<_>>());

    for row in rows {
        line(row);
    }
}
