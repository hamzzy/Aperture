//! eBPF program loader
//!
//! Loads and attaches eBPF programs to the kernel

use anyhow::{Context, Result};
use aya::{
    programs::{
        KProbe,
        kprobe::KProbeLinkId,
    },
    Bpf,
};
use tracing::info;

/// Storage for kprobe links to keep them alive
pub struct PerfEventLinks {
    links: Vec<KProbeLinkId>,
}

impl PerfEventLinks {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add(&mut self, link: KProbeLinkId) {
        self.links.push(link);
    }
}

/// Load the CPU profiler eBPF program
pub fn load_cpu_profiler() -> Result<Bpf> {
    use aya::BpfLoader;

    info!("Loading CPU profiler eBPF program");

    // For debugging, try loading from file first
    #[cfg(debug_assertions)]
    {
        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../target/bpfel-unknown-none/debug/cpu-profiler");

        info!("Loading eBPF from file: {:?}", path);

        if path.exists() {
            let bpf = BpfLoader::new()
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
        let bpf = BpfLoader::new()
            .allow_unsupported_maps()
            .load(bpf_data)
            .context("Failed to load eBPF program")?;
        info!("Successfully loaded CPU profiler eBPF program");
        Ok(bpf)
    }
}

/// Attach CPU profiler as kprobe
///
/// Attaches the eBPF program as a kprobe on finish_task_switch
pub fn attach_cpu_profiler(bpf: &mut Bpf, _sample_rate_hz: u64) -> Result<PerfEventLinks> {
    use tracing::debug;

    info!("Attaching CPU profiler as kprobe on do_sys_openat2");

    // Debug: List all programs in the BPF object
    debug!("Available programs:");
    for (name, program) in bpf.programs() {
        debug!("  - {} (type: {:?})", name, program.prog_type());
    }

    let program: &mut KProbe = bpf
        .program_mut("cpu_profiler")
        .context("Failed to find cpu_profiler program")?
        .try_into()
        .context("Program is not a KProbe")?;

    // Load the program into the kernel
    info!("Loading program into kernel");
    program.load().context("Failed to load kprobe program")?;
    info!("Program loaded successfully");

    let mut links = PerfEventLinks::new();

    // Attach to do_sys_openat2 kernel function for testing
    // vfs_read doesn't fire on OrbStack kernels, but do_sys_openat2 does
    info!("Attaching to do_sys_openat2");
    let link = program
        .attach("do_sys_openat2", 0)
        .context("Failed to attach to do_sys_openat2")?;

    links.add(link);
    info!("Successfully attached kprobe");

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
