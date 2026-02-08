//! eBPF Profiler Agent Library
//!
//! This library provides the core functionality for the profiling agent,
//! including eBPF program loading, event collection, and symbol resolution.

pub mod collector;
pub mod config;
pub mod ebpf;
pub mod output;
pub mod retry;
pub mod wasm;

pub use config::Config;
pub use config::ProfileMode;

use anyhow::{Context, Result};
use aperture_shared::protocol::wire::Message;
use aperture_shared::types::events::ProfileEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Global monotonic sequence counter for aggregator pushes.
static PUSH_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// How often to stream pending events to the aggregator during profiling.
const PUSH_INTERVAL: Duration = Duration::from_secs(5);
const PUSH_INTERVAL_MAX: Duration = Duration::from_secs(30);

/// Generate an agent ID from the hostname (or fallback to PID).
fn agent_id() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| format!("agent-{}", std::process::id()))
}

/// Max gRPC message size for push (default 32 MiB). Must be <= server limit (APERTURE_MAX_MESSAGE_SIZE_MB).
fn max_message_size_bytes() -> usize {
    std::env::var("APERTURE_MAX_MESSAGE_SIZE_MB")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(32)
        .saturating_mul(1024 * 1024)
}

/// When the server returns "message length too large", the agent splits the batch and retries (no pre-size check).

/// gRPC request timeout (default 120s). Large pushes (e.g. millions of syscall events) can take a long time.
fn grpc_timeout() -> Duration {
    std::env::var("APERTURE_GRPC_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(120))
}

/// Connect to the aggregator with timeouts. Used for connection reuse and reconnects.
async fn connect_aggregator(
    url: &str,
) -> Result<
    aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient<
        tonic::transport::Channel,
    >,
    anyhow::Error,
> {
    use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
    use tonic::codec::CompressionEncoding;
    use tonic::transport::Channel;

    let channel = Channel::from_shared(url.to_string())?
        .connect_timeout(Duration::from_secs(5))
        .timeout(grpc_timeout())
        .connect()
        .await
        .context("Failed to connect to aggregator")?;
    let max_bytes = max_message_size_bytes();
    Ok(AggregatorClient::new(channel)
        .max_encoding_message_size(max_bytes)
        .max_decoding_message_size(max_bytes)
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip))
}

/// Push a single batch (payload must be within size limit). Returns Ok(Some(backpressure)) when a push
/// was performed, Ok(None) when events were empty.
async fn push_with_client(
    client: &mut aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient<
        tonic::transport::Channel,
    >,
    agent_id: &str,
    events: Vec<ProfileEvent>,
) -> Result<Option<bool>, anyhow::Error> {
    use aperture_aggregator::server::grpc::proto::PushRequest;

    if events.is_empty() {
        return Ok(None);
    }
    let count = events.len();
    let sequence = PUSH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let message = Message::new(sequence, events);
    let payload = message.to_bytes()?;
    let req = PushRequest {
        agent_id: agent_id.to_string(),
        sequence,
        payload,
    };
    let mut request = tonic::Request::new(req);
    if let Ok(token) = std::env::var("APERTURE_AUTH_TOKEN") {
        let value = format!("Bearer {}", token);
        if let Ok(v) = value.parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>() {
            request.metadata_mut().insert("authorization", v);
        }
    }
    let res = client.push(request).await?;
    let inner = res.into_inner();
    if !inner.ok {
        anyhow::bail!("Aggregator push failed: {}", inner.error);
    }
    info!("Pushed {} events (seq={}) to aggregator", count, sequence);
    Ok(Some(inner.backpressure))
}

/// Returns true if the error indicates the message was too large for the server.
fn is_message_too_large(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("message length too large")
        || msg.contains("Message too large")
        || msg.contains("OutOfRange")
}

