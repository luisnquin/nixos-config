use serde::Serialize;
use serde_json::Value;

/// The shape `phone` on the controlling host already parses out of a
/// `uiautomator` dump. Emitting it here rather than an iOS-flavoured tree means
/// the matching, ambiguity and `@index` handling stay in one implementation and
/// a simulator answers `phone tap "Log in"` the same way a handset does.
#[derive(Debug, Serialize)]
pub struct Node {
    pub index: usize,
    pub text: String,
    pub desc: String,
    pub res_id: String,
    pub class: String,
    pub clickable: bool,
    pub bounds: Bounds,
    /// Every frame this element sits inside, nearest first. The controlling
    /// host crops to these when asked for a control rather than for the words
    /// on it, and a run of wrappers drawn on one frame counts as one box.
    pub ancestors: Vec<Bounds>,
}

/// Points, not pixels. Android reports the panel in pixels and iOS reports it in
/// points; each side taps in whatever it reported, so nothing has to convert.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// Roles that answer to a touch. `enabled` is not the test: a springboard icon
/// reports `enabled: false` and opens anyway, so the role is what decides.
const ACTIONABLE: &[&str] = &[
    "button",
    "textfield",
    "searchfield",
    "secure",
    "switch",
    "link",
    "cell",
    "tab",
    "slider",
    "stepper",
    "menuitem",
    "checkbox",
    "radiobutton",
];

pub fn flatten(tree: &Value) -> Vec<Node> {
    let mut nodes = Vec::new();

    if let Some(roots) = tree.get("roots").and_then(Value::as_array) {
        for root in roots {
            walk(root, &[], &mut nodes);
        }
    }

    nodes
}

fn walk(node: &Value, enclosing: &[Bounds], out: &mut Vec<Node>) {
    if let Some(mapped) = map(node, out.len(), enclosing) {
        out.push(mapped);
    }

    let nested;
    let enclosing = match bounds(node) {
        Some(frame) if enclosing.first() != Some(&frame) => {
            nested = [&[frame][..], enclosing].concat();
            &nested
        }
        _ => enclosing,
    };

    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            walk(child, enclosing, out);
        }
    }
}

fn map(node: &Value, index: usize, enclosing: &[Bounds]) -> Option<Node> {
    if node.get("hidden").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }

    let bounds = bounds(node)?;

    if bounds.x2 <= bounds.x1 || bounds.y2 <= bounds.y1 {
        return None;
    }

    let class = field(node, "role").unwrap_or_default();
    let clickable = is_actionable(&class, &field(node, "type").unwrap_or_default());

    // AXLabel names the control and AXValue holds what it currently carries,
    // which is the same split Android draws between content-desc and text.
    let desc = field(node, "AXLabel").unwrap_or_default();
    let text = field(node, "AXValue")
        .or_else(|| field(node, "title"))
        .unwrap_or_default();

    if text.trim().is_empty() && desc.trim().is_empty() && !clickable {
        return None;
    }

    Some(Node {
        index,
        text,
        desc,
        res_id: field(node, "AXUniqueId").unwrap_or_default(),
        class,
        clickable,
        bounds,
        ancestors: enclosing.to_vec(),
    })
}

fn is_actionable(role: &str, kind: &str) -> bool {
    let haystack = format!("{role} {kind}").to_lowercase();

    ACTIONABLE.iter().any(|needle| haystack.contains(needle))
}

fn field(node: &Value, key: &str) -> Option<String> {
    let value = node.get(key)?.as_str()?.to_string();

    (!value.is_empty()).then_some(value)
}

