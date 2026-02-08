//! eBPF program loader
//!
//! Loads and attaches eBPF programs to the kernel

use anyhow::{Context, Result};
use aya::{
    programs::{
        PerfEvent,
        perf_event::{PerfEventLinkId, PerfTypeId, PerfEventScope, SamplePolicy},
    },
    util::online_cpus,
    Ebpf,
};
use tracing::info;

/// Get the device and inode numbers for the current PID namespace.
/// These are needed by `bpf_get_ns_current_pid_tgid()` to resolve
/// namespace-relative PIDs in eBPF programs.
fn get_pidns_dev_ino() -> Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata("/proc/self/ns/pid")
        .context("Failed to stat /proc/self/ns/pid")?;
    Ok((meta.dev(), meta.ino()))
}

/// Storage for perf event links to keep them alive
pub struct PerfEventLinks {
    links: Vec<PerfEventLinkId>,
}

impl PerfEventLinks {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add(&mut self, link: PerfEventLinkId) {
        self.links.push(link);
    }
}

/// Load the CPU profiler eBPF program
pub fn load_cpu_profiler() -> Result<Ebpf> {
    use aya::EbpfLoader;

    info!("Loading CPU profiler eBPF program");

    // For debugging, try loading from file first
    #[cfg(debug_assertions)]
    {
        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../target/bpfel-unknown-none/debug/cpu-profiler");

        info!("Loading eBPF from file: {:?}", path);

        if path.exists() {
            let bpf = EbpfLoader::new()
                .load_file(&path)
                .context("Failed to load eBPF program from file")?;
            info!("Successfully loaded CPU profiler eBPF program from file");
            return Ok(bpf);
        } else {
            anyhow::bail!("eBPF program file not found: {:?}", path);
        }
    }

    // Release: try file first (Docker copies eBPF to /usr/local/share/aperture), then embedded
    #[cfg(not(debug_assertions))]
    {
        let file_paths = [
            std::path::Path::new("/usr/local/share/aperture/cpu-profiler"),
            std::path::Path::new("cpu-profiler"),
        ];
        if let Some(path) = std::env::var_os("APERTURE_EBPF_CPU_PROFILER").map(std::path::PathBuf::from) {
            if path.exists() {
                info!("Loading eBPF from APERTURE_EBPF_CPU_PROFILER: {:?}", path);
                let bpf = EbpfLoader::new()
                    .allow_unsupported_maps()
                    .load_file(&path)
                    .context("Failed to load eBPF program from file")?;
                info!("Successfully loaded CPU profiler eBPF program from file");
                return Ok(bpf);
            }
        }
        for path in &file_paths {
            if path.exists() {
                info!("Loading eBPF from file: {:?}", path);
                let bpf = EbpfLoader::new()
                    .allow_unsupported_maps()
                    .load_file(path)
                    .context("Failed to load eBPF program from file")?;
                info!("Successfully loaded CPU profiler eBPF program from file");
                return Ok(bpf);
            }
        }
        // Fallback: embedded bytecode (aligned for ELF parsing)
        let bpf_data = aya::include_bytes_aligned!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/bpfel-unknown-none/release/cpu-profiler"
        ));
        let bpf = EbpfLoader::new()
            .allow_unsupported_maps()
            .load(bpf_data)
            .context("Failed to load eBPF program")?;
        info!("Successfully loaded CPU profiler eBPF program (embedded)");
        Ok(bpf)
    }
}

