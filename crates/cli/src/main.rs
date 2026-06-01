//! Entry point — bootstrap only (CKSPEC-ARCH-006).
//! All logic lives in domain and infrastructure crates.

mod ping;
mod root;

use clap::Parser;
use infrastructure::{
    config::Config,
    logging::{self, LogConfig},
    output::{Output, OutputMode},
};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    // Parse CLI args first — we need to know the output format
    // before we can route errors correctly.
    let cli = root::Cli::parse();
    let json_mode = matches!(cli.output, root::OutputFormat::Json);
    // Capture the subcommand name BEFORE moving `cli` into `run_inner`,
    // so the error envelope can identify which subcommand failed
    // (CKSPEC-OUT-003). Earned 2026-04-22 — prior versions hardcoded
    // "init", producing `{"command":"init"}` for every failing
    // subcommand regardless of which one was running.
    let cmd_name = subcommand_name(&cli.command);

    match run_inner(cli) {
        Ok(()) => 0,
        Err(e) => {
            // CKSPEC-OUT-002: errors in JSON mode MUST be JSON envelopes on stdout.
            // Errors in human mode go to stderr.
            // CKSPEC-OUT-003: the envelope MUST identify the failing subcommand.
            let output = Output::new(if json_mode {
                OutputMode::Json
            } else {
                OutputMode::Human
            });
            let _ = output.error(
                cmd_name,
                &e.to_string(),
                &mut std::io::stdout(),
                &mut std::io::stderr(),
            );
            1
        }
    }
}

/// Map a parsed `Commands` variant to its CLI-visible name. A plain
/// `match` so adding a new subcommand is a compile error here until a
/// name is assigned — no silent "init" fallback. Consumers of ckeletin
/// extend this alongside their own `root::Commands` additions.
fn subcommand_name(command: &root::Commands) -> &'static str {
    match command {
        root::Commands::Ping => "ping",
    }
}

fn run_inner(cli: root::Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration (defaults → file → env)
    let config = Config::load(cli.config.as_deref(), "CKELETIN_")?;

    // Determine output mode: CLI flag overrides config.
    // --output json on CLI takes precedence. Config json=true is fallback.
    let json_mode = matches!(cli.output, root::OutputFormat::Json) || config.json;

    // Determine log level: --verbose overrides config
    let log_level = if cli.verbose {
        "debug".to_string()
    } else {
        config.log_level.clone()
    };

    // Audit log (CKSPEC-OUT-004) is on by default; --no-audit turns it off for
    // this run. The path is resolved to a stable per-user location (default
    // ~/.config/<app>/logs/app.log) so it doesn't depend on the cwd.
    let audit_enabled = config.log_file_enabled && !cli.no_audit;
    let audit_path = logging::resolve_audit_path(
        &config.log_file_path,
        &config.log_location,
        env!("CARGO_BIN_NAME"),
    );

    // First-run heads-up: tell the user once — when the audit log directory is
    // first created — that we're writing it and how to turn it off. Goes to the
    // status stream (stderr), human mode only; silent in JSON mode and on every
    // later run.
    if audit_enabled && !json_mode {
        let first_run = audit_path
            .parent()
            .is_some_and(|dir| !dir.as_os_str().is_empty() && !dir.exists());
        if first_run {
            eprintln!(
                "note: writing an audit log to {} (this notice won't repeat; \
                 disable with --no-audit or log_file_enabled=false in config)",
                audit_path.display()
            );
        }
    }

    // Initialize logging — suppress stderr in JSON mode for clean output
    let log_config = LogConfig {
        console_level: if json_mode {
            "off".to_string()
        } else {
            log_level
        },
        file_enabled: audit_enabled,
        file_path: audit_path.to_string_lossy().into_owned(),
        file_level: config.log_file_level.clone(),
    };
    let _guard = logging::init(&log_config)?;

    let output = Output::new(if json_mode {
        OutputMode::Json
    } else {
        OutputMode::Human
    });

    // Dispatch to command handler
    match cli.command {
        root::Commands::Ping => ping::execute(&output)?,
    }

    Ok(())
}
