//! Machine-readable command catalog (CKSPEC-AGENT-006).
//!
//! These types ARE the catalog schema — emitting them as the `data` of an
//! OUT-002 success envelope is how a ckeletin CLI self-reports its command
//! surface as structured data, so an agent discovers capabilities without
//! parsing human `--help` text or trusting hand-written AGENTS.md prose.
//!
//! The schema is the cross-implementation contract agreed with ckeletin-go on
//! the spec issue: a **required core** every implementation derives losslessly
//! (so one parser works across both), plus **optional** fields each emits where
//! its CLI framework exposes them (omitted, never hand-filled).
//!
//! Required core:
//!   - command: `name`, `description`, `flags`, `commands` (recursive)
//!   - flag: `long`, `required`, `takes_value`
//!   - top level: `name`, `description`, `global_flags` (listed once), `commands`
//!
//! Optional (emit where the CLI framework derives them, omit otherwise):
//! flag `short`, `description`, `default`, `possible_values`, `aliases`,
//! `value_type` (the value name shown in help, such as `FILE`), `help_heading`;
//! command `aliases`, `positional_args`, `valid_args`, `external_subcommands`,
//! `help_heading`.
//!
//! Defining the types in the framework crate (not the cli crate) makes the
//! schema a single shared Rust type: a derived project literally cannot emit a
//! wrong-shaped catalog. The clap walk lives in `.ckeletin/catalog-clap`.
//! This crate does not depend on clap (CKSPEC-ARCH-003). Clap 4.6 does not
//! expose a command deprecation string or the help heading of each subcommand,
//! so those are not fields: a value that cannot be derived is omitted, not
//! invented.

use serde::Serialize;
use std::fmt;

/// A single flag in the catalog.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogFlag {
    /// Long form, without `--` (required core).
    pub long: String,
    /// Whether the flag must be supplied (required core).
    pub required: bool,
    /// Whether the flag consumes a value (`--x value`) vs. a boolean switch
    /// (required core; the normalized intersection both clap and cobra derive).
    pub takes_value: bool,
    /// Short form, without `-` (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Help text (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Default value, if any (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Allowed values for enumerated flags, e.g. `--output text|json` (optional).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Other long names besides `long` (optional).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Value name shown in help, such as `FILE` (optional rich type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Help heading this flag is listed under (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_heading: Option<String>,
}

/// A positional argument.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogPositional {
    pub name: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}

/// A command and its subcommands.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogCommand {
    pub name: String,
    pub description: String,
    pub flags: Vec<CatalogFlag>,
    pub commands: Vec<CatalogCommand>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positional_args: Vec<CatalogPositional>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub valid_args: Vec<String>,
    /// True when the command accepts subcommands it does not enumerate.
    #[serde(skip_serializing_if = "is_false")]
    pub external_subcommands: bool,
    /// Heading this command is listed under in its parent's help (optional).
    /// Clap 4.6 stores one subcommand heading on the parent, not one per child.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_heading: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The whole command surface of a CLI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Catalog {
    pub name: String,
    pub description: String,
    /// Flags that apply to every command, listed once here (not duplicated into
    /// each command's `flags`).
    pub global_flags: Vec<CatalogFlag>,
    pub commands: Vec<CatalogCommand>,
    /// True when the root command accepts subcommands it does not enumerate.
    #[serde(skip_serializing_if = "is_false")]
    pub external_subcommands: bool,
}

fn fmt_flag(f: &mut fmt::Formatter<'_>, flag: &CatalogFlag) -> fmt::Result {
    let value = if flag.takes_value { " <value>" } else { "" };
    let desc = flag
        .description
        .as_deref()
        .map(|d| format!("  {d}"))
        .unwrap_or_default();
    writeln!(f, "  --{}{}{}", flag.long, value, desc)
}

impl fmt::Display for Catalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} — {}", self.name, self.description)?;
        if !self.global_flags.is_empty() {
            writeln!(f, "\nGlobal flags:")?;
            for flag in &self.global_flags {
                fmt_flag(f, flag)?;
            }
        }
        writeln!(f, "\nCommands:")?;
        for cmd in &self.commands {
            writeln!(f, "  {:<12} {}", cmd.name, cmd.description)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Catalog {
        Catalog {
            name: "demo".into(),
            description: "A demo CLI".into(),
            external_subcommands: false,
            global_flags: vec![CatalogFlag {
                long: "output".into(),
                required: false,
                takes_value: true,
                short: None,
                description: Some("Output format".into()),
                default: Some("text".into()),
                possible_values: vec!["text".into(), "json".into()],
                aliases: vec![],
                value_type: Some("FORMAT".into()),
                help_heading: None,
            }],
            commands: vec![CatalogCommand {
                name: "ping".into(),
                description: "Check connectivity".into(),
                flags: vec![],
                commands: vec![],
                aliases: vec![],
                positional_args: vec![],
                valid_args: vec![],
                external_subcommands: false,
                help_heading: None,
            }],
        }
    }

    #[test]
    fn serializes_required_core_and_skips_empty_optionals() {
        let json = serde_json::to_value(sample()).unwrap();
        assert_eq!(json["name"], "demo");
        assert_eq!(json["global_flags"][0]["long"], "output");
        assert_eq!(json["global_flags"][0]["takes_value"], true);
        assert_eq!(json["global_flags"][0]["possible_values"][1], "json");
        assert_eq!(json["commands"][0]["name"], "ping");
        // A flag with no short/possible_values omits them rather than emitting null/[].
        let bare = serde_json::to_value(CatalogFlag {
            long: "verbose".into(),
            required: false,
            takes_value: false,
            short: None,
            description: None,
            default: None,
            possible_values: vec![],
            aliases: vec![],
            value_type: None,
            help_heading: None,
        })
        .unwrap();
        assert!(bare.get("short").is_none());
        assert!(bare.get("possible_values").is_none());
        assert_eq!(bare["takes_value"], false);
    }

    #[test]
    fn display_lists_commands_and_global_flags() {
        let out = format!("{}", sample());
        assert!(out.contains("Commands:"));
        assert!(out.contains("ping"));
        assert!(out.contains("--output <value>"));
    }
}
