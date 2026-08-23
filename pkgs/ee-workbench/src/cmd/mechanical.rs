use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::bridge;
use crate::cli::{
    BodyCommand, DocumentCommand, Format, MechanicalCommand, PadCommand, ParamCommand,
    PocketCommand, PreviewCommand, SessionCommand, SketchCommand,
};
use crate::cmd;
use crate::paths;
use crate::spawn;

pub fn run(command: MechanicalCommand) -> Result<i32> {
    match command {
        MechanicalCommand::Status { format } => status(format),
        MechanicalCommand::Session { command } => session(command),
        MechanicalCommand::Document { command } => document(command),
        MechanicalCommand::Body { command } => body(command),
        MechanicalCommand::Sketch { command } => sketch(command),
        MechanicalCommand::Pad { command } => pad(command),
        MechanicalCommand::Pocket { command } => pocket(command),
        MechanicalCommand::Param { command } => param(command),
        MechanicalCommand::Preview { command } => preview(command),
    }
}

/// Optional arguments are omitted rather than sent as null, so the server's
/// "not given" and "given as empty" stay distinguishable.
fn params(entries: Vec<(&str, Option<Value>)>) -> Value {
    let mut out = Map::new();

    for (key, value) in entries {
        if let Some(value) = value {
            out.insert(key.to_string(), value);
        }
    }

    Value::Object(out)
}

fn text(value: Option<String>) -> Option<Value> {
    value.map(Value::from)
}

/// Every path in a request goes through here. The server writes files from its
/// own working directory, which is whichever one the session was started from
/// and is frozen for its lifetime, so a relative path must be resolved against
/// the caller instead. The server rejects anything still relative.
fn resolved_path(value: Option<String>) -> Result<Option<Value>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let resolved = paths::absolute(&value)?;
    let text = resolved.into_os_string().into_string().map_err(|raw| {
        anyhow!(
            "{} is not valid UTF-8 and the cad protocol is JSON",
            std::path::Path::new(&raw).display()
        )
    })?;

    Ok(Some(Value::from(text)))
}

/// Every verb but `status` starts a session if none is listening. The document
/// graph lives in that process, so the alternative is a tool that only works
/// when somebody else remembered to run a daemon.
fn call(method: &str, params: Value) -> Result<Value> {
    let socket = paths::cad_socket();
    spawn::ensure(&socket)?;

    match bridge::call(&socket, method, params.clone()) {
        // `ensure` probes by connecting, and a connection is accepted into the
        // listen backlog by a server that is one instruction from retiring on
        // its idle deadline. The session that would have answered is gone, so
        // its documents are gone with it: starting a fresh one and replaying
        // once cannot double anything that survived.
        Err(error) if bridge::is_disconnect(&error) => {
            spawn::ensure(&socket)?;
            bridge::call(&socket, method, params)
        }
        outcome => outcome,
    }
}

/// Only `session start` needs both halves of that retry - whether it started
/// anything, and the status of what is running now.
fn ensure_then_status(socket: &std::path::Path) -> Result<(spawn::Started, Value)> {
    let mut started = spawn::ensure(socket)?;
    let mut status = bridge::call(socket, "session.status", json!({}));

    if status.as_ref().err().is_some_and(bridge::is_disconnect) {
        started = spawn::ensure(socket)?;
        status = bridge::call(socket, "session.status", json!({}));
    }

    Ok((started, status?))
}

fn number(value: Option<f64>) -> Option<Value> {
    value.map(|value| json!(value))
}

/// Every mutating command answers with the server's own reply: the CLI never
/// paraphrases what FreeCAD reports.
fn emit(result: &Value, format: Format, summary: impl FnOnce(&Value)) -> Result<i32> {
    if format.json {
        cmd::emit_json(result)?;
    } else {
        summary(result);
    }

    Ok(0)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("-")
}

