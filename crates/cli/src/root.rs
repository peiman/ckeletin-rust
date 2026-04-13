use clap::{Parser, Subcommand, ValueEnum};

/// A production-ready Rust CLI built with ckeletin-rust.
#[derive(Parser, Debug)]
#[command(name = "ckeletin-rust", version, about)]
pub struct Cli {
    /// Output format: text (human-readable) or json (machine-readable)
    #[arg(long, global = true, default_value = "text")]
    pub output: OutputFormat,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Enable verbose output (debug level)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format selection (CKSPEC-OUT-002).
/// Matches ckeletin-go convention: --output text|json
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check connectivity — returns pong
    Ping,
}
