/// HID keyboard usages, the namespace the bridge passes to SimulatorKit.
const MODIFIER_SHIFT: u32 = 1 << 0;

/// A caller holding text and needing key events crosses here once instead of
/// growing a keymap of its own.
pub fn hid_for_char(character: char) -> Option<(u16, u32)> {
    if character.is_ascii_lowercase() {
        return Some((character as u16 - 'a' as u16 + 4, 0));
    }

    if character.is_ascii_uppercase() {
        return Some((character as u16 - 'A' as u16 + 4, MODIFIER_SHIFT));
    }

    let (hid, shifted) = match character {
        '1' => (30, false),
        '!' => (30, true),
        '2' => (31, false),
        '@' => (31, true),
        '3' => (32, false),
        '#' => (32, true),
        '4' => (33, false),
        '$' => (33, true),
        '5' => (34, false),
        '%' => (34, true),
        '6' => (35, false),
        '^' => (35, true),
        '7' => (36, false),
        '&' => (36, true),
        '8' => (37, false),
        '*' => (37, true),
        '9' => (38, false),
        '(' => (38, true),
        '0' => (39, false),
        ')' => (39, true),
        ' ' => (44, false),
        '-' => (45, false),
        '_' => (45, true),
        '=' => (46, false),
        '+' => (46, true),
        '[' => (47, false),
        '{' => (47, true),
        ']' => (48, false),
        '}' => (48, true),
        '\\' => (49, false),
        '|' => (49, true),
        ';' => (51, false),
        ':' => (51, true),
        '\'' => (52, false),
        '"' => (52, true),
        '`' => (53, false),
        '~' => (53, true),
        ',' => (54, false),
        '<' => (54, true),
        '.' => (55, false),
        '>' => (55, true),
        '/' => (56, false),
        '?' => (56, true),
        _ => return None,
    };

    Some((hid, if shifted { MODIFIER_SHIFT } else { 0 }))
}
