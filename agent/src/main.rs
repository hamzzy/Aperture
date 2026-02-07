//! eBPF Profiler Agent
//!
//! Main entry point for the profiling agent binary.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aperture_agent::Config;

#[derive(Parser, Debug)]
#[command(name = "profiler-agent")]
#[command(about = "eBPF-based CPU profiler", long_about = None)]
#[command(version)]
struct Args {
    /// Profiling mode (cpu, lock, syscall, all)
    #[arg(short, long, default_value = "cpu")]
    mode: String,

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
    let args = Args::parse();

    // Initialize tracing
    init_tracing(args.verbose)?;

    info!("Starting eBPF profiler agent");
    info!("Configuration: {:?}", args);

    // Parse duration
    let duration = aperture_shared::utils::parse_duration(&args.duration)
        .context("Failed to parse duration")?;

    // Parse mode
    use std::str::FromStr;
    use aperture_agent::ProfileMode;
    let mode = ProfileMode::from_str(&args.mode)?;

    // Create configuration
    let config = Config {
        mode,
        target_pid: args.pid,
        sample_rate_hz: args.sample_rate,
        duration,
        output_path: args.output.clone(),
        json_output: args.json.clone(),
    };

    // Check if running as root (required for eBPF)
    #[cfg(target_os = "linux")]
    unsafe {
        if libc::geteuid() != 0 {
            warn!("Warning: Not running as root. eBPF programs require root privileges.");
            warn!("Try: sudo {}", std::env::current_exe()?.display());
        }
    }

    // Run profiler
    aperture_agent::run_profiler(config).await
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
