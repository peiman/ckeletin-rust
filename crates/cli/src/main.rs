//! Entry point — bootstrap only (CKSPEC-ARCH-006).
//! All logic lives in domain and infrastructure crates.

mod ping;
mod root;

use ckeletin_infrastructure::{
    config::Config,
    logging::{self, LogConfig},
    output::{Output, OutputMode},
};
use clap::Parser;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    // Parse CLI args first — we need to know the output format
    // before we can route errors correctly.
    let cli = root::Cli::parse();
    let json_mode = matches!(cli.output, root::OutputFormat::Json);

    match run_inner(cli) {
        Ok(()) => 0,
        Err(e) => {
            // CKSPEC-OUT-002: errors in JSON mode MUST be JSON envelopes on stdout.
            // Errors in human mode go to stderr.
            let output = Output::new(if json_mode {
                OutputMode::Json
            } else {
                OutputMode::Human
            });
            let _ = output.error(
                "init",
                &e.to_string(),
                &mut std::io::stdout(),
                &mut std::io::stderr(),
            );
            1
        }
    }
}

fn run_inner(cli: root::Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration (defaults → file → env)
    let config = Config::load(cli.config.as_deref())?;

    // Determine output mode: CLI flag overrides config.
    // --output json on CLI takes precedence. Config json=true is fallback.
    let json_mode = matches!(cli.output, root::OutputFormat::Json) || config.json;

    // Determine log level: --verbose overrides config
    let log_level = if cli.verbose {
        "debug".to_string()
    } else {
        config.log_level.clone()
    };

    // Initialize logging — suppress stderr in JSON mode for clean output
    let log_config = LogConfig {
        console_level: if json_mode {
            "off".to_string()
        } else {
            log_level
        },
        file_enabled: config.log_file_enabled,
        file_path: config.log_file_path.clone(),
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
