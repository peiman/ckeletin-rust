//! `catalog` command — emits the machine-readable command catalog
//! (CKSPEC-AGENT-006), derived from the SAME clap command tree the parser uses
//! (`Cli::command()`). Because the catalog and the parser read one tree, the
//! catalog cannot drift from the actual command set — anti-drift is structural,
//! not tested-in.
//!
//! The `Catalog` types are the framework schema. The clap walk lives in
//! `.ckeletin/catalog-clap`, so a framework update replaces it. This file
//! calls that walk.

use clap::{Command as ClapCommand, CommandFactory};
use infrastructure::catalog::Catalog;
use infrastructure::output::Output;
use std::io;

use crate::root::Cli;

/// Derive the catalog from a clap command tree.
pub fn build(root: &ClapCommand) -> Catalog {
    ckeletin_catalog_clap::build(root)
}

/// Execute the `catalog` command through the output pipeline.
pub fn execute(output: &Output) -> io::Result<()> {
    let catalog = build(&Cli::command());
    output.success("catalog", &catalog, &mut io::stdout())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_the_command_tree() {
        let catalog = build(&Cli::command());

        // Name is non-empty and derived from the command (rename-stable: a
        // derived project renames it, so we don't hardcode the scaffold name).
        assert!(!catalog.name.is_empty());

        // Every subcommand the parser knows is in the catalog — including
        // `catalog` itself (self-referential).
        let names: Vec<&str> = catalog.commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"ping"));
        assert!(names.contains(&"version"));
        assert!(names.contains(&"catalog"));

        // Required-core normalization: --output consumes a value, --verbose is a switch.
        let output = catalog
            .global_flags
            .iter()
            .find(|f| f.long == "output")
            .expect("output is a global flag");
        assert!(output.takes_value);
        let verbose = catalog
            .global_flags
            .iter()
            .find(|f| f.long == "verbose")
            .expect("verbose is a global flag");
        assert!(!verbose.takes_value);

        // --output is enumerated → optional possible_values derived.
        assert!(output.possible_values.contains(&"text".to_string()));

        // clap's auto --help is excluded.
        assert!(!catalog.global_flags.iter().any(|f| f.long == "help"));
    }
}
