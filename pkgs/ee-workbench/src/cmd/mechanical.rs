use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::bridge;
use crate::cli::{
    BodyCommand, DocumentCommand, Format, MechanicalCommand, PadCommand, PreviewCommand,
    SketchCommand,
};
use crate::cmd;
use crate::paths;

pub fn run(command: MechanicalCommand) -> Result<i32> {
    match command {
        MechanicalCommand::Status { format } => status(format),
        MechanicalCommand::Document { command } => document(command),
        MechanicalCommand::Body { command } => body(command),
        MechanicalCommand::Sketch { command } => sketch(command),
        MechanicalCommand::Pad { command } => pad(command),
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

fn call(method: &str, params: Value) -> Result<Value> {
    bridge::call(&paths::cad_socket(), method, params)
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
                                field(document, "name").to_string(),
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
        Err(_) => println!("session  not running (start ee-freecad-server)"),
    }

    Ok(0)
}

fn document(command: DocumentCommand) -> Result<i32> {
    match command {
        DocumentCommand::New { name, format } => {
            let result = call("document.new", params(vec![("name", text(name))]))?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "name"));
            })
        }
        DocumentCommand::Open { path, format } => {
            let result = call("document.open", json!({ "path": path }))?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "name"));
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
                params(vec![("document", text(document)), ("path", text(path))]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "name"));
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
    println!("document {}", field(result, "name"));
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
            format,
        } => {
            let result = call(
                "sketch.new",
                params(vec![
                    ("document", text(document)),
                    ("body", text(body)),
                    ("plane", Some(Value::from(plane))),
                    ("name", text(name)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("body     {}", field(result, "body"));
                println!("sketch   {}", field(result, "sketch"));
                println!("plane    {}", field(result, "plane"));
            })
        }
        SketchCommand::Rectangle {
            width,
            height,
            document,
            sketch,
            format,
        } => {
            let result = call(
                "sketch.rectangle",
                params(vec![
                    ("document", text(document)),
                    ("sketch", text(sketch)),
                    ("width", Some(json!(width))),
                    ("height", Some(json!(height))),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("sketch      {}", field(result, "sketch"));
                println!("width       {}", result["width"]);
                println!("height      {}", result["height"]);
                println!(
                    "geometry    {}",
                    result["geometry"].as_array().map_or(0, Vec::len)
                );
                println!(
                    "constraints {}",
                    result["constraints"].as_array().map_or(0, Vec::len)
                );
                println!("dof         {}", result["dof"]);
            })
        }
    }
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

fn bounds_summary(result: &Value) {
    let bounds = &result["bounds"];

    println!("bounds {} x {} x {}", bounds["x"], bounds["y"], bounds["z"]);
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
                    ("path", text(path)),
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
    }
}