/// Push a batch of events to the aggregator with retry and optional client reuse.
/// If the server rejects due to message size, splits the batch and retries in a loop (no recursion).
/// Returns Ok(Some(backpressure)) when a push was performed, Ok(None) when events were empty.
async fn push_to_aggregator(
    client: &mut Option<
        aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient<
            tonic::transport::Channel,
        >,
    >,
    url: &str,
    agent_id: &str,
    events: Vec<ProfileEvent>,
) -> Result<Option<bool>, anyhow::Error> {
    if events.is_empty() {
        return Ok(None);
    }
    if client.is_none() {
        *client = Some(connect_aggregator(url).await?);
    }
    let mut queue = std::collections::VecDeque::from([events]);
    let mut last_backpressure = None;
    while let Some(chunk) = queue.pop_front() {
        if chunk.is_empty() {
            continue;
        }
        let c = client.as_mut().unwrap();
        match push_with_client(c, agent_id, chunk.clone()).await {
            Ok(b) => {
                last_backpressure = b;
            }
            Err(e) => {
                if is_message_too_large(&e) && chunk.len() > 1 {
                    let mid = chunk.len() / 2;
                    let (first, second) = chunk.split_at(mid);
                    queue.push_front(second.to_vec());
                    queue.push_front(first.to_vec());
                    continue;
                }
                let msg = e.to_string();
                let is_connection_error = msg.contains("connection")
                    || msg.contains("Connection")
                    || msg.contains("unavailable");
                if is_connection_error {
                    *client = None;
                }
                return Err(e);
            }
        }
    }
    Ok(last_backpressure)
}

/// Push with up to 3 attempts (exponential backoff). Keeps `client` in scope so it can be mutated.
async fn push_to_aggregator_with_retry(
    client: &mut Option<
        aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient<
            tonic::transport::Channel,
        >,
    >,
    url: &str,
    agent_id: &str,
    events: Vec<ProfileEvent>,
) -> Result<Option<bool>, anyhow::Error> {
    let mut delay = Duration::from_millis(500);
    for attempt in 1..=3 {
        match push_to_aggregator(client, url, agent_id, events.clone()).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                warn!("aggregator push failed (attempt {}/3): {}", attempt, e);
                if attempt < 3 {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(PUSH_INTERVAL_MAX);
                } else {
                    return Err(e);
                }
            }
        }
    }
    unreachable!()
}

/// Check symbol resolution prerequisites and warn about common issues.
fn check_symbol_prerequisites(pid: Option<i32>) {
    // Check kptr_restrict — if non-zero, kernel symbols resolve as hex
    match std::fs::read_to_string("/proc/sys/kernel/kptr_restrict") {
        Ok(val) => {
            let v = val.trim();
            if v != "0" {
                warn!(
                    "kernel.kptr_restrict={} — kernel symbols will show as hex addresses. \
                     Fix: sudo sysctl kernel.kptr_restrict=0",
                    v
                );
            } else {
                debug!("kernel.kptr_restrict=0 (kernel symbols accessible)");
            }
        }
        Err(e) => {
            debug!("Could not read /proc/sys/kernel/kptr_restrict: {}", e);
        }
    }

    // Check /proc/kallsyms readability
    match std::fs::read_to_string("/proc/kallsyms") {
        Ok(content) => {
            let has_real_addrs = content
                .lines()
                .take(5)
                .any(|l| !l.starts_with("0000000000000000"));
            if !has_real_addrs {
                warn!(
                    "/proc/kallsyms shows zeroed addresses — kernel symbols won't resolve. \
                     Ensure running as root with kptr_restrict=0"
                );
            } else {
                let count = content.lines().count();
                info!(
                    "Kernel symbols available: {} entries in /proc/kallsyms",
                    count
                );
            }
        }
        Err(e) => warn!(
            "Cannot read /proc/kallsyms: {} — kernel symbols unavailable",
            e
        ),
    }

    // Check target process /proc/PID/maps
    if let Some(pid) = pid {
        let maps_path = format!("/proc/{}/maps", pid);
        match std::fs::read_to_string(&maps_path) {
            Ok(content) => {
                let mappings = content.lines().count();
                info!(
                    "Process {} maps available: {} memory mappings",
                    pid, mappings
                );
            }
            Err(e) => warn!(
                "Cannot read {} — userspace symbols for PID {} unavailable: {}",
                maps_path, pid, e
            ),
        }
    }
}