fn status(format: Format) -> Result<i32> {
    let socket = paths::cad_socket();

    let session = bridge::call(&socket, "session.status", json!({}));

    let report = match &session {
        Ok(result) => json!({
            "socket": socket.display().to_string(),
            "protocol": bridge::PROTOCOL,
            "running": true,
            "session": result,
        }),
        Err(error) => json!({
            "socket": socket.display().to_string(),
            "protocol": bridge::PROTOCOL,
            "running": false,
            "error": format!("{error:#}"),
        }),
    };

    if format.json {
        cmd::emit_json(&report)?;

        return Ok(0);
    }

    println!("socket   {}", socket.display());
    println!("protocol {}", bridge::PROTOCOL);

    match session {
        Ok(result) => {
            println!("session  running");
            println!("freecad  {}", field(&result["freecad"], "version"));
            println!("idle     {}s", result["idle"]["timeout"]);

            let unsaved = result["unsaved"].as_array().cloned().unwrap_or_default();
            if !unsaved.is_empty() {
                let names: Vec<&str> = unsaved.iter().filter_map(Value::as_str).collect();
                println!("unsaved  {}", names.join(", "));
            }
            println!(
                "active   {}",
                result
                    .get("active")
                    .and_then(Value::as_str)
                    .unwrap_or("none")
            );

            let rows: Vec<Vec<String>> = result["documents"]
                .as_array()
                .map(|documents| {
                    documents
                        .iter()
                        .map(|document| {
                            vec![
                                field(document, "document").to_string(),
                                field(document, "label").to_string(),
                                document["objects"].to_string(),
                                field(document, "file").to_string(),
                            ]
                        })
                        .collect()
                })
                .unwrap_or_default();

            if !rows.is_empty() {
                println!();
                cmd::print_table(&["document", "label", "objects", "file"], &rows);
            }
        }
        Err(_) => println!("session  not running (any other verb starts one)"),
    }

    Ok(0)
}

fn session(command: SessionCommand) -> Result<i32> {
    let socket = paths::cad_socket();

    match command {
        SessionCommand::Start { format } => {
            let (started, result) = ensure_then_status(&socket)?;

            let report = json!({
                "socket": socket.display().to_string(),
                "started": matches!(started, spawn::Started::Spawned),
                "log": spawn::log_for(&socket).display().to_string(),
                "session": result,
            });

            emit(&report, format, |report| {
                println!("socket  {}", field(report, "socket"));
                println!(
                    "session {}",
                    if report["started"] == json!(true) {
                        "started"
                    } else {
                        "already running"
                    }
                );
                println!("log     {}", field(report, "log"));
            })
        }
        SessionCommand::Stop { force, format } => {
            let status = bridge::call(&socket, "session.status", json!({}));
            let Ok(status) = status else {
                let report = json!({ "stopped": false, "running": false });

                return emit(&report, format, |_| println!("session not running"));
            };

            let unsaved: Vec<String> = status["unsaved"]
                .as_array()
                .map(|names| {
                    names
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();

            if !unsaved.is_empty() && !force {
                anyhow::bail!(
                    "{} has unsaved changes: save it, or stop with --force to discard it",
                    unsaved.join(", ")
                );
            }

            bridge::call(&socket, "server.shutdown", json!({}))?;

            let report = json!({ "stopped": true, "running": false, "discarded": unsaved });

            emit(&report, format, |report| {
                println!("session stopped");
                if let Some(discarded) = report["discarded"].as_array()
                    && !discarded.is_empty()
                {
                    println!("discarded {}", discarded.len());
                }
            })
        }
    }
}

fn document(command: DocumentCommand) -> Result<i32> {
    match command {
        DocumentCommand::New { name, format } => {
            let result = call("document.new", params(vec![("name", text(name))]))?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
            })
        }
        DocumentCommand::Open {
            path: target,
            format,
        } => {
            let result = call("document.open", params(vec![("path", resolved_path(target.get())?)]))?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("objects  {}", result["objects"]);
            })
        }
        DocumentCommand::Recompute { document, format } => {
            let result = call(
                "document.recompute",
                params(vec![("document", text(document))]),
            )?;

            emit(&result, format, |result| {
                println!("document   {}", field(result, "document"));
                println!("recomputed {}", result["recomputed"]);
                println!("failed     {}", result["failed"]);
            })
        }
        DocumentCommand::Save {
            document,
            path,
            format,
        } => {
            let result = call(
                "document.save",
                params(vec![("document", text(document)), ("path", resolved_path(path)?)]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("file     {}", field(result, "file"));
            })
        }
        DocumentCommand::Inspect { document, format } => {
            let result = call(
                "document.inspect",
                params(vec![("document", text(document))]),
            )?;

            emit(&result, format, inspect_summary)
        }
    }
}