/// Attach CPU profiler as perf_event
///
/// Uses software CPU clock sampling at the given frequency.
/// When `target_pid` is Some, attaches to that single process (kernel handles
/// PID namespace translation). When None, attaches to all processes on each CPU.
pub fn attach_cpu_profiler(
    bpf: &mut Ebpf,
    sample_rate_hz: u64,
    target_pid: Option<i32>,
) -> Result<PerfEventLinks> {
    use tracing::debug;

    info!("Attaching CPU profiler as perf_event at {} Hz", sample_rate_hz);

    debug!("Available programs:");
    for (name, program) in bpf.programs() {
        debug!("  - {} (type: {:?})", name, program.prog_type());
    }

    let program: &mut PerfEvent = bpf
        .program_mut("cpu_profiler")
        .context("Failed to find cpu_profiler program")?
        .try_into()
        .context("Program is not a PerfEvent")?;

    info!("Loading program into kernel");
    program.load().context("Failed to load perf_event program")?;
    info!("Program loaded successfully");

    let mut links = PerfEventLinks::new();

    match target_pid {
        Some(pid) => {
            // Attach to a specific process on any CPU.
            // perf_event_open accepts namespace-relative PIDs, so this works
            // correctly in PID namespaces (e.g., OrbStack VMs).
            info!("Attaching to PID {} on any CPU", pid);
            let link = program
                .attach(
                    PerfTypeId::Software,
                    0, // PERF_COUNT_SW_CPU_CLOCK
                    PerfEventScope::OneProcessAnyCpu { pid: pid as u32 },
                    SamplePolicy::Frequency(sample_rate_hz),
                    false,
                )
                .context(format!("Failed to attach perf_event for PID {}", pid))?;

            links.add(link);
            info!("Successfully attached to PID {}", pid);
        }
        None => {
            // Attach to all processes, one perf event per CPU.
            let cpus = online_cpus().map_err(|(msg, e)| anyhow::anyhow!("{}: {}", msg, e))?;
            info!("Attaching to all processes on {} CPUs", cpus.len());

            for cpu in &cpus {
                let link = program
                    .attach(
                        PerfTypeId::Software,
                        0, // PERF_COUNT_SW_CPU_CLOCK
                        PerfEventScope::AllProcessesOneCpu { cpu: *cpu },
                        SamplePolicy::Frequency(sample_rate_hz),
                        false,
                    )
                    .context(format!("Failed to attach perf_event on CPU {}", cpu))?;

                links.add(link);
            }

            info!("Successfully attached to {} CPUs", cpus.len());
        }
    }

    Ok(links)
}

/// Detach and cleanup eBPF programs
pub fn cleanup(links: PerfEventLinks) {
    info!("Cleaning up {} perf event links", links.links.len());
    // Links are automatically cleaned up when dropped
    drop(links);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires eBPF build artifacts
    fn test_load_cpu_profiler() {
        // This test requires the eBPF program to be built
        let result = load_cpu_profiler();
        assert!(result.is_err()); // Expected to fail until implemented
    }
}

use aya::programs::trace_point::TracePointLinkId;
use aya::programs::raw_trace_point::RawTracePointLinkId;
use aya::programs::{TracePoint, RawTracePoint};

/// Storage for tracepoint links
pub struct TracepointLinks {
    links: Vec<TracePointLinkId>,
}

impl TracepointLinks {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add(&mut self, link: TracePointLinkId) {
        self.links.push(link);
    }
}

/// Storage for raw tracepoint links
pub struct RawTracepointLinks {
    links: Vec<RawTracePointLinkId>,
}

impl RawTracepointLinks {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add(&mut self, link: RawTracePointLinkId) {
        self.links.push(link);
    }
}

/// Load the lock profiler eBPF program
pub fn load_lock_profiler() -> Result<Ebpf> {
    use aya::EbpfLoader;
    info!("Loading lock profiler eBPF program");

    #[cfg(debug_assertions)]
    {
        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../target/bpfel-unknown-none/debug/lock-profiler");
        if path.exists() {
            return EbpfLoader::new().load_file(&path).context("Failed to load lock profiler");
        }
    }

    #[cfg(not(debug_assertions))]
    {
        let bpf_data = aya::include_bytes_aligned!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/bpfel-unknown-none/release/lock-profiler"
        ));
        EbpfLoader::new().allow_unsupported_maps().load(bpf_data).context("Failed to load lock profiler")
    }
    
    // Fallback for debug if file not found
    #[cfg(debug_assertions)]
    anyhow::bail!("Lock profiler binary not found")
}

