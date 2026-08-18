mod app;
mod cli;
mod external;
mod fuzzy;
mod model;
mod theme;

use std::io::Read;

use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};

use app::{namespace, State};
use cli::Args;
use model::Entry;

fn main() -> Result<(), iced_layershell::Error> {
    // Force the software renderer: no GPU/wgpu init -> fuzzel-like cold start.
    if std::env::var_os("ICED_BACKEND").is_none() {
        std::env::set_var("ICED_BACKEND", "tiny-skia");
    }

    let args = Args::parse();

    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let entries = Entry::from_stdin(&String::from_utf8_lossy(&input));

    // Wider surface when a preview pane is present; a plain list stays narrow.
    let size = if args.preview.is_some() {
        (860, 500)
    } else {
        (560, 480)
    };

    // iced needs a `&'static str`; leaking the chosen family is fine for a
    // one-shot process.
    let default_font = match args
        .font
        .clone()
        .or_else(|| std::env::var("CLIPLENZ_FONT").ok())
    {
        Some(name) if !name.is_empty() => iced::Font::with_name(Box::leak(name.into_boxed_str())),
        _ => iced::Font::DEFAULT,
    };

    let boot = move || State::boot(entries.clone(), args.clone());

    iced_layershell::application(boot, namespace, State::update, State::view)
        .subscription(State::subscription)
        .style(|_state, _theme| iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: theme::TEXT,
        })
        .settings(Settings {
            default_font,
            default_text_size: iced::Pixels(13.0),
            layer_settings: LayerShellSettings {
                // Empty anchor + fixed size = centered floating surface.
                anchor: Anchor::empty(),
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                size: Some(size),
                exclusive_zone: 0,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