fn inspect_summary(result: &Value) {
    println!("document {}", field(result, "document"));
    println!("file     {}", field(result, "file"));

    let objects = result["objects"].as_array().cloned().unwrap_or_default();

    let rows: Vec<Vec<String>> = objects
        .iter()
        .map(|object| {
            let detail = match object.get("sketch") {
                Some(sketch) => format!(
                    "{} geometry, {} constraints, dof {}",
                    sketch["geometry"].as_array().map_or(0, Vec::len),
                    sketch["constraints"].as_array().map_or(0, Vec::len),
                    sketch["dof"],
                ),
                None => "-".to_string(),
            };

            vec![
                field(object, "name").to_string(),
                field(object, "type").to_string(),
                field(object, "label").to_string(),
                detail,
            ]
        })
        .collect();

    if !rows.is_empty() {
        println!();
        cmd::print_table(&["object", "type", "label", "detail"], &rows);
    }

    let solids = result["solids"].as_array().cloned().unwrap_or_default();
    for solid in &solids {
        println!();
        println!("solid  {}", field(solid, "name"));

        let shape = &solid["shape"];
        println!(
            "size   {} x {} x {}",
            shape["size"]["x"], shape["size"]["y"], shape["size"]["z"]
        );
        shape_summary(shape, "");
    }

    if solids.len() > 1 {
        println!();
        println!("overall");
        shape_summary(&result["bbox"], "");
    }
}

fn flag(value: bool) -> Option<Value> {
    value.then(|| Value::from(true))
}

fn body(command: BodyCommand) -> Result<i32> {
    match command {
        BodyCommand::New {
            document,
            name,
            format,
        } => {
            let result = call(
                "body.new",
                params(vec![("document", text(document)), ("name", text(name))]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("body     {}", field(result, "body"));
            })
        }
    }
}

