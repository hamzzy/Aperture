//! eBPF Profiler Agent Library
//!
//! This library provides the core functionality for the profiling agent,
//! including eBPF program loading, event collection, and symbol resolution.

pub mod collector;
pub mod config;
pub mod ebpf;
pub mod output;

pub use config::Config;

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

/// Run the profiler with the given configuration.
///
/// This is the main entry point for profiling — it loads the eBPF program,
/// collects samples for the configured duration, resolves symbols, and
/// generates output files.
pub async fn run_profiler(config: Config) -> Result<()> {
    use aya::maps::{perf::AsyncPerfEventArray, StackTraceMap};
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use collector::cpu::{CpuCollector, SampleEvent};
    use collector::symbols::SymbolResolver;
    use ebpf::cpu_profiler::CpuProfiler;

    config.validate().context("Invalid configuration")?;

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

    profiler.set_target_pid(config.target_pid);

    profiler.start().context("Failed to start profiler")?;

    // 2. Set up event collector
    let collector = Arc::new(Mutex::new(CpuCollector::new(config.sample_period_ns())));

    // 3. Get maps for reading events and stacks
    let bpf = profiler.bpf_mut();

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
