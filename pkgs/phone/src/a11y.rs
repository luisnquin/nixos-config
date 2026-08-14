use std::time::Duration;

use anyhow::{bail, Result};
use serde::Deserialize;

use crate::adb::{self, Server};
use crate::simctl;

/// uiautomator will not write to stdout on every vendor build, so the dump goes
/// to a file that is read and removed in the same shell.
const REMOTE: &str = "uiautomator dump /sdcard/.phone-a11y.xml >/dev/null 2>&1; \
     cat /sdcard/.phone-a11y.xml; rm -f /sdcard/.phone-a11y.xml";

/// A device to read and press, resolved once per invocation. The two arms differ
/// only in how a verb reaches the device: the elements they report and the
/// coordinates they take are the same shape either way.
pub enum Target {
    Adb(Adb),
    Simulator(Simulator),
}

pub struct Adb {
    pub server: Server,
    pub serial: String,
    pub display: Option<adb::Display>,
    pub focus: Option<(i32, i32)>,
}

/// CoreSimulator is macOS-local, so the verbs run on the host that owns the
/// simulator rather than over a transport pointed at it.
pub struct Simulator {
    pub host: String,
    pub udid: String,
}

impl Adb {
    /// The dump and every key go to whichever window holds focus, so a press
    /// that pulls it has to run in the same shell, where nothing can interleave.
    fn prefix(&self) -> String {
        self.focus
            .map(|(x, y)| format!("input tap {x} {y}; sleep 0.6; "))
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Bounds {
    fn parse(raw: &str) -> Option<Self> {
        let (a, b) = raw.trim_start_matches('[').split_once("][")?;
        let (x1, y1) = a.split_once(',')?;
        let (x2, y2) = b.trim_end_matches(']').split_once(',')?;

        Some(Bounds {
            x1: x1.parse().ok()?,
            y1: y1.parse().ok()?,
            x2: x2.parse().ok()?,
            y2: y2.parse().ok()?,
        })
    }

    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }

    fn area(&self) -> i64 {
        ((self.x2 - self.x1) as i64).max(0) * ((self.y2 - self.y1) as i64).max(0)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    /// Stable only within one dump.
    pub index: usize,
    pub text: String,
    pub desc: String,
    pub res_id: String,
    pub class: String,
    pub clickable: bool,
    pub bounds: Bounds,
}

impl Node {
    pub fn label(&self) -> &str {
        for candidate in [&self.text, &self.desc, &self.res_id] {
            if !candidate.is_empty() {
                return candidate;
            }
        }

        self.class.rsplit('.').next().unwrap_or(&self.class)
    }

    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.to_lowercase();

        [&self.text, &self.desc, &self.res_id]
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
    }
}

/// The elements that can be read or pressed. The rest of the hierarchy is
/// layout containers, which are most of the several hundred nodes a dump holds.
pub fn parse(xml: &str) -> Result<Vec<Node>> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut nodes = Vec::new();

    for element in doc.descendants().filter(|n| n.has_tag_name("node")) {
        let attr = |name| element.attribute(name).unwrap_or_default().to_string();

        let text = attr("text");
        let desc = attr("content-desc");
        let res_id = attr("resource-id");
        let clickable = element.attribute("clickable") == Some("true");

        if text.is_empty() && desc.is_empty() && !clickable {
            continue;
        }

        let Some(bounds) = element.attribute("bounds").and_then(Bounds::parse) else {
            continue;
        };

        if bounds.area() == 0 {
            continue;
        }

        nodes.push(Node {
            index: nodes.len(),
            text,
            desc,
            // the package prefix is identical for every node of one app
            res_id: res_id.rsplit('/').next().unwrap_or(&res_id).to_string(),
            class: attr("class"),
            clickable,
            bounds,
        });
    }

    Ok(nodes)
}

pub async fn dump(t: &Target) -> Result<Vec<Node>> {
    let a = match t {
        Target::Adb(a) => a,
        Target::Simulator(s) => return simctl::snapshot(&s.host, &s.udid).await,
    };

    let remote = format!("{}{REMOTE}", a.prefix());
    let (ok, bytes) = adb::run_bytes(&a.server, &["-s", &a.serial, "exec-out", &remote]).await?;
    let xml = String::from_utf8_lossy(&bytes);

    if !ok || !xml.contains("<hierarchy") {
        bail!("uiautomator returned no hierarchy (is the screen on and unlocked?)");
    }

    parse(&xml)
}

