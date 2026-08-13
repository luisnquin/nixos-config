use std::time::Duration;

use anyhow::{bail, Result};

use crate::adb::{self, Server};

/// Where the dump lands on the device. uiautomator will not write to stdout on
/// every vendor build, so it goes to a file that is read and removed in the same
/// shell, which keeps the whole thing to one round trip.
const REMOTE: &str = "uiautomator dump /sdcard/.phone-a11y.xml >/dev/null 2>&1; \
     cat /sdcard/.phone-a11y.xml; rm -f /sdcard/.phone-a11y.xml";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Bounds {
    /// `[x1,y1][x2,y2]`, the only shape uiautomator emits.
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

#[derive(Clone, Debug)]
pub struct Node {
    /// Position in the emitted list. Stable only within one dump, which is why
    /// it is handed out alongside the text rather than instead of it.
    pub index: usize,
    pub text: String,
    pub desc: String,
    pub res_id: String,
    pub class: String,
    pub clickable: bool,
    pub bounds: Bounds,
}

impl Node {
    /// What to call this element in output a human or an agent reads. Falls
    /// through to the class name so a nameless button is still addressable.
    pub fn label(&self) -> &str {
        for candidate in [&self.text, &self.desc, &self.res_id] {
            if !candidate.is_empty() {
                return candidate;
            }
        }

        short_class(&self.class)
    }

    /// Whether `needle` names this element. Matched case-insensitively across
    /// every field a caller might have read it from, because the label shown is
    /// whichever field happened to be populated.
    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.to_lowercase();

        [&self.text, &self.desc, &self.res_id]
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
    }
}

fn short_class(class: &str) -> &str {
    class.rsplit('.').next().unwrap_or(class)
}

/// The interactive and text-bearing elements of the current screen.
///
/// The raw hierarchy is mostly layout containers — a screen with ten controls
/// dumps a few hundred nodes. Only what can be read or pressed is kept, which
/// is what makes the result small enough to act on.
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

        // a zero-area node cannot be pressed and cannot be read
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

pub async fn dump(server: &Server, serial: &str) -> Result<Vec<Node>> {
    let (ok, bytes) = adb::run_bytes(server, &["-s", serial, "exec-out", REMOTE]).await?;
    let xml = String::from_utf8_lossy(&bytes);

    if !ok || !xml.contains("<hierarchy") {
        bail!("uiautomator returned no hierarchy (is the screen on and unlocked?)");
    }

    parse(&xml)
}

/// The one element `needle` names, or an error that says why it could not be
/// narrowed to one. Ambiguity is reported rather than resolved by picking the
/// first, since acting on the wrong control is worse than not acting.
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
            // an exact label beats the substring hits it is buried in
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

pub async fn tap(server: &Server, serial: &str, x: i32, y: i32) -> Result<()> {
    let (x, y) = (x.to_string(), y.to_string());
    let out = adb::run_timeout(
        server,
        &["-s", serial, "shell", "input", "tap", &x, &y],
        Duration::from_secs(10),
    )
    .await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
    }
}

/// The characters `input text` cannot carry, sorted and deduplicated.
///
/// It renders each character through the device's KeyCharacterMap, which only
/// spells out printable ASCII. Everything else — an accent, a newline, an emoji
/// — is dropped on the way and the command still exits 0, so a half typed
/// string is indistinguishable from a whole one.
fn unsendable(text: &str) -> Vec<char> {
    let mut odd: Vec<char> = text
        .chars()
        .filter(|c| !c.is_ascii_graphic() && *c != ' ')
        .collect();

    odd.sort_unstable();
    odd.dedup();

    odd
}

pub async fn type_text(server: &Server, serial: &str, text: &str) -> Result<()> {
    let odd = unsendable(text);

    if !odd.is_empty() {
        bail!("cannot type {odd:?} — the device spells out ASCII only and drops the rest silently");
    }

    // `input text` splits on spaces and reads the rest as its own flags, so the
    // device-side shell has to receive it as one already-quoted word.
    let quoted = format!("input text {}", shell_quote(text));
    let out = adb::run_timeout(
        server,
        &["-s", serial, "shell", &quoted],
        Duration::from_secs(20),
    )
    .await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
    }
}

/// The keys worth naming when driving a phone.
///
/// `input keyevent` accepts many more, but it also accepts nonsense: an unknown
/// name exits 0 and prints nothing, so the device can never report a typo. A
/// name is checked here instead, and a bare number is left as the way to reach
/// the codes not listed.
const KEYS: &[&str] = &[
    "APP_SWITCH",
    "BACK",
    "CALL",
    "CAMERA",
    "DEL",
    "DPAD_CENTER",
    "DPAD_DOWN",
    "DPAD_LEFT",
    "DPAD_RIGHT",
    "DPAD_UP",
    "ENDCALL",
    "ENTER",
    "ESCAPE",
    "FORWARD_DEL",
    "HOME",
    "MEDIA_NEXT",
    "MEDIA_PLAY_PAUSE",
    "MEDIA_PREVIOUS",
    "MENU",
    "MOVE_END",
    "MOVE_HOME",
    "NOTIFICATION",
    "PAGE_DOWN",
    "PAGE_UP",
    "POWER",
    "SEARCH",
    "SETTINGS",
    "SLEEP",
    "TAB",
    "VOLUME_DOWN",
    "VOLUME_MUTE",
    "VOLUME_UP",
    "WAKEUP",
];

fn keycode(name: &str) -> Result<String> {
    let name = name.trim().to_uppercase();
    let name = name.strip_prefix("KEYCODE_").unwrap_or(&name);

    if name.parse::<u16>().is_ok() || KEYS.contains(&name) {
        return Ok(name.to_string());
    }

    let near: Vec<&str> = KEYS
        .iter()
        .copied()
        .filter(|key| key.contains(name) || name.contains(key))
        .collect();

    if near.is_empty() {
        bail!("unknown key '{name}' (try: back, home, enter, tab)")
    }

    bail!("unknown key '{name}' — did you mean {}?", near.join(", "))
}

pub async fn key(server: &Server, serial: &str, name: &str) -> Result<()> {
    let code = keycode(name)?;
    let out = adb::run_timeout(
        server,
        &["-s", serial, "shell", "input", "keyevent", &code],
        Duration::from_secs(10),
    )
    .await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
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

        // the root FrameLayout carries no text and no press, and the collapsed
        // button has no area to aim at
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

    /// An agent that taps the wrong control reports success, so a needle that
    /// names two things has to fail rather than choose.
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

    /// "Log in" is a substring of nothing else here, but a label that is exactly
    /// the needle must win over one that merely contains it.
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

    /// `input keyevent` exits 0 on a name it does not know and prints nothing,
    /// so a typo that is not caught here reads as a key that was sent.
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