/// Run the profiler with the given configuration.
pub async fn run_profiler(config: Config) -> Result<()> {
    config.validate().context("Invalid configuration")?;

    // Check symbol resolution prerequisites before profiling
    check_symbol_prerequisites(config.target_pid);

    match config.mode {
        config::ProfileMode::Cpu => run_cpu_profiler(config).await,
        config::ProfileMode::Lock => run_lock_profiler(config).await,
        config::ProfileMode::Syscall => run_syscall_profiler(config).await,
        config::ProfileMode::All => {
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

            let (cpu_res, lock_res, syscall_res) =
                tokio::join!(cpu_future, lock_future, syscall_future);

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
    use collector::symbols::{SymbolCache, SymbolResolver};
    use ebpf::cpu_profiler::CpuProfiler;

    info!(
        "Profiling CPU for {} seconds at {} Hz",
        config.duration.as_secs(),
        config.sample_rate_hz
    );

    // 1. Load and start eBPF program
    let mut profiler =
        CpuProfiler::new(config.sample_rate_hz).context("Failed to create CPU profiler")?;

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

    // 5. Spawn streaming push task if aggregator is configured
    let target_pid = config.target_pid;
    let push_handle = if let Some(ref url) = config.aggregator_url {
        let url = url.clone();
        let agent = agent_id();
        let coll = collector.clone();
        let initial_interval = config.push_interval();
        Some(tokio::spawn(async move {
            let mut client = None;
            let mut push_interval = initial_interval;
            let mut sym_cache = SymbolCache::new();
            loop {
                tokio::time::sleep(push_interval).await;
                let mut events = coll.lock().await.take_pending_events();
                sym_cache.symbolize_events(&mut events, target_pid);
                let result = push_to_aggregator_with_retry(&mut client, &url, &agent, events).await;
                match result {
                    Ok(Some(true)) => {
                        push_interval = (push_interval + push_interval).min(PUSH_INTERVAL_MAX)
                    }
                    Ok(Some(false)) | Ok(None) => push_interval = initial_interval,
                    Err(e) => warn!("Streaming push failed: {}", e),
                }
            }
        }))
    } else {
        None
    };

    // 6. Wait for profiling duration
    tokio::time::sleep(config.duration).await;

    // 7. Cleanup — abort reader tasks and streaming push, wait for Arc cleanup
    if let Some(h) = push_handle {
        h.abort();
        let _ = h.await;
    }
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }
    profiler.stop()?;

    // Drop the stack_map Arc so collector is the only one left
    drop(stack_map);

    let mut collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap collector Arc"))?
        .into_inner();

    // Final push of any remaining events (with symbolization)
    if let Some(ref url) = config.aggregator_url {
        let mut client = None;
        let mut events = collector.take_pending_events();
        let mut sym_cache = SymbolCache::new();
        sym_cache.symbolize_events(&mut events, config.target_pid);
        let _ = push_to_aggregator_with_retry(&mut client, url, &agent_id(), events).await;
    }

    let mut profile = collector.build_profile()?;

    // 8. Symbolize & Output
    if profile.total_samples > 0 {
        let mut resolver = SymbolResolver::new();
        resolver.symbolize_profile(&mut profile, config.target_pid)?;
        output::flamegraph::generate_flamegraph(&profile, &config.output_path)?;

        if let Some(json_path) = &config.json_output {
            output::json::generate_json(&profile, json_path)?;
        }
    }

    Ok(())
}

async fn run_lock_profiler(config: Config) -> Result<()> {
    use aya::maps::{perf::AsyncPerfEventArray, StackTraceMap};
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use collector::lock::{LockCollector, LockEventBpf};
    use collector::symbols::{SymbolCache, SymbolResolver};
    use ebpf::lock_profiler::LockProfiler;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    info!(
        "Profiling lock contention for {} seconds",
        config.duration.as_secs()
    );

    let mut profiler = LockProfiler::new()?;
    profiler.set_target_pid(config.target_pid);
    profiler.start()?;

    let collector = Arc::new(Mutex::new(LockCollector::new()));
    let bpf = profiler.bpf_mut();

    let events_map = bpf
        .take_map("LOCK_EVENTS")
        .context("Failed to get LOCK_EVENTS map")?;
    let mut perf_array = AsyncPerfEventArray::try_from(events_map)?;

    let stacks_map = bpf
        .take_map("LOCK_STACKS")
        .context("Failed to get LOCK_STACKS map")?;
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

    // Spawn streaming push task if aggregator is configured
    let target_pid = config.target_pid;
    let push_handle = if let Some(ref url) = config.aggregator_url {
        let url = url.clone();
        let agent = agent_id();
        let coll = collector.clone();
        let initial_interval = config.push_interval();
        Some(tokio::spawn(async move {
            let mut client = None;
            let mut push_interval = initial_interval;
            let mut sym_cache = SymbolCache::new();
            loop {
                tokio::time::sleep(push_interval).await;
                let mut events = coll.lock().await.take_pending_events();
                sym_cache.symbolize_events(&mut events, target_pid);
                let result = push_to_aggregator_with_retry(&mut client, &url, &agent, events).await;
                match result {
                    Ok(Some(true)) => {
                        push_interval = (push_interval + push_interval).min(PUSH_INTERVAL_MAX)
                    }
                    Ok(Some(false)) | Ok(None) => push_interval = initial_interval,
                    Err(e) => warn!("Streaming push failed: {}", e),
                }
            }
        }))
    } else {
        None
    };

    tokio::time::sleep(config.duration).await;

    // Cleanup
    if let Some(h) = push_handle {
        h.abort();
        let _ = h.await;
    }
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }
    profiler.stop();

    let mut collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();

    // Final push of remaining events (with symbolization)
    if let Some(ref url) = config.aggregator_url {
        let mut client = None;
        let mut events = collector.take_pending_events();
        let mut sym_cache = SymbolCache::new();
        sym_cache.symbolize_events(&mut events, config.target_pid);
        let _ = push_to_aggregator_with_retry(&mut client, url, &agent_id(), events).await;
    }

    let mut profile = collector.build_profile()?;

    if profile.total_events > 0 {
        let mut resolver = SymbolResolver::new();
        resolver.symbolize_lock_profile(&mut profile, config.target_pid)?;
        output::flamegraph::generate_lock_flamegraph(&profile, &config.output_path)?;

        if let Some(json_path) = &config.json_output {
            output::json::generate_lock_json(&profile, json_path)?;
        }
    }

    Ok(())
}

