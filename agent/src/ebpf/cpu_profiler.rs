//! CPU profiler eBPF program management
//!
//! Handles the lifecycle of the CPU profiling eBPF program

use anyhow::{Context, Result};
use aya::Ebpf;
use tracing::{info, warn};

use super::loader::{self, PerfEventLinks};

/// CPU profiler manager
pub struct CpuProfiler {
    bpf: Ebpf,
    links: Option<PerfEventLinks>,
    sample_rate_hz: u64,
    target_pid: Option<i32>,
}

impl CpuProfiler {
    /// Create a new CPU profiler
    pub fn new(sample_rate_hz: u64) -> Result<Self> {
        info!("Initializing CPU profiler at {} Hz", sample_rate_hz);

        // Load eBPF program
        let bpf = loader::load_cpu_profiler().context("Failed to load CPU profiler eBPF")?;

        Ok(Self {
            bpf,
            links: None,
            sample_rate_hz,
            target_pid: None,
        })
    }

    /// Set target PID filter (None = profile all processes).
    /// PID filtering is done at the kernel level via perf_event_open scope,
    /// which correctly handles PID namespaces.
    pub fn set_target_pid(&mut self, pid: Option<i32>) {
        if let Some(p) = pid {
            info!("Will filter for PID {}", p);
        }
        self.target_pid = pid;
    }

    /// Start profiling
    pub fn start(&mut self) -> Result<()> {
        info!("Starting CPU profiling");

        if self.links.is_some() {
            warn!("Profiler already started");
            return Ok(());
        }

        // Attach eBPF program to perf events
        let links =
            loader::attach_cpu_profiler(&mut self.bpf, self.sample_rate_hz, self.target_pid)
                .context("Failed to attach CPU profiler")?;

        self.links = Some(links);
        info!("CPU profiling started successfully");

        Ok(())
    }

    /// Stop profiling
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping CPU profiling");

        if let Some(links) = self.links.take() {
            loader::cleanup(links);
            info!("CPU profiling stopped");
        } else {
            warn!("Profiler was not running");
        }

        Ok(())
    }

    /// Get mutable reference to the BPF object for map access
    pub fn bpf_mut(&mut self) -> &mut Ebpf {
        &mut self.bpf
    }

}

impl Drop for CpuProfiler {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::error!("Error stopping profiler: {}", e);
        }
    }
}