/// The one element `needle` names. Ambiguity is reported rather than resolved by
/// picking the first, since acting on the wrong control is worse than not acting.
pub fn pick<'a>(nodes: &'a [Node], needle: &str) -> Result<&'a Node> {
    if let Some(index) = needle
        .strip_prefix('@')
        .and_then(|n| n.parse::<usize>().ok())
    {
        return nodes
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("no element @{index} in this snapshot"));
    }

    let hits: Vec<&Node> = nodes.iter().filter(|n| n.matches(needle)).collect();

    match hits.as_slice() {
        [] => bail!("nothing on screen matches '{needle}'"),
        [one] => Ok(one),
        many => {
            let exact: Vec<&&Node> = many
                .iter()
                .filter(|n| n.label().eq_ignore_ascii_case(needle))
                .collect();

            if let [one] = exact.as_slice() {
                return Ok(one);
            }

            let listed = many
                .iter()
                .map(|n| format!("@{} {}", n.index, n.label()))
                .collect::<Vec<_>>()
                .join(", ");

            bail!("'{needle}' matches {} elements: {listed}", many.len())
        }
    }
}

/// Sent as one device-side command, because `input text` reads the rest of the
/// line as its own words and flags. Without `-d` it aims at logical display 0,
/// which is the live panel on everything that has one.
async fn input(a: &Adb, args: &str) -> Result<()> {
    let aim = a
        .display
        .map(|d| format!(" -d {}", d.logical))
        .unwrap_or_default();

    let remote = format!("{}input{aim} {args}", a.prefix());
    let out = adb::run_timeout(
        &a.server,
        &["-s", &a.serial, "shell", &remote],
        Duration::from_secs(20),
    )
    .await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
    }
}

pub async fn tap(t: &Target, x: i32, y: i32) -> Result<()> {
    match t {
        Target::Adb(a) => input(a, &format!("tap {x} {y}")).await,
        Target::Simulator(s) => simctl::tap(&s.host, &s.udid, x, y).await,
    }
}

/// What `input text` cannot carry. It spells characters through the device
/// KeyCharacterMap, which covers printable ASCII: the rest is dropped on the way
/// and the command still exits 0.
fn unsendable(text: &str) -> Vec<char> {
    let mut odd: Vec<char> = text
        .chars()
        .filter(|c| !c.is_ascii_graphic() && *c != ' ')
        .collect();

    odd.sort_unstable();
    odd.dedup();

    odd
}

pub async fn type_text(t: &Target, text: &str) -> Result<()> {
    let odd = unsendable(text);

    if !odd.is_empty() {
        bail!("cannot type {odd:?} — the device spells out ASCII only and drops the rest silently");
    }

    match t {
        Target::Adb(a) => input(a, &format!("text {}", shell_quote(text))).await,
        Target::Simulator(s) => simctl::type_text(&s.host, &s.udid, text).await,
    }
}

/// `input keyevent` exits 0 on a name it does not know and prints nothing, so
/// names are checked here. A bare number reaches the codes not listed.
const KEYS: &str = "APP_SWITCH BACK CALL CAMERA DEL DPAD_CENTER DPAD_DOWN DPAD_LEFT DPAD_RIGHT \
     DPAD_UP ENDCALL ENTER ESCAPE FORWARD_DEL HOME MEDIA_NEXT MEDIA_PLAY_PAUSE MEDIA_PREVIOUS \
     MENU MOVE_END MOVE_HOME NOTIFICATION PAGE_DOWN PAGE_UP POWER SEARCH SETTINGS SLEEP TAB \
     VOLUME_DOWN VOLUME_MUTE VOLUME_UP WAKEUP";

fn keycode(name: &str) -> Result<String> {
    let name = name.trim().to_uppercase();
    let name = name.strip_prefix("KEYCODE_").unwrap_or(&name);

    if name.parse::<u16>().is_ok() || KEYS.split_whitespace().any(|key| key == name) {
        return Ok(name.to_string());
    }

    let near: Vec<&str> = KEYS
        .split_whitespace()
        .filter(|key| key.contains(name) || name.contains(key))
        .collect();

    if near.is_empty() {
        bail!("unknown key '{name}' (try: back, home, enter, tab)")
    }

    bail!("unknown key '{name}' — did you mean {}?", near.join(", "))
}

