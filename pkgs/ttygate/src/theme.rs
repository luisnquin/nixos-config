use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Accent {
    #[default]
    Amber,
    Blue,
    Green,
    Mono,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
    pub on_accent: Color,
    pub fg: Color,
    pub dim: Color,
    pub error: Color,
    pub bg: Color,
    pub field_bg: Color,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Overrides {
    pub accent: Option<Color>,
    pub on_accent: Option<Color>,
    pub fg: Option<Color>,
    pub dim: Option<Color>,
    pub error: Option<Color>,
    pub bg: Option<Color>,
    pub field_bg: Option<Color>,
}

impl Theme {
    pub fn preset(accent: Accent) -> Self {
        let bg = Color::Rgb(10, 10, 12);
        match accent {
            Accent::Amber => Theme {
                accent: Color::Rgb(255, 176, 0),
                on_accent: Color::Rgb(10, 10, 10),
                fg: Color::Rgb(230, 224, 214),
                dim: Color::Rgb(120, 110, 92),
                error: Color::Rgb(255, 90, 60),
                bg,
                field_bg: brighten(bg, 14),
            },
            Accent::Blue => Theme {
                accent: Color::Rgb(30, 60, 255),
                on_accent: Color::Rgb(235, 238, 255),
                fg: Color::Rgb(225, 228, 240),
                dim: Color::Rgb(96, 102, 130),
                error: Color::Rgb(255, 80, 80),
                bg,
                field_bg: brighten(bg, 14),
            },
            Accent::Green => Theme {
                accent: Color::Rgb(70, 240, 120),
                on_accent: Color::Rgb(6, 16, 8),
                fg: Color::Rgb(200, 240, 205),
                dim: Color::Rgb(70, 120, 82),
                error: Color::Rgb(255, 90, 90),
                bg,
                field_bg: brighten(bg, 14),
            },
            Accent::Mono => Theme {
                accent: Color::Rgb(220, 220, 220),
                on_accent: Color::Rgb(12, 12, 12),
                fg: Color::Rgb(220, 220, 220),
                dim: Color::Rgb(110, 110, 110),
                error: Color::Rgb(230, 120, 120),
                bg,
                field_bg: brighten(bg, 14),
            },
        }
    }

    pub fn resolve(accent: Accent, o: &Overrides) -> Self {
        let mut t = Theme::preset(accent);
        if let Some(c) = o.accent {
            t.accent = c;
        }
        if let Some(c) = o.on_accent {
            t.on_accent = c;
        }
        if let Some(c) = o.fg {
            t.fg = c;
        }
        if let Some(c) = o.dim {
            t.dim = c;
        }
        if let Some(c) = o.error {
            t.error = c;
        }
        if let Some(c) = o.bg {
            t.bg = c;
            // Background set but field color not: keep the field a touch brighter.
            if o.field_bg.is_none() {
                t.field_bg = brighten(c, 14);
            }
        }
        if let Some(c) = o.field_bg {
            t.field_bg = c;
        }
        t
    }
}

pub fn parse_hex(s: &str) -> Result<Color, String> {
    let h = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("invalid hex color '{s}' (expected #rrggbb)"));
    }
    let n = u32::from_str_radix(h, 16).map_err(|e| e.to_string())?;
    Ok(Color::Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

fn brighten(c: Color, by: u8) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(by),
            g.saturating_add(by),
            b.saturating_add(by),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_with_and_without_hash() {
        assert_eq!(parse_hex("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex("00ff00").unwrap(), Color::Rgb(0, 255, 0));
    }

    #[test]
    fn hex_rejects_garbage() {
        assert!(parse_hex("nope").is_err());
        assert!(parse_hex("#12345").is_err());
        assert!(parse_hex("#zzzzzz").is_err());
    }

    #[test]
    fn override_replaces_only_specified_field() {
        let o = Overrides {
            accent: Some(Color::Rgb(1, 2, 3)),
            ..Default::default()
        };
        let t = Theme::resolve(Accent::Amber, &o);
        assert_eq!(t.accent, Color::Rgb(1, 2, 3));
        assert_eq!(t.error, Theme::preset(Accent::Amber).error);
    }
}