fn sketch(command: SketchCommand) -> Result<i32> {
    match command {
        SketchCommand::New {
            document,
            body,
            plane,
            name,
            offset_x,
            offset_y,
            offset_z,
            rotate,
            format,
        } => {
            let result = call(
                "sketch.new",
                params(vec![
                    ("document", text(document)),
                    ("body", text(body)),
                    ("plane", Some(Value::from(plane))),
                    ("name", text(name)),
                    ("offset_x", number(offset_x)),
                    ("offset_y", number(offset_y)),
                    ("offset_z", number(offset_z)),
                    ("rotate", number(rotate)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("body     {}", field(result, "body"));
                println!("sketch   {}", field(result, "sketch"));
                println!("plane    {}", field(result, "plane"));
                basis_summary(&result["basis"]);
            })
        }
        SketchCommand::Rectangle {
            width,
            height,
            document,
            sketch,
            x,
            y,
            centered,
            name_width,
            name_height,
            format,
        } => {
            let result = call(
                "sketch.rectangle",
                params(vec![
                    ("document", text(document)),
                    ("sketch", text(sketch)),
                    ("width", Some(json!(width))),
                    ("height", Some(json!(height))),
                    ("x", number(x)),
                    ("y", number(y)),
                    ("centered", flag(centered)),
                    ("name_width", text(name_width)),
                    ("name_height", text(name_height)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("sketch      {}", field(result, "sketch"));
                println!("width       {}", result["width"]);
                println!("height      {}", result["height"]);
                println!(
                    "corner      {} {}",
                    result["corner"]["x"], result["corner"]["y"]
                );
                sketch_summary(result);
            })
        }
        SketchCommand::Circle {
            radius,
            document,
            sketch,
            x,
            y,
            name_radius,
            format,
        } => {
            let result = call(
                "sketch.circle",
                params(vec![
                    ("document", text(document)),
                    ("sketch", text(sketch)),
                    ("radius", Some(json!(radius))),
                    ("x", number(x)),
                    ("y", number(y)),
                    ("name_radius", text(name_radius)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("sketch      {}", field(result, "sketch"));
                println!("radius      {}", result["radius"]);
                println!(
                    "centre      {} {}",
                    result["centre"]["x"], result["centre"]["y"]
                );
                sketch_summary(result);
            })
        }
    }
}

/// The plane's global axes. Without them the caller cannot tell which way a
/// sketch on xz grows, and only finds out from a solid in the wrong octant.
fn basis_summary(basis: &Value) {
    let axis = |name: &str| {
        let value = &basis[name];

        format!("{} {} {}", value["x"], value["y"], value["z"])
    };

    println!("origin   {}", axis("origin"));
    println!("u        {}", axis("x"));
    println!("v        {}", axis("y"));
    println!("normal   {}", axis("normal"));
}

fn sketch_summary(result: &Value) {
    println!(
        "geometry    {}",
        result["geometry"].as_array().map_or(0, Vec::len)
    );
    println!(
        "constraints {}",
        result["constraints"].as_array().map_or(0, Vec::len)
    );
    println!("dof         {}", result["dof"]);
}

fn pad(command: PadCommand) -> Result<i32> {
    match command {
        PadCommand::New {
            length,
            document,
            body,
            sketch,
            midplane,
            reversed,
            name,
            format,
        } => {
            let result = call(
                "pad.new",
                params(vec![
                    ("document", text(document)),
                    ("body", text(body)),
                    ("sketch", text(sketch)),
                    ("length", Some(json!(length))),
                    ("midplane", flag(midplane)),
                    ("reversed", flag(reversed)),
                    ("name", text(name)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pad    {}", field(result, "pad"));
                println!("length {}", result["length"]);
                println!("solid  {}", result["solid"]);
                bounds_summary(result);
            })
        }
        PadCommand::Length {
            length,
            document,
            pad,
            format,
        } => {
            let result = call(
                "pad.length",
                params(vec![
                    ("document", text(document)),
                    ("pad", text(pad)),
                    ("length", Some(json!(length))),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pad      {}", field(result, "pad"));
                println!("length   {}", result["length"]);
                println!("previous {}", result["previous"]);
                bounds_summary(result);
            })
        }
    }
}

/// A size alone cannot tell two layouts apart, so the box is printed with the
/// corner it starts at and the volume it encloses.
fn bounds_summary(result: &Value) {
    let bounds = &result["bounds"];

    println!("bounds {} x {} x {}", bounds["x"], bounds["y"], bounds["z"]);
    shape_summary(&result["shape"], "");
}

fn shape_summary(shape: &Value, indent: &str) {
    if !shape.is_object() {
        return;
    }

    let corner = |name: &str| {
        let value = &shape[name];

        format!("{} {} {}", value["x"], value["y"], value["z"])
    };

    println!("{indent}min    {}", corner("min"));
    println!("{indent}max    {}", corner("max"));
    if shape.get("volume").is_some() {
        println!("{indent}volume {}", shape["volume"]);
        println!("{indent}centre {}", corner("centre_of_mass"));
    }
}

fn pocket(command: PocketCommand) -> Result<i32> {
    match command {
        PocketCommand::New {
            length,
            document,
            body,
            sketch,
            through_all,
            midplane,
            reversed,
            name,
            format,
        } => {
            let result = call(
                "pocket.new",
                params(vec![
                    ("document", text(document)),
                    ("body", text(body)),
                    ("sketch", text(sketch)),
                    ("length", Some(json!(length))),
                    ("through_all", flag(through_all)),
                    ("midplane", flag(midplane)),
                    ("reversed", flag(reversed)),
                    ("name", text(name)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pocket {}", field(result, "pocket"));
                println!("length {}", result["length"]);
                println!("solid  {}", result["solid"]);
                bounds_summary(result);
            })
        }
        PocketCommand::Length {
            length,
            document,
            pocket,
            format,
        } => {
            let result = call(
                "pocket.length",
                params(vec![
                    ("document", text(document)),
                    ("pocket", text(pocket)),
                    ("length", Some(json!(length))),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pocket   {}", field(result, "pocket"));
                println!("length   {}", result["length"]);
                println!("previous {}", result["previous"]);
                bounds_summary(result);
            })
        }
    }
}

fn param(command: ParamCommand) -> Result<i32> {
    match command {
        ParamCommand::List { document, format } => {
            let result = call("param.list", params(vec![("document", text(document))]))?;

            emit(&result, format, |result| {
                let rows: Vec<Vec<String>> = result["parameters"]
                    .as_array()
                    .map(|parameters| {
                        parameters
                            .iter()
                            .map(|parameter| {
                                vec![
                                    field(parameter, "name").to_string(),
                                    parameter["value"].to_string(),
                                    field(parameter, "type").to_string(),
                                    field(parameter, "sketch").to_string(),
                                ]
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if rows.is_empty() {
                    println!("no named dimensions");
                } else {
                    cmd::print_table(&["name", "value", "type", "sketch"], &rows);
                }
            })
        }
        ParamCommand::Set {
            name,
            value,
            document,
            format,
        } => {
            let result = call(
                "param.set",
                params(vec![
                    ("document", text(document)),
                    ("name", Some(Value::from(name))),
                    ("value", Some(json!(value))),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("name     {}", field(result, "name"));
                println!("value    {}", result["value"]);
                println!("previous {}", result["previous"]);
                println!("dof      {}", result["dof"]);
            })
        }
    }
}

fn preview(command: PreviewCommand) -> Result<i32> {
    match command {
        PreviewCommand::Export {
            document,
            object,
            path,
            deflection,
            angular,
            once,
            format,
        } => {
            let result = call(
                "preview.export",
                params(vec![
                    ("document", text(document)),
                    ("object", text(object)),
                    ("path", resolved_path(path)?),
                    ("deflection", deflection.map(|value| json!(value))),
                    ("angular", angular.map(|value| json!(value))),
                    ("follow", once.then(|| Value::from(false))),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("object    {}", field(result, "object"));
                println!("path      {}", field(result, "path"));
                println!("triangles {}", result["triangles"]);
                println!("follow    {}", result["follow"]);
            })
        }
        PreviewCommand::Render {
            document,
            object,
            path,
            view,
            width,
            height,
            deflection,
            format,
        } => {
            let result = call(
                "preview.render",
                params(vec![
                    ("document", text(document)),
                    ("object", text(object)),
                    ("path", resolved_path(path)?),
                    ("view", Some(Value::from(view))),
                    ("width", width.map(|value| json!(value))),
                    ("height", height.map(|value| json!(value))),
                    ("deflection", number(deflection)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("path      {}", field(result, "path"));
                println!("view      {}", field(result, "view"));
                println!("size      {} x {}", result["width"], result["height"]);
                println!("mm/pixel  {}", result["mm_per_pixel"]);
                println!("triangles {}", result["triangles"]);
                shape_summary(&result["bbox"], "");
            })
        }
    }
}
