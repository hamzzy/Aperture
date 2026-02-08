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

    /// Push collected data to this aggregator gRPC URL (e.g. http://127.0.0.1:50051)
    #[arg(long)]
    aggregator: Option<String>,
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

    // Low-overhead preset: reduce CPU and network usage (APERTURE_LOW_OVERHEAD=1)
    let low_overhead = std::env::var("APERTURE_LOW_OVERHEAD").as_deref() == Ok("1");
    let sample_rate_hz = if low_overhead && args.sample_rate == 99 {
        49
    } else {
        args.sample_rate
    };
    let push_interval_secs = if low_overhead { Some(10) } else { None };

    // Create configuration
    let config = Config {
        mode,
        target_pid: args.pid,
        sample_rate_hz,
        duration,
        output_path: args.output.clone(),
        json_output: args.json.clone(),
        filter_path: None,
        aggregator_url: args.aggregator,
        push_interval_secs,
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
    use tracing_subscriber::fmt;

    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let registry = tracing_subscriber::registry().with(filter);

    if std::env::var("APERTURE_LOG_FORMAT").as_deref() == Ok("json") {
        registry.with(fmt::layer().json().with_target(true)).init();
    } else {
        registry.with(fmt::layer().with_target(false)).init();
    }

    Ok(())
}
