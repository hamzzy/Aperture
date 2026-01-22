//! eBPF Profiler Agent
//!
//! Main entry point for the profiling agent that loads and manages eBPF programs,
//! collects profiling data, and generates output (flamegraphs, JSON, etc.)

use anyhow::Result;
use clap::Parser;
use color_eyre::eyre::WrapErr;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod collector;
mod config;
mod ebpf;
mod output;

use config::Config;

#[derive(Parser, Debug)]
#[command(name = "profiler-agent")]
#[command(about = "eBPF-based CPU profiler", long_about = None)]
#[command(version)]
struct Args {
    /// Process ID to profile (default: profile all processes)
    #[arg(short, long)]
    pid: Option<i32>,

    /// Duration to profile (e.g., "30s", "5m", "1h")
    #[arg(short, long, default_value = "30s")]
    duration: String,

    /// Sampling frequency in Hz
    #[arg(short, long, default_value = "99")]
    sample_rate: u64,

    /// Output file for flamegraph (SVG format)
    #[arg(short, long, default_value = "flamegraph.svg")]
    output: String,

    /// Also output raw data in JSON format
    #[arg(long)]
    json: Option<String>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    // Initialize tracing
    init_tracing(args.verbose)?;

    info!("Starting eBPF profiler agent");
    info!("Configuration: {:?}", args);

    // Parse duration
    let duration = shared::utils::parse_duration(&args.duration)
        .wrap_err("Failed to parse duration")?;

    // Create configuration
    let config = Config {
        target_pid: args.pid,
        sample_rate_hz: args.sample_rate,
        duration,
        output_path: args.output.clone(),
        json_output: args.json.clone(),
    };

    // Check if running as root (required for eBPF)
    if !nix::unistd::Uid::effective().is_root() {
        warn!("Warning: Not running as root. eBPF programs require root privileges.");
        warn!("Try: sudo {}", std::env::current_exe()?.display());
    }

    // Run profiler
    run_profiler(config).await
}

/// Initialize tracing/logging
fn init_tracing(verbose: bool) -> Result<()> {
    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    Ok(())
}

/// Main profiler logic
async fn run_profiler(config: Config) -> Result<()> {
    info!(
        "Profiling {} for {} seconds at {} Hz",
        config
            .target_pid
            .map(|p| format!("PID {}", p))
            .unwrap_or_else(|| "all processes".to_string()),
        config.duration.as_secs(),
        config.sample_rate_hz
    );

    // TODO Phase 1: Implement profiler workflow
    // 1. Load eBPF program using ebpf::loader
    // 2. Attach to perf events
    // 3. Start collection using collector::cpu
    // 4. Wait for duration
    // 5. Process samples and resolve symbols using collector::symbols
    // 6. Generate output using output::flamegraph and output::json

    warn!("TODO: Implement eBPF program loading and profiling logic");
    warn!("See agent/src/ebpf/loader.rs and agent/src/collector/cpu.rs");

    // Placeholder: sleep for the duration
    tokio::time::sleep(config.duration).await;

    info!("Profiling complete");
    info!("Output written to: {}", config.output_path);

    Ok(())
}
