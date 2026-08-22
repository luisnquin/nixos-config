use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::ids;
use crate::model::{Experiment, Measurement, Part, Project, Stock, StockEvent};
use crate::paths;

/// Marks a directory as a workbench and pins the layout its files use.
pub const MARKER: &str = ".ee-workbench";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Marker {
    pub schema_version: u32,
    pub created_at: DateTime<Utc>,
}

pub struct Workbench {
    pub root: PathBuf,
}

pub struct InitOutcome {
    pub root: PathBuf,
    pub created: bool,
    pub git_initialized: bool,
}

impl Workbench {
    pub fn open() -> Result<Self> {
        Self::open_at(paths::data_root())
    }

    pub fn open_at(root: PathBuf) -> Result<Self> {
        if !root.join(MARKER).is_file() {
            bail!(
                "no workbench at {}: run `ee repo init` first",
                root.display()
            );
        }

        let marker: Marker = read_toml(&root.join(MARKER))?;

        if marker.schema_version > SCHEMA_VERSION {
            bail!(
                "workbench at {} uses schema {} but this build knows {SCHEMA_VERSION}",
                root.display(),
                marker.schema_version
            );
        }

        Ok(Self { root })
    }

    /// Creates the layout and, for a fresh directory, an empty Git repository.
    /// It never commits: the first commit is the operator's call.
    pub fn init(root: PathBuf, now: DateTime<Utc>) -> Result<InitOutcome> {
        let marker = root.join(MARKER);
        let created = !marker.is_file();

        for dir in [
            root.join("projects"),
            root.join("inventory/parts"),
            root.join("inventory/events"),
            root.join("experiments"),
            root.join("measurements"),
        ] {
            fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        }

        if created {
            write_toml(
                &marker,
                &Marker {
                    schema_version: SCHEMA_VERSION,
                    created_at: now,
                },
            )?;
        }

        let git_initialized = !root.join(".git").exists();

        if git_initialized {
            crate::git::init_repo(&root)?;
        }

        Ok(InitOutcome {
            root,
            created,
            git_initialized,
        })
    }

    pub fn project_path(&self, slug: &str) -> PathBuf {
        self.root.join("projects").join(slug).join("project.toml")
    }

    pub fn part_path(&self, slug: &str) -> PathBuf {
        self.root
            .join("inventory/parts")
            .join(format!("{slug}.toml"))
    }

    pub fn stock_event_path(&self, at: DateTime<Utc>, id: &str) -> PathBuf {
        self.root
            .join("inventory/events")
            .join(at.year().to_string())
            .join(format!("{id}.toml"))
    }

    pub fn experiment_path(&self, project: &str, slug: &str) -> PathBuf {
        self.root
            .join("experiments")
            .join(project)
            .join(slug)
            .join("experiment.toml")
    }

    pub fn measurement_path(&self, project: &str, id: &str) -> PathBuf {
        self.root
            .join("measurements")
            .join(project)
            .join(format!("{id}.toml"))
    }

    pub fn load_project(&self, slug: &str) -> Result<Project> {
        ids::check_slug("project", slug)?;

        let path = self.project_path(slug);

        if !path.is_file() {
            bail!("unknown project {slug:?}");
        }

        read_toml(&path)
    }

    pub fn create_project(&self, project: &Project) -> Result<PathBuf> {
        let path = self.project_path(&project.slug);

        if path.exists() {
            bail!("project {:?} already exists", project.slug);
        }

        write_toml(&path, project)?;

        Ok(path)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut projects = Vec::new();

        for slug in child_dirs(&self.root.join("projects"))? {
            let path = self.project_path(&slug);

            if path.is_file() {
                projects.push(read_toml(&path)?);
            }
        }

        Ok(projects)
    }

    pub fn load_part(&self, slug: &str) -> Result<Part> {
        ids::check_slug("part", slug)?;

        let path = self.part_path(slug);

        if !path.is_file() {
            bail!("unknown part {slug:?}");
        }

        read_toml(&path)
    }

    pub fn create_part(&self, part: &Part) -> Result<PathBuf> {
        let path = self.part_path(&part.slug);

        if path.exists() {
            bail!("part {:?} already exists", part.slug);
        }

        write_toml(&path, part)?;

        Ok(path)
    }

    pub fn list_parts(&self) -> Result<Vec<Part>> {
        let mut parts = Vec::new();

        for path in toml_files(&self.root.join("inventory/parts"))? {
            parts.push(read_toml(&path)?);
        }

        Ok(parts)
    }

    /// Events are write-once: a taken name is re-rolled instead of being
    /// overwritten, and `create_new` still refuses the write if it loses.
    pub fn append_stock_event(&self, mut event: StockEvent) -> Result<(StockEvent, PathBuf)> {
        let at = event.at;
        let path = self.free_event_path(&mut event.id, at, |id| self.stock_event_path(at, id))?;

        write_new_toml(&path, &event)?;

        Ok((event, path))
    }

    fn free_event_path(
        &self,
        id: &mut String,
        at: DateTime<Utc>,
        path_of: impl Fn(&str) -> PathBuf,
    ) -> Result<PathBuf> {
        for _ in 0..8 {
            let path = path_of(id);

            if !path.exists() {
                return Ok(path);
            }

            *id = ids::event_id(at)?;
        }

        bail!("could not find a free event name for {id}");
    }

