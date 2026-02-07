//! eBPF Profiler Agent Library
//!
//! This library provides the core functionality for the profiling agent,
//! including eBPF program loading, event collection, and symbol resolution.

pub mod collector;
pub mod config;
pub mod ebpf;
pub mod output;
pub mod wasm;

pub use config::Config;
pub use config::ProfileMode;

use anyhow::{Context, Result};
use aperture_shared::protocol::wire::Message;
use aperture_shared::types::events::ProfileEvent;
use tracing::{debug, info};

/// Push collected events to the aggregator (Phase 5+).
async fn push_to_aggregator(
    url: &str,
    agent_id: &str,
    sequence: u64,
    events: &[ProfileEvent],
) -> Result<()> {
    use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
    use aperture_aggregator::server::grpc::proto::PushRequest;
    use tonic::transport::Channel;

    if events.is_empty() {
        return Ok(());
    }
    let message = Message::new(sequence, events.to_vec());
    let payload = message.to_bytes()?;
    let mut client = AggregatorClient::<Channel>::connect(url.to_string())
        .await
        .context("Failed to connect to aggregator")?;
    let req = PushRequest {
        agent_id: agent_id.to_string(),
        sequence,
        payload,
    };
    let res = client.push(tonic::Request::new(req)).await?;
    let inner = res.into_inner();
    if !inner.ok {
        anyhow::bail!("Aggregator push failed: {}", inner.error);
    }
    info!("Pushed {} events to aggregator at {}", events.len(), url);
    Ok(())
}

/// Run the profiler with the given configuration.
pub async fn run_profiler(config: Config) -> Result<()> {
    config.validate().context("Invalid configuration")?;

    match config.mode {
        config::ProfileMode::Cpu => run_cpu_profiler(config).await,
        config::ProfileMode::Lock => run_lock_profiler(config).await,
        config::ProfileMode::Syscall => run_syscall_profiler(config).await,
        config::ProfileMode::All => {
            // validating multiple modes concurrently might be tricky with eBPF resources (maps, etc)
            // For now, let's just run them sequentially or pick one?
            // The plan says "All->tokio::join! all three".
            // However, they all print to stdout/logs.
            // And they might need separate output files?
            // Existing config has single output_path.
            // I'll implement sequential or just error for now, or just implement CPU as fallback?
            // The plan says "All->tokio::join! all three".
            // But they share Config which has one output path.
            // We should probably derive output paths like "output.lock.svg", "output.cpu.svg".
            info!("Running all profilers concurrently");
            
            // Clone config for each
            let mut cpu_config = config.clone();
            cpu_config.output_path = format!("{}.cpu.svg", config.output_path);
            if let Some(ref json) = config.json_output {
                cpu_config.json_output = Some(format!("{}.cpu.json", json));
            }

            let mut lock_config = config.clone();
            lock_config.output_path = format!("{}.lock.svg", config.output_path);
            if let Some(ref json) = config.json_output {
                lock_config.json_output = Some(format!("{}.lock.json", json));
            }

            let mut syscall_config = config.clone();
            syscall_config.output_path = format!("{}.syscall.txt", config.output_path);
            if let Some(ref json) = config.json_output {
                syscall_config.json_output = Some(format!("{}.syscall.json", json));
            }

            let cpu_future = run_cpu_profiler(cpu_config);
            let lock_future = run_lock_profiler(lock_config);
            let syscall_future = run_syscall_profiler(syscall_config);

            let (cpu_res, lock_res, syscall_res) = tokio::join!(cpu_future, lock_future, syscall_future);
            
            cpu_res.context("CPU profiler failed")?;
            lock_res.context("Lock profiler failed")?;
            syscall_res.context("Syscall profiler failed")?;
            
            Ok(())
        }
    }
}

