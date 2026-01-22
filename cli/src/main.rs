//! CLI for Aperture
//!
//! This is a higher-level CLI that will eventually support multiple commands:
//! - profile: Run profiling (wraps agent)
//! - query: Query aggregated data (Phase 5+)
//! - analyze: Analyze profile data
//! - export: Export profiles in various formats

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod output;

#[derive(Parser)]
#[command(name = "aperture")]
#[command(about = "Aperture - eBPF-based profiler", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run profiling on a process or system
    Profile(commands::profile::ProfileArgs),

    /// Query aggregated profiling data (Phase 5+)
    #[command(hide = true)]
    Query(commands::query::QueryArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Profile(args) => commands::profile::run(args).await,
        Commands::Query(args) => commands::query::run(args).await,
    }
}
