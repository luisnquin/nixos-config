use clap::{Args, Parser, Subcommand};

use crate::model::{ExperimentStatus, ProjectStatus};

/// The command tree is domain-shaped on purpose: every verb belongs to a
/// bench domain, so nothing lands on a vague root operation.
#[derive(Parser)]
#[command(
    name = "ee",
    version,
    about = "terminal-first personal electronics engineering workbench",
    long_about = "Bare `ee` opens the workbench TUI. Subcommands are the automation surface: \
                  every query takes --json, and the Git repository under \
                  $XDG_DATA_HOME/ee-workbench is the only authority."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Bench projects: the unit everything else hangs off
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Parts and the immutable stock ledger
    Inventory {
        #[command(subcommand)]
        command: InventoryCommand,
    },
    /// Experiments run against a project
    Experiment {
        #[command(subcommand)]
        command: ExperimentCommand,
    },
    /// Recorded readings, immutable once written
    Measurement {
        #[command(subcommand)]
        command: MeasurementCommand,
    },
    /// FreeCAD documents, through the local session
    Mechanical {
        #[command(subcommand)]
        command: MechanicalCommand,
    },
    /// The workbench repository itself
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },
    /// Run git in the workbench repository, transparently
    #[command(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        disable_help_flag = true
    )]
    Git {
        /// Arguments passed to git verbatim
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// Create a project
    New {
        slug: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long, value_enum, default_value = "active")]
        status: ProjectStatus,
    },
    /// List projects
    List {
        #[arg(long, value_enum)]
        status: Option<ProjectStatus>,
        #[command(flatten)]
        format: Format,
    },
    /// Show one project
    Show {
        slug: String,
        #[command(flatten)]
        format: Format,
    },
    /// Map a project to a checkout on this machine only, outside the repository
    Link {
        slug: String,
        /// Defaults to the current directory
        #[arg(long)]
        path: Option<String>,
        /// Drop the mapping instead of setting it
        #[arg(long)]
        remove: bool,
    },
}