async fn run_cpu_profiler(config: Config) -> Result<()> {
    use aya::maps::{perf::AsyncPerfEventArray, StackTraceMap};
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use collector::cpu::{CpuCollector, SampleEvent};
    use collector::symbols::SymbolResolver;
    use ebpf::cpu_profiler::CpuProfiler;

    info!(
        "Profiling CPU for {} seconds at {} Hz",
        config.duration.as_secs(),
        config.sample_rate_hz
    );

    // 1. Load and start eBPF program
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

    // 4. Spawn per-CPU reader tasks
    let cpus = online_cpus().map_err(|(msg, e)| anyhow::anyhow!("{}: {}", msg, e))?;
    let mut handles = Vec::new();

    for cpu_id in cpus {
        let mut buf = perf_array.open(cpu_id, None)?;
        let collector = collector.clone();
        let stack_map = stack_map.clone();

        handles.push(tokio::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(core::mem::size_of::<SampleEvent>() + 64))
                .collect::<Vec<_>>();

            loop {
                match buf.read_events(&mut buffers).await {
                    Ok(events) => {
                        for i in 0..events.read {
                            let buf_ref = &buffers[i];
                            if buf_ref.len() >= core::mem::size_of::<SampleEvent>() {
                                let event = unsafe { &*(buf_ref.as_ptr() as *const SampleEvent) };
                                let mut coll = collector.lock().await;
                                if let Err(e) = coll.process_event(event, &stack_map) {
                                    debug!("Error processing event: {}", e);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }));
    }

    // 5. Wait
    tokio::time::sleep(config.duration).await;

    // 6. Cleanup — abort tasks and wait for them to drop their Arc clones
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }
    profiler.stop()?;

    // Drop the stack_map Arc so collector is the only one left
    drop(stack_map);

    let collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap collector Arc"))?
        .into_inner();
    let events = collector.profile_events();
    let mut profile = collector.build_profile()?;

    // 7. Symbolize & Output
    if profile.total_samples > 0 {
        let mut resolver = SymbolResolver::new();
        resolver.symbolize_profile(&mut profile, config.target_pid)?;
        output::flamegraph::generate_flamegraph(&profile, &config.output_path)?;
        
        if let Some(json_path) = &config.json_output {
            output::json::generate_json(&profile, json_path)?;
        }
    }

    if let Some(ref url) = config.aggregator_url {
        push_to_aggregator(url, "agent", 1, &events).await?;
    }

    Ok(())
}

async fn run_lock_profiler(config: Config) -> Result<()> {
    use aya::maps::{perf::AsyncPerfEventArray, StackTraceMap};
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use collector::lock::{LockCollector, LockEventBpf};
    use collector::symbols::SymbolResolver;
    use ebpf::lock_profiler::LockProfiler;

    info!("Profiling lock contention for {} seconds", config.duration.as_secs());

    let mut profiler = LockProfiler::new()?;
    profiler.set_target_pid(config.target_pid);
    profiler.start()?;

    let collector = Arc::new(Mutex::new(LockCollector::new()));
    let bpf = profiler.bpf_mut();
    
    let events_map = bpf.take_map("LOCK_EVENTS").context("Failed to get LOCK_EVENTS map")?;
    let mut perf_array = AsyncPerfEventArray::try_from(events_map)?;

    let stacks_map = bpf.take_map("LOCK_STACKS").context("Failed to get LOCK_STACKS map")?;
    let stack_map = Arc::new(StackTraceMap::try_from(stacks_map)?);

    let cpus = online_cpus().map_err(|(msg, e)| anyhow::anyhow!("{}: {}", msg, e))?;
    let mut handles = Vec::new();

    for cpu_id in cpus {
        let mut buf = perf_array.open(cpu_id, None)?;
        let collector = collector.clone();
        let stack_map = stack_map.clone();

        handles.push(tokio::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(core::mem::size_of::<LockEventBpf>() + 64))
                .collect::<Vec<_>>();

            loop {
                match buf.read_events(&mut buffers).await {
                    Ok(events) => {
                        for i in 0..events.read {
                            let buf_ref = &buffers[i];
                            if buf_ref.len() >= core::mem::size_of::<LockEventBpf>() {
                                let event = unsafe { &*(buf_ref.as_ptr() as *const LockEventBpf) };
                                let mut coll = collector.lock().await;
                                if let Err(e) = coll.process_event(event, &stack_map) {
                                    debug!("Error processing lock event: {}", e);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }));
    }


    tokio::time::sleep(config.duration).await;

    // Abort all tasks and wait for them to finish
    for handle in &handles {
        handle.abort();
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    profiler.stop();

    let collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();
    let events = collector.profile_events();
    let mut profile = collector.build_profile()?;

    if profile.total_events > 0 {
        let mut resolver = SymbolResolver::new();
        resolver.symbolize_lock_profile(&mut profile, config.target_pid)?;
        output::flamegraph::generate_lock_flamegraph(&profile, &config.output_path)?;
        
        if let Some(json_path) = &config.json_output {
            output::json::generate_lock_json(&profile, json_path)?;
        }
    }

    if let Some(ref url) = config.aggregator_url {
        push_to_aggregator(url, "agent", 1, &events).await?;
    }

    Ok(())
}

async fn run_syscall_profiler(config: Config) -> Result<()> {
    use aya::maps::perf::AsyncPerfEventArray;
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use collector::syscall::{SyscallCollector, SyscallEventBpf};
    use ebpf::syscall_tracer::SyscallTracer;

    info!("Tracing syscalls for {} seconds", config.duration.as_secs());

    let mut tracer = SyscallTracer::new()?;
    tracer.set_target_pid(config.target_pid);
    tracer.start()?;

    let collector = Arc::new(Mutex::new(SyscallCollector::new()));
    let bpf = tracer.bpf_mut();
    
    let events_map = bpf.take_map("SYSCALL_EVENTS").context("Failed to get SYSCALL_EVENTS map")?;
    let mut perf_array = AsyncPerfEventArray::try_from(events_map)?;

    let cpus = online_cpus().map_err(|(msg, e)| anyhow::anyhow!("{}: {}", msg, e))?;
    let mut handles = Vec::new();

    for cpu_id in cpus {
        let mut buf = perf_array.open(cpu_id, None)?;
        let collector = collector.clone();

        handles.push(tokio::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(core::mem::size_of::<SyscallEventBpf>() + 64))
                .collect::<Vec<_>>();

            loop {
                match buf.read_events(&mut buffers).await {
                    Ok(events) => {
                        for i in 0..events.read {
                            let buf_ref = &buffers[i];
                            if buf_ref.len() >= core::mem::size_of::<SyscallEventBpf>() {
                                let event = unsafe { &*(buf_ref.as_ptr() as *const SyscallEventBpf) };
                                let mut coll = collector.lock().await;
                                if let Err(e) = coll.process_event(event) {
                                    debug!("Error processing syscall event: {}", e);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }));
    }


    tokio::time::sleep(config.duration).await;

    // Abort all tasks and wait for them to finish
    for handle in &handles {
        handle.abort();
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    tracer.stop();

    let collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();
    let events = collector.profile_events();
    let profile = collector.build_profile()?;

    if profile.total_events > 0 {
        output::histogram::generate_syscall_histogram(&profile, &config.output_path)?;
        
        if let Some(json_path) = &config.json_output {
            output::json::generate_syscall_json(&profile, json_path)?;
        }
    }

    if let Some(ref url) = config.aggregator_url {
        push_to_aggregator(url, "agent", 1, &events).await?;
    }

    Ok(())
}
