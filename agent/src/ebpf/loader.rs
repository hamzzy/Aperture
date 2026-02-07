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

    // For release builds, embed the bytecode
    #[cfg(not(debug_assertions))]
    {
        let bpf_data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/bpfel-unknown-none/release/cpu-profiler"
        ));

        // Load the eBPF program
        let bpf = EbpfLoader::new()
            .allow_unsupported_maps()
            .load(bpf_data)
            .context("Failed to load eBPF program")?;
        info!("Successfully loaded CPU profiler eBPF program");
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