async fn run_syscall_profiler(config: Config) -> Result<()> {
    use aya::maps::perf::AsyncPerfEventArray;
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use collector::syscall::{SyscallCollector, SyscallEventBpf};
    use ebpf::syscall_tracer::SyscallTracer;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    info!("Tracing syscalls for {} seconds", config.duration.as_secs());

    let mut tracer = SyscallTracer::new()?;
    tracer.set_target_pid(config.target_pid);
    tracer.start()?;

    let collector = Arc::new(Mutex::new(SyscallCollector::new()));
    let bpf = tracer.bpf_mut();

    let events_map = bpf
        .take_map("SYSCALL_EVENTS")
        .context("Failed to get SYSCALL_EVENTS map")?;
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
                                let event =
                                    unsafe { &*(buf_ref.as_ptr() as *const SyscallEventBpf) };
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

    // Spawn streaming push task if aggregator is configured
    let push_handle = if let Some(ref url) = config.aggregator_url {
        let url = url.clone();
        let agent = agent_id();
        let coll = collector.clone();
        let initial_interval = config.push_interval();
        Some(tokio::spawn(async move {
            let mut client = None;
            let mut push_interval = initial_interval;
            loop {
                tokio::time::sleep(push_interval).await;
                let events = coll.lock().await.take_pending_events();
                let result = push_to_aggregator_with_retry(&mut client, &url, &agent, events).await;
                match result {
                    Ok(Some(true)) => {
                        push_interval = (push_interval + push_interval).min(PUSH_INTERVAL_MAX)
                    }
                    Ok(Some(false)) | Ok(None) => push_interval = initial_interval,
                    Err(e) => warn!("Streaming push failed: {}", e),
                }
            }
        }))
    } else {
        None
    };

    tokio::time::sleep(config.duration).await;

    // Cleanup
    if let Some(h) = push_handle {
        h.abort();
        let _ = h.await;
    }
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }
    tracer.stop();

    let mut collector = Arc::try_unwrap(collector)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();

    // Final push of remaining events
    if let Some(ref url) = config.aggregator_url {
        let mut client = None;
        let events = collector.take_pending_events();
        let _ = push_to_aggregator_with_retry(&mut client, url, &agent_id(), events).await;
    }

    let profile = collector.build_profile()?;

    if profile.total_events > 0 {
        output::histogram::generate_syscall_histogram(&profile, &config.output_path)?;

        if let Some(json_path) = &config.json_output {
            output::json::generate_syscall_json(&profile, json_path)?;
        }
    }

    Ok(())
}
