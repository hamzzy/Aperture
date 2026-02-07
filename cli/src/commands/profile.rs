//! Profile command implementation

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args, Debug)]
pub struct ProfileArgs {
    /// Profiling mode (cpu, lock, syscall, all)
    #[arg(short, long, default_value = "cpu")]
    pub mode: String,

    /// Process ID to profile
    #[arg(short, long)]
    pub pid: Option<i32>,

    /// Duration to profile (e.g., "30s", "5m")
    #[arg(short, long, default_value = "30s")]
    pub duration: String,

    /// Sampling frequency in Hz
    #[arg(short, long, default_value = "99")]
    pub sample_rate: u64,

    /// Output file for flamegraph (SVG format)
    #[arg(short, long, default_value = "flamegraph.svg")]
    pub output: String,

    /// Also output raw data in JSON format
    #[arg(long)]
    pub json: Option<String>,

    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn run(args: ProfileArgs) -> Result<()> {
    // Parse duration
    let duration = aperture_shared::utils::parse_duration(&args.duration)
        .context("Failed to parse duration")?;

    // Parse mode
    use std::str::FromStr;
    use aperture_agent::ProfileMode;
    let mode = ProfileMode::from_str(&args.mode)?;

    let config = aperture_agent::Config {
        mode,
        target_pid: args.pid,
        sample_rate_hz: args.sample_rate,
        duration,
        output_path: args.output,
        json_output: args.json,
    };

    aperture_agent::run_profiler(config).await
}