#[derive(Subcommand)]
pub enum InventoryCommand {
    /// Part definitions
    Part {
        #[command(subcommand)]
        command: PartCommand,
    },
    /// Record parts arriving
    Receive {
        part: String,
        #[arg(long, default_value_t = 1)]
        qty: u32,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Record parts leaving, optionally against a project
    Consume {
        part: String,
        #[arg(long, default_value_t = 1)]
        qty: u32,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// On-hand quantities, summed from the ledger
    Stock {
        part: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// The raw stock ledger, oldest first
    Events {
        #[arg(long)]
        part: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum PartCommand {
    /// Define a part
    Add {
        slug: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        package: Option<String>,
        /// Manufacturer part number
        #[arg(long)]
        mpn: Option<String>,
        #[arg(long)]
        datasheet: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// List part definitions
    List {
        #[command(flatten)]
        format: Format,
    },
    /// Show one part with its on-hand quantity
    Show {
        slug: String,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum ExperimentCommand {
    /// Create an experiment under a project
    New {
        /// <project>/<slug>
        reference: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        hypothesis: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long, value_enum, default_value = "planned")]
        status: ExperimentStatus,
    },
    /// List experiments
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long, value_enum)]
        status: Option<ExperimentStatus>,
        #[command(flatten)]
        format: Format,
    },
    /// Show one experiment with its measurements
    Show {
        /// <project>/<slug>
        reference: String,
        #[command(flatten)]
        format: Format,
    },
    /// Update the mutable fields of an experiment
    Set {
        /// <project>/<slug>
        reference: String,
        #[arg(long, value_enum)]
        status: Option<ExperimentStatus>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        hypothesis: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MeasurementCommand {
    /// Append a reading to the ledger
    Record {
        #[arg(long)]
        project: String,
        /// What was measured, e.g. ripple, vout, temperature
        #[arg(long)]
        quantity: String,
        #[arg(long, allow_negative_numbers = true)]
        value: f64,
        #[arg(long)]
        unit: String,
        /// Experiment slug inside the same project
        #[arg(long)]
        experiment: Option<String>,
        #[arg(long)]
        instrument: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// List readings, oldest first
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        experiment: Option<String>,
        #[arg(long)]
        quantity: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[command(flatten)]
        format: Format,
    },
    /// Show one reading by event id
    Show {
        id: String,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum MechanicalCommand {
    /// Report the CAD socket and what the FreeCAD session holds. Never starts
    /// one: it is the probe every other verb is allowed to act on.
    Status {
        #[command(flatten)]
        format: Format,
    },
    /// Start or stop the FreeCAD session explicitly
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Documents in the running FreeCAD session
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// PartDesign bodies
    Body {
        #[command(subcommand)]
        command: BodyCommand,
    },
    /// Sketches and their geometry
    Sketch {
        #[command(subcommand)]
        command: SketchCommand,
    },
    /// Pads: the solid a sketch becomes
    Pad {
        #[command(subcommand)]
        command: PadCommand,
    },
    /// Pockets: the material a sketch removes
    Pocket {
        #[command(subcommand)]
        command: PocketCommand,
    },
    /// Take a feature back out of a body's tree, or a sketch out of a document
    Feature {
        #[command(subcommand)]
        command: FeatureCommand,
    },
    /// Named dimensions, readable and settable after the fact
    Param {
        #[command(subcommand)]
        command: ParamCommand,
    },
    /// Export the printable mesh, or render a picture of the model
    Preview {
        #[command(subcommand)]
        command: PreviewCommand,
    },
}

#[derive(Subcommand)]
pub enum SessionCommand {
    /// Start ee-freecad-server if nothing is listening, and wait for it
    Start {
        #[command(flatten)]
        format: Format,
    },
    /// Ask the session to exit. Refuses while a document is unsaved.
    Stop {
        /// Exit anyway, discarding unsaved documents
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum DocumentCommand {
    /// Create a document and make it active
    New {
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Open an existing .FCStd and make it active
    Open {
        #[command(flatten)]
        path: OpenPath,
        #[command(flatten)]
        format: Format,
    },
    /// Recompute the touched objects
    Recompute {
        #[arg(long)]
        document: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Save the document, to its own path or to a new one
    Save {
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Objects, sketch geometry, constraints and degrees of freedom
    Inspect {
        #[arg(long)]
        document: Option<String>,
        /// Also list each body's features in order, with what drives them and
        /// which ones FreeCAD could not build
        #[arg(long)]
        features: bool,
        #[command(flatten)]
        format: Format,
    },
}

/// A numeric slot argument: a literal, or the name of the parameter that
/// drives it. Every verb that sets a dimension takes both forms on every call,
/// so binding a slot to a parameter is an edit rather than a property of how
/// the slot was first created.
#[derive(Clone, Debug)]
pub enum Slot {
    Literal(f64),
    Parameter(String),
}

impl std::str::FromStr for Slot {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Ok(number) = text.parse::<f64>() {
            return Ok(Self::Literal(number));
        }
        if is_parameter_name(text) {
            return Ok(Self::Parameter(text.to_string()));
        }

        Err(format!(
            "{text} is neither a number nor a parameter name: a name starts with a letter or \
             an underscore and holds letters, digits and underscores"
        ))
    }
}

/// What a parameter itself holds. An expression may be written FreeCAD's way,
/// with a leading `=`, or without it; the two spellings mean the same thing and
/// refusing one would only be a spelling test.
#[derive(Clone, Debug)]
pub enum ParamValue {
    Number(f64),
    Expression(String),
}

impl std::str::FromStr for ParamValue {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix('=') {
            return Ok(Self::Expression(rest.trim().to_string()));
        }
        match text.parse::<f64>() {
            Ok(number) => Ok(Self::Number(number)),
            Err(_) if text.is_empty() => Err("a parameter needs a value".to_string()),
            Err(_) => Ok(Self::Expression(text.to_string())),
        }
    }
}

/// The same grammar the server enforces at `param new`. Kept here too so a
/// typo is refused before it becomes a request.
fn is_parameter_name(text: &str) -> bool {
    let mut characters = text.chars();

    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

/// `document open x.FCStd` and `document open --path x.FCStd` name the same
/// file. Save, export and render all spell it as a flag, and an agent writing
/// a script should not have to remember which verb is the odd one out.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct OpenPath {
    #[arg(value_name = "PATH")]
    positional: Option<String>,
    #[arg(long = "path", value_name = "PATH")]
    flag: Option<String>,
}

impl OpenPath {
    pub fn get(self) -> Option<String> {
        self.positional.or(self.flag)
    }
}

#[derive(Subcommand)]
pub enum BodyCommand {
    /// Create a PartDesign body with its origin planes
    New {
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum SketchCommand {
    /// Create a sketch on one of the body's origin planes, optionally offset
    New {
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        body: Option<String>,
        /// xy, xz or yz
        #[arg(long, default_value = "xy")]
        plane: String,
        #[arg(long)]
        name: Option<String>,
        /// Slide the sketch along the plane's own first axis, in millimetres
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        offset_x: Option<Slot>,
        /// Slide the sketch along the plane's own second axis
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        offset_y: Option<Slot>,
        /// Lift the sketch off the plane, along its normal
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        offset_z: Option<Slot>,
        /// Spin the sketch about its own normal, in degrees
        #[arg(long, allow_negative_numbers = true, value_name = "DEG|PARAM")]
        rotate: Option<Slot>,
        #[command(flatten)]
        format: Format,
    },
    /// Draw a fully constrained rectangle
    Rectangle {
        #[arg(long, value_name = "MM|PARAM")]
        width: Slot,
        #[arg(long, value_name = "MM|PARAM")]
        height: Slot,
        #[arg(long)]
        document: Option<String>,
        /// Defaults to the newest sketch, which is the one just drawn on
        #[arg(long)]
        sketch: Option<String>,
        /// Sketch-plane coordinate of the reference point, default 0
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        x: Option<Slot>,
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        y: Option<Slot>,
        /// Treat --x/--y as the centre instead of the lower left corner
        #[arg(long)]
        centered: bool,
        #[command(flatten)]
        format: Format,
    },
    /// Draw a fully constrained circle
    Circle {
        #[arg(long, value_name = "MM|PARAM")]
        radius: Slot,
        #[arg(long)]
        document: Option<String>,
        /// Defaults to the newest sketch, which is the one just drawn on
        #[arg(long)]
        sketch: Option<String>,
        /// Sketch-plane coordinate of the centre, default 0
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        x: Option<Slot>,
        #[arg(long, allow_negative_numbers = true, value_name = "MM|PARAM")]
        y: Option<Slot>,
        #[command(flatten)]
        format: Format,
    },
    /// Point one of a sketch's dimensions at a parameter, or back at a number
    Set {
        /// width, height, radius, x, y, offset_x, offset_y, offset_z or rotate
        slot: String,
        #[arg(allow_negative_numbers = true, value_name = "MM|PARAM")]
        value: Slot,
        #[arg(long)]
        document: Option<String>,
        /// Defaults to the newest sketch
        #[arg(long)]
        sketch: Option<String>,
        /// Replace a parameter with a literal, which no other spelling will do
        #[arg(long)]
        unbind: bool,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum PadCommand {
    /// Pad a sketch into a solid inside its body
    New {
        #[arg(long, value_name = "MM|PARAM")]
        length: Slot,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        body: Option<String>,
        /// Defaults to the newest sketch, which is the one just drawn on
        #[arg(long)]
        sketch: Option<String>,
        /// Grow symmetrically about the sketch plane
        #[arg(long)]
        midplane: bool,
        /// Grow against the sketch normal
        #[arg(long)]
        reversed: bool,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Change the length of an existing pad and recompute
    Length {
        #[arg(value_name = "MM|PARAM")]
        length: Slot,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        pad: Option<String>,
        /// Replace a parameter with a literal, which no other spelling will do
        #[arg(long)]
        unbind: bool,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum PocketCommand {
    /// Cut a sketch into the body's existing material
    New {
        /// Depth of the cut; ignored with --through-all
        #[arg(long, default_value = "0", value_name = "MM|PARAM")]
        length: Slot,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        body: Option<String>,
        /// Defaults to the newest sketch, which is the one just drawn on
        #[arg(long)]
        sketch: Option<String>,
        /// Cut all the way through whatever is in the way
        #[arg(long)]
        through_all: bool,
        /// Cut symmetrically about the sketch plane
        #[arg(long)]
        midplane: bool,
        /// Cut along the sketch normal instead of against it
        #[arg(long)]
        reversed: bool,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Change the depth of an existing pocket and recompute
    Length {
        #[arg(value_name = "MM|PARAM")]
        length: Slot,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        pocket: Option<String>,
        /// Replace a parameter with a literal, which no other spelling will do
        #[arg(long)]
        unbind: bool,
        #[command(flatten)]
        format: Format,
    },
}

/// The only verb that makes the model smaller. Everything else adds, so this
/// is where the tree's own links have to be repaired by hand: FreeCAD clears a
/// dangling BaseFeature rather than healing it, and a body whose chain has a
/// hole in it rebuilds to the material below the hole and calls itself
/// up-to-date.
#[derive(Subcommand)]
pub enum FeatureCommand {
    /// Remove a pad, a pocket or a widowed sketch, repairing what pointed at it
    Remove {
        /// Named, never inferred: this is the one verb that cannot be undone
        feature: String,
        #[arg(long)]
        document: Option<String>,
        /// Report what this would change and change nothing
        #[arg(long)]
        dry_run: bool,
        #[command(flatten)]
        format: Format,
    },
}

/// Parameters are the document's only arithmetic. Geometry reads them and
/// never computes: a slot holds a number or the name of one parameter, so
/// `param list` is a complete description of what can move rather than an
/// index of where to look for it.
#[derive(Subcommand)]
pub enum ParamCommand {
    /// Declare a parameter, as a number or an expression over the others
    New {
        name: String,
        #[arg(allow_negative_numbers = true, value_name = "VALUE|EXPR")]
        value: ParamValue,
        #[arg(long)]
        document: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Drive a parameter to a new value and recompute everything it reaches
    Set {
        name: String,
        #[arg(allow_negative_numbers = true, value_name = "VALUE|EXPR")]
        value: ParamValue,
        #[arg(long)]
        document: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Every parameter, what it computes to, and which slots follow it
    List {
        #[arg(long)]
        document: Option<String>,
        #[command(flatten)]
        format: Format,
    },
    /// Remove a parameter; --force freezes every slot it drives at its value
    Remove {
        name: String,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum PreviewCommand {
    /// Tessellate the solid to STL and keep it in step with later edits
    Export {
        #[arg(long)]
        document: Option<String>,
        /// Object to mesh; defaults to the document's only solid
        #[arg(long)]
        object: Option<String>,
        /// Defaults to $XDG_CACHE_HOME/ee-workbench/preview/<document>.stl
        #[arg(long)]
        path: Option<String>,
        /// Maximum linear deviation of the mesh, in millimetres
        #[arg(long)]
        deflection: Option<f64>,
        /// Maximum angular deviation of the mesh, in radians
        #[arg(long)]
        angular: Option<f64>,
        /// Stop re-exporting this document after every successful recompute
        #[arg(long)]
        once: bool,
        #[command(flatten)]
        format: Format,
    },
    /// Rasterize the model to a PNG, offscreen, with no GUI involved
    Render {
        #[arg(long)]
        document: Option<String>,
        /// Object to draw; defaults to every top level solid
        #[arg(long)]
        object: Option<String>,
        /// Defaults to $XDG_CACHE_HOME/ee-workbench/preview/<document>-<view>.png
        #[arg(long)]
        path: Option<String>,
        /// iso, front, back, left, right, top or bottom
        #[arg(long, default_value = "iso")]
        view: String,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        /// Maximum linear deviation of the mesh, in millimetres
        #[arg(long)]
        deflection: Option<f64>,
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Subcommand)]
pub enum RepoCommand {
    /// Create the layout and an empty Git repository. Never commits.
    Init {
        #[arg(long)]
        path: Option<String>,
    },
    /// Counts, Git branch and the uncommitted paths
    Status {
        #[command(flatten)]
        format: Format,
    },
    /// Print the data root
    Path,
    /// Parse every record and report broken references
    Check {
        #[command(flatten)]
        format: Format,
    },
}

#[derive(Args, Clone, Copy)]
pub struct Format {
    /// Emit stable JSON instead of the human table
    #[arg(long)]
    pub json: bool,
}
