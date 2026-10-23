//! Clap → [`ckeletin::catalog::Catalog`] walk.
//!
//! The framework crate does not depend on clap. This crate does, and it ships
//! inside `.ckeletin/` so `just ckeletin-update` replaces the walk for every
//! consumer. Call [`build`] on the same `Command` the parser uses.

use ckeletin::catalog::{Catalog, CatalogCommand, CatalogFlag, CatalogPositional};
use clap::{Arg, ArgAction, Command as ClapCommand};

/// Derive the catalog from a clap command tree. Global flags are collected once
/// at the top level and are not repeated on each command.
pub fn build(root: &ClapCommand) -> Catalog {
    Catalog {
        name: root.get_name().to_string(),
        description: about_of(root),
        global_flags: root
            .get_arguments()
            .filter(|arg| arg.is_global_set() && !is_help_or_version(arg))
            .map(to_flag)
            .collect(),
        commands: child_commands(root),
        external_subcommands: root.is_allow_external_subcommands_set(),
    }
}

fn about_of(cmd: &ClapCommand) -> String {
    cmd.get_about().map(|s| s.to_string()).unwrap_or_default()
}

fn is_help_or_version(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

fn value_type_of(arg: &Arg) -> Option<String> {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map(|name| name.as_str().to_string())
        .filter(|name| !name.is_empty())
}

fn possible_values_of(arg: &Arg, takes_value: bool) -> Vec<String> {
    if !takes_value {
        return Vec::new();
    }
    arg.get_possible_values()
        .iter()
        .map(|value| value.get_name().to_string())
        .collect()
}

fn to_flag(arg: &Arg) -> CatalogFlag {
    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    let takes_value = matches!(arg.get_action(), ArgAction::Set | ArgAction::Append);
    let long = arg
        .get_long()
        .map(str::to_string)
        .unwrap_or_else(|| arg.get_id().as_str().to_string());
    // `Arg::get_aliases` is the hidden set. The catalog publishes the names help shows.
    let aliases = arg
        .get_visible_aliases()
        .into_iter()
        .flatten()
        .map(str::to_string)
        .filter(|alias| alias != &long)
        .collect();
    CatalogFlag {
        long,
        required: arg.is_required_set(),
        takes_value,
        short: arg.get_short().map(|c| c.to_string()),
        description: arg.get_help().map(|text| text.to_string()),
        default: (!defaults.is_empty()).then(|| defaults.join(",")),
        possible_values: possible_values_of(arg, takes_value),
        aliases,
        value_type: takes_value.then(|| value_type_of(arg)).flatten(),
        help_heading: arg.get_help_heading().map(str::to_string),
    }
}

fn to_positional(arg: &Arg) -> CatalogPositional {
    let takes_value = true;
    CatalogPositional {
        name: arg.get_id().as_str().to_string(),
        required: arg.is_required_set(),
        value_type: value_type_of(arg),
        description: arg.get_help().map(|text| text.to_string()),
        possible_values: possible_values_of(arg, takes_value),
    }
}

fn child_commands(cmd: &ClapCommand) -> Vec<CatalogCommand> {
    let heading = cmd.get_subcommand_help_heading().map(str::to_string);
    cmd.get_subcommands()
        .map(|child| {
            let mut walked = walk(child);
            if walked.help_heading.is_none() {
                walked.help_heading = heading.clone();
            }
            walked
        })
        .collect()
}

fn walk(cmd: &ClapCommand) -> CatalogCommand {
    let positionals: Vec<CatalogPositional> = cmd
        .get_arguments()
        .filter(|arg| arg.is_positional() && !is_help_or_version(arg))
        .map(to_positional)
        .collect();
    let valid_args = positionals
        .iter()
        .flat_map(|positional| positional.possible_values.clone())
        .collect();
    CatalogCommand {
        name: cmd.get_name().to_string(),
        description: about_of(cmd),
        flags: cmd
            .get_arguments()
            .filter(|arg| !arg.is_global_set() && !arg.is_positional() && !is_help_or_version(arg))
            .map(to_flag)
            .collect(),
        commands: child_commands(cmd),
        aliases: cmd.get_visible_aliases().map(str::to_string).collect(),
        positional_args: positionals,
        valid_args,
        external_subcommands: cmd.is_allow_external_subcommands_set(),
        help_heading: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};

    fn sample() -> Command {
        Command::new("demo")
            .about("A demo")
            .allow_external_subcommands(true)
            .subcommand_help_heading("Work")
            .arg(
                Arg::new("output")
                    .long("output")
                    .visible_alias("out")
                    .short('o')
                    .value_name("FORMAT")
                    .value_parser(["text", "json"])
                    .default_value("text")
                    .global(true)
                    .help("Output format")
                    .help_heading("Global"),
            )
            .subcommand(
                Command::new("run").about("Run it").visible_alias("go").arg(
                    Arg::new("file")
                        .value_name("FILE")
                        .required(true)
                        .help("Input file")
                        .value_parser(["a", "b"]),
                ),
            )
    }

    #[test]
    fn walks_positionals_aliases_headings_and_external_subcommands() {
        let catalog = build(&sample());
        assert_eq!(catalog.name, "demo");
        assert!(catalog.commands.iter().any(|cmd| cmd.name == "run"));
        let output = catalog
            .global_flags
            .iter()
            .find(|flag| flag.long == "output")
            .unwrap();
        assert_eq!(output.value_type.as_deref(), Some("FORMAT"));
        assert_eq!(output.aliases, vec!["out".to_string()]);
        assert_eq!(output.help_heading.as_deref(), Some("Global"));
        assert!(output.possible_values.contains(&"json".to_string()));

        let run = catalog
            .commands
            .iter()
            .find(|cmd| cmd.name == "run")
            .unwrap();
        assert_eq!(run.aliases, vec!["go".to_string()]);
        assert_eq!(run.help_heading.as_deref(), Some("Work"));
        assert_eq!(run.positional_args[0].name, "file");
        assert!(run.positional_args[0].required);
        assert_eq!(run.positional_args[0].value_type.as_deref(), Some("FILE"));
        assert!(run.valid_args.contains(&"a".to_string()));
        assert!(!run.external_subcommands);
        assert!(catalog.external_subcommands);
    }
}