pub async fn key(t: &Target, name: &str) -> Result<()> {
    // Both sides are addressed by the Android key name, so `phone key home` is
    // one command whatever answers it. Validated against the list each backend
    // actually has rather than against a union of both.
    match t {
        Target::Adb(a) => input(a, &format!("keyevent {}", keycode(name)?)).await,
        Target::Simulator(s) => simctl::key(&s.host, &s.udid, name).await,
    }
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
 <node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">
  <node class="android.widget.TextView" bounds="[40,100][600,180]" clickable="false" text="Sign in" content-desc="" resource-id="com.app:id/title"/>
  <node class="android.widget.EditText" bounds="[40,300][1040,400]" clickable="true" text="" content-desc="Email" resource-id="com.app:id/email"/>
  <node class="android.widget.Button" bounds="[40,500][1040,600]" clickable="true" text="Log in" content-desc="" resource-id="com.app:id/submit"/>
  <node class="android.widget.Button" bounds="[0,0][0,0]" clickable="true" text="Collapsed" content-desc="" resource-id=""/>
 </node>
</hierarchy>"#;

    #[test]
    fn keeps_only_what_can_be_read_or_pressed() {
        let nodes = parse(SAMPLE).expect("sample must parse");

        let labels: Vec<&str> = nodes.iter().map(|n| n.label()).collect();
        assert_eq!(labels, ["Sign in", "Email", "Log in"]);
    }

    #[test]
    fn strips_the_package_prefix_from_a_resource_id() {
        let nodes = parse(SAMPLE).unwrap();

        assert_eq!(nodes[0].res_id, "title");
    }

    #[test]
    fn aims_at_the_middle_of_an_element() {
        let nodes = parse(SAMPLE).unwrap();
        let button = pick(&nodes, "Log in").unwrap();

        assert_eq!(button.bounds.center(), (540, 550));
    }

    #[test]
    fn refuses_an_ambiguous_needle() {
        let nodes = parse(SAMPLE).unwrap();
        let err = pick(&nodes, "i").unwrap_err().to_string();

        assert!(err.contains("matches 3 elements"), "{err}");
        assert!(
            err.contains("@0"),
            "the alternatives must be addressable: {err}"
        );
    }

    #[test]
    fn an_exact_label_beats_the_substring_matches_around_it() {
        let xml = SAMPLE.replace(r#"text="Sign in""#, r#"text="Log in now""#);
        let nodes = parse(&xml).unwrap();

        assert_eq!(pick(&nodes, "Log in").unwrap().res_id, "submit");
    }

    #[test]
    fn addresses_an_element_by_its_index() {
        let nodes = parse(SAMPLE).unwrap();

        assert_eq!(pick(&nodes, "@1").unwrap().desc, "Email");
        assert!(pick(&nodes, "@9").is_err());
    }

    #[test]
    fn refuses_a_key_the_device_would_silently_drop() {
        assert_eq!(keycode("back").unwrap(), "BACK");
        assert_eq!(keycode("KEYCODE_HOME").unwrap(), "HOME");
        assert_eq!(keycode("66").unwrap(), "66", "a raw code reaches the rest");

        let err = keycode("voluem_up").unwrap_err().to_string();
        assert!(err.contains("unknown key"), "{err}");
    }

    #[test]
    fn points_at_the_key_that_was_probably_meant() {
        let err = keycode("volume").unwrap_err().to_string();

        assert!(
            err.contains("VOLUME_UP") && err.contains("VOLUME_DOWN"),
            "{err}"
        );
    }

    #[test]
    fn quotes_text_so_the_device_shell_sees_one_word() {
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn spanish_text_is_refused_rather_than_half_typed() {
        assert!(unsendable("hola que tal").is_empty());
        assert_eq!(unsendable("Menú de opciónes"), vec!['ó', 'ú']);
        assert_eq!(unsendable("line\nbreak"), vec!['\n']);
    }
}
