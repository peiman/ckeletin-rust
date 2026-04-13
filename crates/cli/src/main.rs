//! Entry point — bootstrap only (CKSPEC-ARCH-006).
//! All logic lives in domain and infrastructure crates.

mod ping;
mod root;

use clap::Parser;
use ckeletin_infrastructure::{
    config::Config,
    logging::{self, LogConfig},
    output::{Output, OutputMode},
};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    match run_inner() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    let cli = root::Cli::parse();
    let config = Config::load(cli.config.as_deref())?;

    // Determine output mode: CLI flag overrides config
    let json_mode = cli.json || config.json;

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
