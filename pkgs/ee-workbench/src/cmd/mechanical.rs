use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::bridge;
use crate::cli::{
    BodyCommand, DocumentCommand, Format, MechanicalCommand, PadCommand, ParamCommand, ParamValue,
    PocketCommand, PreviewCommand, SessionCommand, SketchCommand, Slot,
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

/// A slot crosses the wire as its own JSON type: a number is a literal, a
/// string is the parameter that drives it. Nothing else marks the difference,
/// which is why the two can be swapped on any call.
fn slot(value: Slot) -> Option<Value> {
    Some(match value {
        Slot::Literal(number) => json!(number),
        Slot::Parameter(name) => Value::from(name),
    })
}

fn slot_opt(value: Option<Slot>) -> Option<Value> {
    value.and_then(slot)
}

fn param_value(value: ParamValue) -> Vec<(&'static str, Option<Value>)> {
    match value {
        ParamValue::Number(number) => vec![("value", Some(json!(number)))],
        ParamValue::Expression(text) => vec![("expression", Some(Value::from(text)))],
    }
}

/// A slot read back is a number plus whatever computes it, and printing only
/// the number would say the model is fine when what it means is that this one
/// value happens to be right today.
fn slot_text(value: &Value) -> String {
    let number = value
        .get("value")
        .map_or_else(|| value.to_string(), Value::to_string);

    if let Some(parameter) = value.get("parameter").and_then(Value::as_str) {
        return format!("{number} <- {parameter}");
    }
    match value.get("expression").and_then(Value::as_str) {
        Some(expression) => format!("{number} <- ={expression}"),
        None => number,
    }
}

/// True when the reply says FreeCAD could not build something. Read from
/// either shape, because `document recompute` answers with the recompute
/// itself and every other verb nests it.
fn broke(result: &Value) -> bool {
    result
        .pointer("/recompute/failed")
        .or_else(|| result.get("failed"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn failures(result: &Value) -> &[Value] {
    result
        .pointer("/recompute/errors")
        .or_else(|| result.get("errors"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Driving one parameter can push six features outside their material at once,
/// and the geometry cannot say so on its own. The exit status carries it to a
/// shell that would otherwise read a printed bounding box as success.
fn emit(result: &Value, format: Format, summary: impl FnOnce(&Value)) -> Result<i32> {
    if format.json {
        cmd::emit_json(result)?;
    } else {
        summary(result);
        broken_summary(result);
    }

    Ok(i32::from(broke(result)))
}

fn broken_summary(result: &Value) {
    let errors = failures(result);
    if errors.is_empty() {
        return;
    }

    let rows: Vec<Vec<String>> = errors
        .iter()
        .map(|error| {
            vec![
                field(error, "object").to_string(),
                field(error, "label").to_string(),
                field(error, "status").to_string(),
            ]
        })
        .collect();

    println!();
    println!("{} feature(s) did not build", rows.len());
    cmd::print_table(&["feature", "label", "why"], &rows);
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("-")
}

/// `session.status` answers across a build mismatch on purpose, so this is the
/// one verb that can name the drift instead of refusing over it.
fn build_report(session: &Value) -> Value {
    let running = session.get("build").and_then(Value::as_str).unwrap_or("");
    let expected = spawn::expected_build();
    // A server that names no build is not an unknown, it is one that predates
    // the field: older than anything this `ee` was paired with.
    let stale = expected.as_deref().is_some_and(|want| running != want);

    json!({ "running": running, "expected": expected, "stale": stale })
}

fn status(format: Format) -> Result<i32> {
    let socket = paths::cad_socket();

    let session = bridge::call(&socket, "session.status", json!({}));

    let report = match &session {
        Ok(result) => json!({
            "socket": socket.display().to_string(),
            "protocol": bridge::PROTOCOL,
            "running": true,
            "build": build_report(result),
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
            let build = build_report(&result);
            if build["stale"] == json!(true) {
                let running = field(&build, "running");
                let named = if running.is_empty() {
                    "unnamed, older than this check"
                } else {
                    running
                };
                println!("build    STALE {named}");
                println!("         want  {}", field(&build, "expected"));
            }
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
            let result = call(
                "document.open",
                params(vec![("path", resolved_path(target.get())?)]),
            )?;

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
                params(vec![
                    ("document", text(document)),
                    ("path", resolved_path(path)?),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("file     {}", field(result, "file"));
            })
        }
        DocumentCommand::Inspect {
            document,
            features,
            format,
        } => {
            let result = call(
                "document.inspect",
                params(vec![
                    ("document", text(document)),
                    ("features", flag(features)),
                ]),
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
                object
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            ]
        })
        .collect();

    if !rows.is_empty() {
        println!();
        cmd::print_table(&["object", "type", "label", "detail", "error"], &rows);
    }

    features_summary(result);

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

/// The build order, which the object list cannot show: it is sorted by
/// creation and says nothing about which sketch a pad consumed or where that
/// sketch sits. Only printed when it was asked for.
fn features_summary(result: &Value) {
    let Some(bodies) = result.get("bodies").and_then(Value::as_array) else {
        return;
    };

    for body in bodies {
        println!();
        println!("body   {}", field(body, "body"));

        let rows: Vec<Vec<String>> = body["features"]
            .as_array()
            .map(|features| {
                features
                    .iter()
                    .map(|feature| {
                        let sketch = &feature["sketch"];
                        vec![
                            field(feature, "name").to_string(),
                            feature
                                .get("kind")
                                .and_then(Value::as_str)
                                .unwrap_or_else(|| field(feature, "type"))
                                .to_string(),
                            feature.get("length").map_or_else(
                                || "-".to_string(),
                                |length| {
                                    if feature["through_all"] == json!(true) {
                                        "through all".to_string()
                                    } else {
                                        slot_text(length)
                                    }
                                },
                            ),
                            field(sketch, "name").to_string(),
                            field(sketch, "plane").to_string(),
                            sketch
                                .get("offset")
                                .map_or_else(|| "-".to_string(), offset_text),
                            feature
                                .get("error")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default();

        if rows.is_empty() {
            println!("no features");
            continue;
        }

        cmd::print_table(
            &[
                "feature", "kind", "length", "sketch", "plane", "offset", "error",
            ],
            &rows,
        );

        for feature in body["features"].as_array().unwrap_or(&Vec::new()) {
            let dimensions = feature
                .pointer("/sketch/dimensions")
                .and_then(Value::as_array)
                .filter(|dimensions| !dimensions.is_empty());
            let Some(dimensions) = dimensions else {
                continue;
            };

            println!();
            println!(
                "{} dimensions of {}",
                field(feature, "name"),
                field(&feature["sketch"], "name")
            );
            let rows: Vec<Vec<String>> = dimensions
                .iter()
                .map(|dimension| {
                    vec![
                        field(dimension, "slot").to_string(),
                        field(dimension, "type").to_string(),
                        slot_text(dimension),
                    ]
                })
                .collect();
            cmd::print_table(&["slot", "type", "value"], &rows);
        }
    }
}

fn offset_text(offset: &Value) -> String {
    ["x", "y", "z", "rotate"]
        .iter()
        .filter_map(|axis| {
            let slot = offset.get(axis)?;
            let driven = slot.get("parameter").and_then(Value::as_str).is_some()
                || slot.get("expression").and_then(Value::as_str).is_some();
            // `0` and `0.0` are different JSON numbers and the same offset, so
            // the comparison has to happen after the type is gone.
            let moved = slot["value"].as_f64().is_none_or(|value| value != 0.0);
            (driven || moved).then(|| format!("{axis} {}", slot_text(slot)))
        })
        .collect::<Vec<_>>()
        .join(", ")
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
                    ("offset_x", slot_opt(offset_x)),
                    ("offset_y", slot_opt(offset_y)),
                    ("offset_z", slot_opt(offset_z)),
                    ("rotate", slot_opt(rotate)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("document {}", field(result, "document"));
                println!("body     {}", field(result, "body"));
                println!("sketch   {}", field(result, "sketch"));
                println!("plane    {}", field(result, "plane"));
                println!("offset   {}", offset_text(&result["offset"]));
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
            format,
        } => {
            let result = call(
                "sketch.rectangle",
                params(vec![
                    ("document", text(document)),
                    ("sketch", text(sketch)),
                    ("width", slot(width)),
                    ("height", slot(height)),
                    ("x", slot_opt(x)),
                    ("y", slot_opt(y)),
                    ("centered", flag(centered)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("sketch      {}", field(result, "sketch"));
                println!("width       {}", slot_text(&result["width"]));
                println!("height      {}", slot_text(&result["height"]));
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
            format,
        } => {
            let result = call(
                "sketch.circle",
                params(vec![
                    ("document", text(document)),
                    ("sketch", text(sketch)),
                    ("radius", slot(radius)),
                    ("x", slot_opt(x)),
                    ("y", slot_opt(y)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("sketch      {}", field(result, "sketch"));
                println!("radius      {}", slot_text(&result["radius"]));
                println!(
                    "centre      {} {}",
                    result["centre"]["x"], result["centre"]["y"]
                );
                sketch_summary(result);
            })
        }
        SketchCommand::Set {
            slot: name,
            value,
            document,
            sketch,
            unbind,
            format,
        } => set_slot("sketch", name, value, document, sketch, unbind, format),
    }
}

/// Pad length, pocket depth and every sketch dimension are the same operation
/// on a different object, so they are the same request. `kind` is how the
/// server resolves an unnamed one.
fn set_slot(
    kind: &str,
    name: String,
    value: Slot,
    document: Option<String>,
    object: Option<String>,
    unbind: bool,
    format: Format,
) -> Result<i32> {
    let result = call(
        "slot.set",
        params(vec![
            ("document", text(document)),
            ("object", text(object)),
            ("kind", Some(Value::from(kind))),
            ("slot", Some(Value::from(name))),
            ("value", slot(value)),
            ("unbind", flag(unbind)),
        ]),
    )?;

    emit(&result, format, |result| {
        println!("object   {}", field(result, "object"));
        println!("slot     {}", field(result, "slot"));
        println!("value    {}", slot_text(&result["value"]));
        println!("previous {}", result["previous"]);
        if result.get("dof").is_some() {
            println!("dof      {}", result["dof"]);
        }
        if result.get("bounds").is_some() {
            bounds_summary(result);
        }
    })
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
                    ("length", slot(length)),
                    ("midplane", flag(midplane)),
                    ("reversed", flag(reversed)),
                    ("name", text(name)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pad    {}", field(result, "pad"));
                println!("length {}", slot_text(&result["length"]));
                println!("solid  {}", result["solid"]);
                bounds_summary(result);
            })
        }
        PadCommand::Length {
            length,
            document,
            pad,
            unbind,
            format,
        } => set_slot(
            "pad",
            "length".to_string(),
            length,
            document,
            pad,
            unbind,
            format,
        ),
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
                    ("length", slot(length)),
                    ("through_all", flag(through_all)),
                    ("midplane", flag(midplane)),
                    ("reversed", flag(reversed)),
                    ("name", text(name)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("pocket {}", field(result, "pocket"));
                println!("length {}", slot_text(&result["length"]));
                println!("solid  {}", result["solid"]);
                bounds_summary(result);
            })
        }
        PocketCommand::Length {
            length,
            document,
            pocket,
            unbind,
            format,
        } => set_slot(
            "pocket",
            "length".to_string(),
            length,
            document,
            pocket,
            unbind,
            format,
        ),
    }
}

fn param(command: ParamCommand) -> Result<i32> {
    match command {
        ParamCommand::New {
            name,
            value,
            document,
            format,
        } => declare("param.new", name, value, document, format),
        ParamCommand::Set {
            name,
            value,
            document,
            format,
        } => declare("param.set", name, value, document, format),
        ParamCommand::List { document, format } => {
            let result = call("param.list", params(vec![("document", text(document))]))?;

            emit(&result, format, param_list_summary)
        }
        ParamCommand::Remove {
            name,
            document,
            force,
            format,
        } => {
            let result = call(
                "param.remove",
                params(vec![
                    ("document", text(document)),
                    ("name", Some(Value::from(name))),
                    ("force", flag(force)),
                ]),
            )?;

            emit(&result, format, |result| {
                println!("removed {}", field(result, "name"));

                let froze = result["froze"].as_array().cloned().unwrap_or_default();
                if froze.is_empty() {
                    return;
                }

                let rows: Vec<Vec<String>> = froze
                    .iter()
                    .map(|entry| {
                        vec![
                            field(entry, "object").to_string(),
                            field(entry, "slot").to_string(),
                        ]
                    })
                    .collect();

                println!();
                println!("{} slot(s) now hold a literal", rows.len());
                cmd::print_table(&["object", "slot"], &rows);
            })
        }
    }
}

/// `param new` and `param set` differ only in whether the name may already
/// exist, and the server decides that: neither one can quietly do the other's
/// job, which is the whole reason they are two verbs.
fn declare(
    method: &str,
    name: String,
    value: ParamValue,
    document: Option<String>,
    format: Format,
) -> Result<i32> {
    let mut request = vec![
        ("document", text(document)),
        ("name", Some(Value::from(name))),
    ];
    request.extend(param_value(value));

    let result = call(method, params(request))?;

    emit(&result, format, |result| {
        println!("name     {}", field(result, "name"));
        println!("value    {}", result["value"]);
        if let Some(expression) = result.get("expression").and_then(Value::as_str) {
            println!("computed ={expression}");
        }
        if let Some(previous) = result.get("previous").filter(|value| !value.is_null()) {
            println!("previous {previous}");
        }

        let drives = result["drives"].as_array().cloned().unwrap_or_default();
        println!("drives   {}", drives.len());
        for entry in &drives {
            println!(
                "         {} {}",
                field(entry, "object"),
                field(entry, "slot")
            );
        }

        // A fresh parameter drives nothing by definition, but driving one that
        // reaches no slot moved no geometry, and the command still succeeded.
        // Nothing else in the output distinguishes that from a change that
        // worked, and the caller cannot see the model to notice.
        if drives.is_empty() && method == "param.set" {
            println!("         nothing follows it, so this changed no geometry");
        }
    })
}

fn param_list_summary(result: &Value) {
    let parameters = result["parameters"].as_array().cloned().unwrap_or_default();

    if parameters.is_empty() {
        println!("no parameters: `param new <name> <value>` declares one");
    } else {
        let rows: Vec<Vec<String>> = parameters
            .iter()
            .map(|parameter| {
                let drives = parameter["drives"].as_array().cloned().unwrap_or_default();

                vec![
                    field(parameter, "name").to_string(),
                    parameter["value"].to_string(),
                    parameter
                        .get("expression")
                        .and_then(Value::as_str)
                        .map(|expression| format!("={expression}"))
                        .unwrap_or_default(),
                    drives
                        .iter()
                        .map(|entry| format!("{}.{}", field(entry, "object"), field(entry, "slot")))
                        .collect::<Vec<_>>()
                        .join(" "),
                ]
            })
            .collect();

        cmd::print_table(&["name", "value", "computed", "drives"], &rows);
    }

    // Named dimensions nothing drives. A document written before this registry
    // existed is all orphans, and the way in is two commands rather than a
    // migration verb.
    let orphans = result["orphans"].as_array().cloned().unwrap_or_default();
    if orphans.is_empty() {
        return;
    }

    // Spelled out per orphan rather than as the shape of the command. A slot
    // carries whatever name it was drawn with, which is rarely the one a reader
    // would guess, and an unnamed `--sketch` hits the newest sketch rather than
    // the one on the row. Both are known here, so the only part left blank is
    // the parameter's name, which is the one thing this cannot decide.
    let rows: Vec<Vec<String>> = orphans
        .iter()
        .map(|orphan| {
            let object = field(orphan, "object");
            let slot = field(orphan, "slot");

            vec![
                object.to_string(),
                slot.to_string(),
                field(orphan, "type").to_string(),
                orphan["value"].to_string(),
                format!("sketch set {slot} <name> --sketch {object}"),
            ]
        })
        .collect();

    println!();
    println!(
        "{} dimension(s) no parameter drives; `param new <name> <value>` declares one, \
         then the line beside a row binds it",
        rows.len()
    );
    cmd::print_table(&["object", "slot", "type", "value", "adopt"], &rows);
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
