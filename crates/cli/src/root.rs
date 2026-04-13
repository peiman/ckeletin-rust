use clap::{Parser, Subcommand};

/// A production-ready Rust CLI built with ckeletin-rust.
#[derive(Parser, Debug)]
#[command(name = "ckeletin-rust", version, about)]
pub struct Cli {
    /// Output in JSON format (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Enable verbose output (debug level)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check connectivity — returns pong
    Ping,
}
