//! Palette + widget styles, lifted from the user's fuzzel config so cliplenz
//! matches the rest of the desktop.

use iced::widget::{container, scrollable, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

pub const BACKGROUND: Color = rgb(0x16, 0x16, 0x16);
pub const TEXT: Color = rgb(0xff, 0xff, 0xff);
pub const MATCH: Color = rgb(0xee, 0x53, 0x96);
pub const SELECTION: Color = rgb(0x26, 0x26, 0x26);
pub const SELECTION_TEXT: Color = rgb(0x33, 0xb1, 0xff);
pub const BORDER: Color = rgb(0x52, 0x52, 0x52);
pub const DIM: Color = rgb(0x6f, 0x6f, 0x6f);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Rounded panel filling the (transparent) layer surface — fuzzel border=2 r=8.
pub fn root_panel(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(BACKGROUND)),
        border: Border {
            color: BORDER,
            width: 2.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

/// Background for the currently selected list row.
pub fn selected_row(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SELECTION)),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn transparent_row(_: &Theme) -> container::Style {
    container::Style::default()
}

pub fn search_input(_: &Theme, _: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: DIM,
        placeholder: DIM,
        value: TEXT,
        selection: SELECTION,
    }
}

pub fn scroll(_: &Theme, _: scrollable::Status) -> scrollable::Style {
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(BORDER),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: DIM,
        },
    }
}
