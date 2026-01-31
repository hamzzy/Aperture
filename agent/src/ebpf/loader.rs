//! eBPF program loader
//!
//! Loads and attaches eBPF programs to the kernel

use anyhow::{Context, Result};
use aya::{
    programs::{links::FdLink, perf_event::{PerfEventScope, PerfTypeId, SamplePolicy}, PerfEvent},
    Bpf,
};
use std::sync::Arc;
use tracing::{info, warn};

/// Storage for perf event links to keep them alive
pub struct PerfEventLinks {
    links: Vec<FdLink>,
}

impl PerfEventLinks {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add(&mut self, link: FdLink) {
        self.links.push(link);
    }
}

/// Load the CPU profiler eBPF program
pub fn load_cpu_profiler() -> Result<Bpf> {
    info!("Loading CPU profiler eBPF program");

    // Load the compiled eBPF bytecode
    // This will be embedded from the agent-ebpf build output
    #[cfg(debug_assertions)]
    let bpf_data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/bpfel-unknown-none/debug/cpu-profiler"
    ));

    #[cfg(not(debug_assertions))]
    let bpf_data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/bpfel-unknown-none/release/cpu-profiler"
    ));

    // Load the eBPF program
    let bpf = Bpf::load(bpf_data).context("Failed to load eBPF program")?;

    info!("Successfully loaded CPU profiler eBPF program");
    Ok(bpf)
}

/// Attach CPU profiler to perf events
///
/// Attaches the eBPF program to perf events on all CPUs at the specified sample rate
pub fn attach_cpu_profiler(bpf: &mut Bpf, sample_rate_hz: u64) -> Result<PerfEventLinks> {
    info!(
        "Attaching CPU profiler to perf events at {} Hz",
        sample_rate_hz
    );

    let program: &mut PerfEvent = bpf
        .program_mut("cpu_profiler")
        .context("Failed to find cpu_profiler program")?
        .try_into()
        .context("Program is not a PerfEvent")?;

    // Load the program into the kernel
    program.load().context("Failed to load perf event program")?;

    let mut links = PerfEventLinks::new();

    // Get number of CPUs
    let num_cpus = num_cpus::get();
    info!("Attaching to {} CPU cores", num_cpus);

    // Attach to each CPU
    for cpu_id in 0..num_cpus {
        // Create perf event for CPU profiling
        // Using PERF_TYPE_SOFTWARE with PERF_COUNT_SW_CPU_CLOCK
        let link = program
            .attach(
                PerfTypeId::Software,
                0, // PERF_COUNT_SW_CPU_CLOCK = 0 in perf_event.h
                PerfEventScope::AllProcessesOneCpu { cpu: cpu_id as u32 },
                SamplePolicy::Frequency(sample_rate_hz),
            )
            .with_context(|| format!("Failed to attach to CPU {}", cpu_id))?;

        links.add(link);
    }

    info!("Successfully attached to all {} CPUs", num_cpus);
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