fn bounds(node: &Value) -> Option<Bounds> {
    let frame = node.get("frame")?;
    let read = |key| frame.get(key)?.as_f64();

    let x = read("x")?;
    let y = read("y")?;
    let width = read("width")?;
    let height = read("height")?;

    Some(Bounds {
        x1: x.round() as i32,
        y1: y.round() as i32,
        x2: (x + width).round() as i32,
        y2: (y + height).round() as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> Value {
        json!({
            "roots": [{
                "AXLabel": " ",
                "role": "AXApplication",
                "type": "Application",
                "frame": {"x": 0, "y": 0, "width": 440, "height": 956},
                "children": [
                    {
                        "AXLabel": "Fitness",
                        "AXUniqueId": "Fitness",
                        "AXValue": "",
                        "role": "AXButton",
                        "type": "Button",
                        "enabled": false,
                        "frame": {"x": 33, "y": 92, "width": 72, "height": 94.66666666666666}
                    },
                    {
                        "AXLabel": "Search",
                        "AXValue": "Hola",
                        "role": "AXTextField",
                        "type": "TextField",
                        "frame": {"x": 20, "y": 700, "width": 400, "height": 40}
                    },
                    {
                        "AXLabel": "Collapsed",
                        "role": "AXButton",
                        "type": "Button",
                        "frame": {"x": 0, "y": 0, "width": 0, "height": 0}
                    },
                    {
                        "AXLabel": "Offscreen",
                        "role": "AXButton",
                        "type": "Button",
                        "hidden": true,
                        "frame": {"x": 20, "y": 800, "width": 100, "height": 40}
                    }
                ]
            }]
        })
    }

    /// The application root carries a whitespace label and takes no touch, so it
    /// is noise the controlling host could never match on.
    #[test]
    fn keeps_only_what_can_be_read_or_pressed() {
        let nodes = flatten(&tree());
        let labels: Vec<&str> = nodes.iter().map(|n| n.desc.as_str()).collect();

        assert_eq!(labels, ["Fitness", "Search"]);
    }

    /// A springboard icon reports `enabled: false` and opens anyway, so nothing
    /// may read that field to decide whether the element takes a touch.
    #[test]
    fn a_disabled_looking_icon_is_still_pressable() {
        let nodes = flatten(&tree());

        assert!(nodes[0].clickable, "an AXButton takes a touch");
    }

    #[test]
    fn drops_an_element_with_no_area() {
        let labels: Vec<String> = flatten(&tree()).into_iter().map(|n| n.desc).collect();

        assert!(!labels.contains(&"Collapsed".to_string()));
    }

    #[test]
    fn drops_an_element_the_device_is_not_showing() {
        let labels: Vec<String> = flatten(&tree()).into_iter().map(|n| n.desc).collect();

        assert!(!labels.contains(&"Offscreen".to_string()));
    }

    /// The controlling host taps the middle of these bounds, so the rounding has
    /// to survive a frame that is not a whole number of points.
    #[test]
    fn rounds_a_fractional_frame_to_whole_points() {
        let nodes = flatten(&tree());

        assert_eq!(nodes[0].bounds.y1, 92);
        assert_eq!(nodes[0].bounds.y2, 187);
    }

    #[test]
    fn splits_the_name_of_a_field_from_what_it_holds() {
        let nodes = flatten(&tree());

        assert_eq!(nodes[1].desc, "Search");
        assert_eq!(nodes[1].text, "Hola");
    }

    /// A crop of a label is a crop of the words on a control. The frames above
    /// it are what the controlling host widens to, and it counts them from the
    /// element outwards.
    #[test]
    fn an_element_carries_the_frames_it_sits_inside() {
        let nodes = flatten(&tree());

        assert_eq!(
            nodes[0].ancestors,
            [Bounds {
                x1: 0,
                y1: 0,
                x2: 440,
                y2: 956
            }]
        );
    }

    /// `index` addresses a node in the emitted list, so it has to count what
    /// survived rather than where the element sat in the tree.
    #[test]
    fn numbers_the_nodes_it_emits() {
        let indices: Vec<usize> = flatten(&tree()).into_iter().map(|n| n.index).collect();

        assert_eq!(indices, [0, 1]);
    }
}
