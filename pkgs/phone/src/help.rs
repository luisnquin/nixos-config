use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{CommandFactory, FromArgMatches};

use crate::cli::Cli;

const GROUPS: &[(&str, &[&str])] = &[
    ("Project", &["up", "down", "status"]),
    ("Read the screen", &["snapshot", "shot", "size"]),
    ("Act on it", &["tap", "press", "swipe", "type", "fill", "key"]),
    ("Wait and chain", &["wait", "do"]),
    ("Devices and apps", &["device", "app", "host"]),
    ("Watch", &["mirror", "stream", "record"]),
    ("Help", &["doctor", "hook", "help"]),
];

pub fn parse() -> Cli {
    let mut matches = command().try_get_matches().unwrap_or_else(|mut e| {
        if let Some(tip) = misplaced(&e) {
            e.remove(ContextKind::SuggestedArg);
            e.insert(
                ContextKind::Suggested,
                ContextValue::StyledStrs(vec![tip.into()]),
            );
        }

        e.exit()
    });

    Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|e| e.format(&mut command()).exit())
}

/// A flag this verb does not take, named with the verbs that do. It replaces
/// clap's tips: a fuzzy look-alike here, or passing it as the element's name.
pub fn misplaced(e: &clap::Error) -> Option<String> {
    if e.kind() != ErrorKind::UnknownArgument {
        return None;
    }

    let Some(ContextValue::String(arg)) = e.get(ContextKind::InvalidArg) else {
        return None;
    };
    let long = arg.strip_prefix("--")?.split('=').next()?;

    let mut verbs = Vec::new();
    owners(&Cli::command(), long, "", &mut verbs);

    (!verbs.is_empty()).then(|| format!("--{long} belongs to {}", verbs.join(", ")))
}

fn owners(cmd: &clap::Command, long: &str, path: &str, out: &mut Vec<String>) {
    for sub in cmd.get_subcommands().filter(|s| !s.is_hide_set()) {
        let name = format!("{path}{}", sub.get_name());

        if sub.get_arguments().any(|a| a.get_long() == Some(long)) {
            out.push(format!("`{name}`"));
        }

        owners(sub, long, &format!("{name} "), out);
    }
}

pub fn command() -> clap::Command {
    let mut cmd = Cli::command();
    cmd.build();

    let header = *cmd.get_styles().get_header();
    let mut groups = Vec::new();

    for (title, names) in GROUPS {
        let mut group = clap::Command::new(*title)
            .styles(cmd.get_styles().clone())
            .disable_help_subcommand(true)
            .help_template("{subcommands}");

        for (order, name) in names.iter().enumerate() {
            if let Some(sub) = cmd.find_subcommand(name) {
                group = group.subcommand(sub.clone().display_order(order));
            }
        }

        let list = group.render_help().ansi().to_string();
        groups.push(format!("{header}{title}:{header:#}\n{}", list.trim_end()));
    }

    cmd.help_template(format!(
        "{{before-help}}{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n{}\n\n{header}Options:{header:#}\n{{options}}{{after-help}}",
        groups.join("\n\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_visible_command_sits_in_exactly_one_group() {
        let mut cmd = Cli::command();
        cmd.build();

        let visible: Vec<&str> = cmd
            .get_subcommands()
            .filter(|c| !c.is_hide_set())
            .map(|c| c.get_name())
            .collect();
        let grouped: Vec<&str> = GROUPS
            .iter()
            .flat_map(|(_, names)| names.iter().copied())
            .collect();

        for name in &visible {
            let n = grouped.iter().filter(|g| *g == name).count();
            assert_eq!(n, 1, "`{name}` appears in {n} help groups");
        }
        for name in &grouped {
            assert!(
                visible.contains(name),
                "help group names `{name}`, which is no visible command"
            );
        }
    }

    #[test]
    fn help_lists_every_group() {
        let help = command().render_help().to_string();

        for (title, names) in GROUPS {
            assert!(help.contains(&format!("{title}:")), "{title} missing");
            for name in *names {
                assert!(help.contains(&format!("\n  {name} ")), "{name} missing");
            }
        }
        assert!(help.contains("How this is meant to be used"));
        assert!(!help.contains("Commands:"));
    }
}
