//! Minimal argument parsing. cliplenz is a dmenu: stdin -> pick -> stdout.
//! Flags are kept close to fuzzel's so it drops into the same wrappers.

#[derive(Clone)]
pub struct Args {
    pub prompt: String,
    pub placeholder: String,
    /// Command run (via `sh -c`) to render the focused entry's full content.
    /// The entry's raw line is fed on stdin; stdout is shown (image or text).
    pub preview: Option<String>,
    /// Interface font family. An unknown name falls back silently to whatever
    /// font is loaded; `None` leaves iced's default.
    pub font: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            prompt: "❯".to_string(),
            placeholder: "Search…".to_string(),
            preview: None,
            font: None,
        }
    }
}

impl Args {
    pub fn parse() -> Args {
        let mut args = Args::default();
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            match flag.as_str() {
                "--prompt" | "-p" => {
                    if let Some(v) = it.next() {
                        args.prompt = v;
                    }
                }
                "--placeholder" => {
                    if let Some(v) = it.next() {
                        args.placeholder = v;
                    }
                }
                "--preview" => args.preview = it.next(),
                "--font" => args.font = it.next(),
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        args
    }
}

fn print_help() {
    println!(
        "cliplenz — fuzzy dmenu with image preview\n\n\
         usage: <producer> | cliplenz [--preview <cmd>] [--prompt <s>] [--placeholder <s>] [--font <family>]\n\n\
         reads newline-separated entries on stdin, prints the selected entry on stdout.\n\
         --preview <cmd>  run <cmd> (sh -c) with the focused entry on stdin; its stdout\n\
         \x20                is rendered as an image (if it sniffs as one) or as text.\n\
         --font <family>  interface font family; must be loadable by the binary.\n\n\
         example: cliphizt list | cliplenz --preview 'cliphizt decode' | wl-copy ... "
    );
}
