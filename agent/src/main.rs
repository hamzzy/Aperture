//! eBPF Profiler Agent
//!
//! Main entry point for the profiling agent that loads and manages eBPF programs,
//! collects profiling data, and generates output (flamegraphs, JSON, etc.)

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{debug, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod collector;
mod config;
mod ebpf;
mod output;

use collector::cpu::{CpuCollector, SampleEvent};
use collector::symbols::SymbolResolver;
use config::Config;
use ebpf::cpu_profiler::CpuProfiler;

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
    let args = Args::parse();

    // Initialize tracing
    init_tracing(args.verbose)?;

    info!("Starting eBPF profiler agent");
    info!("Configuration: {:?}", args);

    // Parse duration
    let duration = aperture_shared::utils::parse_duration(&args.duration)
        .context("Failed to parse duration")?;

    // Create configuration
    let config = Config {
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
    use aya::maps::{perf::AsyncPerfEventArray, StackTraceMap};
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    info!(
        "Profiling {} for {} seconds at {} Hz",
        config
            .target_pid
            .map(|p| format!("PID {}", p))
            .unwrap_or_else(|| "all processes".to_string()),
        config.duration.as_secs(),
        config.sample_rate_hz
    );

    // 1. Load and start eBPF program
    info!("Loading eBPF CPU profiler");
    let mut profiler = CpuProfiler::new(config.sample_rate_hz)
        .context("Failed to create CPU profiler")?;

    profiler.start().context("Failed to start profiler")?;

    // 2. Set up event collector
    let sample_period_ns = 1_000_000_000 / config.sample_rate_hz;
    let collector = Arc::new(Mutex::new(CpuCollector::new(sample_period_ns)));

    // 3. Get maps for reading events and stacks
    let bpf = profiler.bpf_mut();

    // Debug: list all maps
    info!("Available maps:");
    for (name, _map) in bpf.maps() {
        info!("  map: {}", name);
    }

    let events_map = bpf.take_map("EVENTS").context("Failed to get EVENTS map")?;
    let mut perf_array = AsyncPerfEventArray::try_from(events_map)?;

    let stacks_map = bpf.take_map("STACKS").context("Failed to get STACKS map")?;
    let stack_map = Arc::new(StackTraceMap::try_from(stacks_map)?);

    info!(
        "Profiler started, collecting samples for {} seconds...",
        config.duration.as_secs()
    );

    // 4. Spawn per-CPU reader tasks
    let cpus = online_cpus().map_err(|(msg, e)| anyhow::anyhow!("{}: {}", msg, e))?;
    info!("Reading events from {} CPUs", cpus.len());

    let mut handles = Vec::new();

    for cpu_id in cpus {
        let mut buf = perf_array
            .open(cpu_id, None)
            .context(format!("Failed to open perf buffer for CPU {}", cpu_id))?;

        let collector_clone = collector.clone();
        let stack_map_clone = stack_map.clone();

        let handle = tokio::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(core::mem::size_of::<SampleEvent>() + 64))
                .collect::<Vec<_>>();

            let mut events_read: u64 = 0;

            loop {
                match buf.read_events(&mut buffers).await {
                    Ok(events) => {
                        for i in 0..events.read {
                            let buf_ref = &buffers[i];
                            if buf_ref.len() >= core::mem::size_of::<SampleEvent>() {
                                let event = unsafe {
                                    &*(buf_ref.as_ptr() as *const SampleEvent)
                                };
                                let mut coll = collector_clone.lock().await;
                                if let Err(e) = coll.process_event(event, &stack_map_clone) {
                                    debug!("Error processing event on CPU {}: {}", cpu_id, e);
                                }
                                events_read += 1;
                            }
                        }
                        if events.lost > 0 {
                            warn!("CPU {}: lost {} events", cpu_id, events.lost);
                        }
                    }
                    Err(e) => {
                        debug!("CPU {} perf buffer read error: {}", cpu_id, e);
                        break;
                    }
                }
            }

            events_read
        });

        handles.push(handle);
    }

    // 5. Wait for the profiling duration
    tokio::time::sleep(config.duration).await;

    info!("Collection period ended");

    // 6. Cancel reader tasks
    for handle in &handles {
        handle.abort();
    }

    let mut total_events: u64 = 0;
    for handle in handles {
        match handle.await {
            Ok(count) => total_events += count,
            Err(_) => {} // Task was aborted, that's expected
        }
    }

    // Stop profiler
    profiler.stop().context("Failed to stop profiler")?;

    let collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap collector"))?
        .into_inner();

    info!(
        "Collection complete. Read {} events, collected {} samples",
        total_events,
        collector.sample_count()
    );

    // 7. Build profile from collected samples
    let mut profile = collector
        .build_profile()
        .context("Failed to build profile")?;

    info!("Profile built with {} unique stacks", profile.samples.len());

    // 8. Symbolize the profile
    if profile.total_samples > 0 {
        info!("Resolving symbols...");
        let mut resolver = SymbolResolver::new();
        resolver
            .symbolize_profile(&mut profile, config.target_pid)
            .context("Failed to symbolize profile")?;

        info!("Resolved {} symbols", resolver.cache_size());
    } else {
        warn!("No samples collected - check if profiler has permissions");
    }

    // 9. Generate flamegraph output
    if profile.total_samples > 0 {
        info!("Generating flamegraph...");
        output::flamegraph::generate_flamegraph(&profile, &config.output_path)
            .context("Failed to generate flamegraph")?;

        info!("Flamegraph written to: {}", config.output_path);

        // 10. Generate JSON output if requested
        if let Some(json_path) = &config.json_output {
            info!("Generating JSON output...");
            output::json::generate_json(&profile, json_path)
                .context("Failed to generate JSON output")?;
            info!("JSON output written to: {}", json_path);
        }

        info!("Profiling complete!");
        info!("Total samples: {}", profile.total_samples);
        info!("Unique stacks: {}", profile.samples.len());
        info!(
            "Duration: {:.2}s",
            profile.duration_ns() as f64 / 1_000_000_000.0
        );
    } else {
        warn!("No samples collected - no output generated");
        warn!("Make sure you're running as root and the target process is active");
    }

    Ok(())
}