    pub fn list_stock_events(&self) -> Result<Vec<StockEvent>> {
        let events_root = self.root.join("inventory/events");
        let mut events: Vec<StockEvent> = Vec::new();

        for year in child_dirs(&events_root)? {
            for path in toml_files(&events_root.join(year))? {
                events.push(read_toml(&path)?);
            }
        }

        events.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(events)
    }

    pub fn stock(&self) -> Result<Vec<Stock>> {
        let parts = self.list_parts()?;
        let events = self.list_stock_events()?;

        let mut totals: BTreeMap<String, (i64, usize, Option<DateTime<Utc>>)> = BTreeMap::new();

        for event in &events {
            let entry = totals.entry(event.part.clone()).or_insert((0, 0, None));

            entry.0 += event.delta;
            entry.1 += 1;
            entry.2 = Some(match entry.2 {
                Some(previous) if previous > event.at => previous,
                _ => event.at,
            });
        }

        Ok(parts
            .into_iter()
            .map(|part| {
                let (on_hand, count, last) =
                    totals.get(&part.slug).copied().unwrap_or((0, 0, None));

                Stock {
                    part: part.slug,
                    name: part.name,
                    on_hand,
                    events: count,
                    last_event_at: last,
                }
            })
            .collect())
    }

    pub fn load_experiment(&self, project: &str, slug: &str) -> Result<Experiment> {
        let path = self.experiment_path(project, slug);

        if !path.is_file() {
            bail!("unknown experiment {project:?}/{slug:?}");
        }

        read_toml(&path)
    }

    pub fn create_experiment(&self, experiment: &Experiment) -> Result<PathBuf> {
        let path = self.experiment_path(&experiment.project, &experiment.slug);

        if path.exists() {
            bail!("experiment {} already exists", experiment.reference());
        }

        write_toml(&path, experiment)?;

        Ok(path)
    }

    pub fn save_experiment(&self, experiment: &Experiment) -> Result<PathBuf> {
        let path = self.experiment_path(&experiment.project, &experiment.slug);

        if !path.is_file() {
            bail!("unknown experiment {}", experiment.reference());
        }

        write_toml(&path, experiment)?;

        Ok(path)
    }

    pub fn list_experiments(&self) -> Result<Vec<Experiment>> {
        let root = self.root.join("experiments");
        let mut experiments = Vec::new();

        for project in child_dirs(&root)? {
            for slug in child_dirs(&root.join(&project))? {
                let path = self.experiment_path(&project, &slug);

                if path.is_file() {
                    experiments.push(read_toml(&path)?);
                }
            }
        }

        Ok(experiments)
    }

    pub fn append_measurement(
        &self,
        mut measurement: Measurement,
    ) -> Result<(Measurement, PathBuf)> {
        let at = measurement.at;
        let project = measurement.project.clone();
        let path = self.free_event_path(&mut measurement.id, at, |id| {
            self.measurement_path(&project, id)
        })?;

        write_new_toml(&path, &measurement)?;

        Ok((measurement, path))
    }

    pub fn list_measurements(&self) -> Result<Vec<Measurement>> {
        let root = self.root.join("measurements");
        let mut measurements: Vec<Measurement> = Vec::new();

        for project in child_dirs(&root)? {
            for path in toml_files(&root.join(project))? {
                measurements.push(read_toml(&path)?);
            }
        }

        measurements.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(measurements)
    }
}

/// Where a project is checked out on *this* machine. Kept in XDG state, out
/// of the repository, so project identity never carries an absolute path.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Checkouts {
    #[serde(default)]
    pub paths: BTreeMap<String, String>,
}

impl Checkouts {
    pub fn load() -> Result<Self> {
        let path = paths::checkouts_file();

        if !path.is_file() {
            return Ok(Self::default());
        }

        read_toml(&path)
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = paths::checkouts_file();

        write_toml(&path, self)?;

        Ok(path)
    }
}

pub fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn to_toml<T: Serialize>(value: &T) -> Result<String> {
    toml::to_string_pretty(value).context("serializing to TOML")
}

/// Rename over a temporary file in the same directory: a crash leaves either
/// the old file or the new one, never a half-written record.
pub fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let body = to_toml(value)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let temp = path.with_extension("toml.tmp");

    fs::write(&temp, body).with_context(|| format!("writing {}", temp.display()))?;
    fs::rename(&temp, path).with_context(|| format!("renaming into {}", path.display()))?;

    Ok(())
}

fn write_new_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let mut file = fs::File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("claiming {}", path.display()))?;

    file.write_all(to_toml(value)?.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;

    Ok(())
}

fn child_dirs(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    if !dir.is_dir() {
        return Ok(names);
    }

    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;

        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
            && !name.starts_with('.')
        {
            names.push(name.to_string());
        }
    }

    names.sort();

    Ok(names)
}

fn toml_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.is_dir() {
        return Ok(files);
    }

    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();

        if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }

    files.sort();

    Ok(files)
}