/// Attach lock profiler
pub fn attach_lock_profiler(
    bpf: &mut Ebpf,
    target_pid: Option<i32>,
) -> Result<TracepointLinks> {
    let mut links = TracepointLinks::new();

    // Attach sys_enter_futex
    let program: &mut TracePoint = bpf
        .program_mut("sys_enter_futex")
        .context("sys_enter_futex not found")?
        .try_into()
        .context("Not a TracePoint")?;
    program.load()?;
    links.add(program.attach("syscalls", "sys_enter_futex")?);

    // Attach sys_exit_futex
    let program: &mut TracePoint = bpf
        .program_mut("sys_exit_futex")
        .context("sys_exit_futex not found")?
        .try_into()
        .context("Not a TracePoint")?;
    program.load()?;
    links.add(program.attach("syscalls", "sys_exit_futex")?);

    // Write PID filter AFTER programs are loaded (so map relocations work)
    let pid_value: u64 = target_pid.unwrap_or(0) as u64;
    let mut filter_map: aya::maps::Array<_, u64> = aya::maps::Array::try_from(
        bpf.map_mut("PID_FILTER").context("Failed to get PID_FILTER map")?
    )?;
    filter_map.set(0, pid_value, 0)?;
    if pid_value != 0 {
        let (dev, ino) = get_pidns_dev_ino()?;
        filter_map.set(1, dev, 0)?;
        filter_map.set(2, ino, 0)?;
        info!("Lock profiler PID filter: pid={}, ns_dev={}, ns_ino={}", pid_value, dev, ino);
    } else {
        info!("Lock profiler PID filter: disabled (tracing all)");
    }

    Ok(links)
}

/// Load the syscall tracer eBPF program
pub fn load_syscall_tracer() -> Result<Ebpf> {
    use aya::EbpfLoader;
    info!("Loading syscall tracer eBPF program");

    #[cfg(debug_assertions)]
    {
        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../target/bpfel-unknown-none/debug/syscall-tracer");
        if path.exists() {
            return EbpfLoader::new().load_file(&path).context("Failed to load syscall tracer");
        }
    }

    #[cfg(not(debug_assertions))]
    {
        let bpf_data = aya::include_bytes_aligned!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/bpfel-unknown-none/release/syscall-tracer"
        ));
        EbpfLoader::new().allow_unsupported_maps().load(bpf_data).context("Failed to load syscall tracer")
    }

    #[cfg(debug_assertions)]
    anyhow::bail!("Syscall tracer binary not found")
}

/// Attach syscall tracer
pub fn attach_syscall_tracer(
    bpf: &mut Ebpf,
    target_pid: Option<i32>,
) -> Result<RawTracepointLinks> {
    let mut links = RawTracepointLinks::new();

    // Attach sys_enter
    let program: &mut RawTracePoint = bpf
        .program_mut("sys_enter")
        .context("sys_enter not found")?
        .try_into()
        .context("Not a RawTracePoint")?;
    program.load()?;
    links.add(program.attach("sys_enter")?);

    // Attach sys_exit
    let program: &mut RawTracePoint = bpf
        .program_mut("sys_exit")
        .context("sys_exit not found")?
        .try_into()
        .context("Not a RawTracePoint")?;
    program.load()?;
    links.add(program.attach("sys_exit")?);

    // Write PID filter AFTER programs are loaded (so map relocations work)
    let pid_value: u64 = target_pid.unwrap_or(0) as u64;
    let mut filter_map: aya::maps::Array<_, u64> = aya::maps::Array::try_from(
        bpf.map_mut("PID_FILTER").context("Failed to get PID_FILTER map")?
    )?;
    filter_map.set(0, pid_value, 0)?;
    if pid_value != 0 {
        let (dev, ino) = get_pidns_dev_ino()?;
        filter_map.set(1, dev, 0)?;
        filter_map.set(2, ino, 0)?;
        info!("Syscall tracer PID filter: pid={}, ns_dev={}, ns_ino={}", pid_value, dev, ino);
    } else {
        info!("Syscall tracer PID filter: disabled (tracing all)");
    }

    Ok(links)
}
