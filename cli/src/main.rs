//! CLI for Aperture
//!
//! This is a higher-level CLI that supports multiple commands:
//! - profile: Run profiling (wraps agent)
//! - query: Query aggregated data (Phase 5+)

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod commands;
#[allow(dead_code)]
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
        Commands::Profile(args) => {
            init_tracing(args.verbose);
            commands::profile::run(args).await
        }
        Commands::Query(args) => commands::query::run(args).await,
    }
}

fn init_tracing(verbose: bool) {
    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
