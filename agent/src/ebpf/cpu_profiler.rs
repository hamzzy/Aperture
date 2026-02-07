//! CPU profiler eBPF program management
//!
//! Handles the lifecycle of the CPU profiling eBPF program

use anyhow::{Context, Result};
use aya::{
    maps::Map,
    Bpf,
};
use tracing::{info, warn};

use super::loader::{self, PerfEventLinks};
use crate::collector::cpu::SampleEvent;

/// CPU profiler manager
pub struct CpuProfiler {
    bpf: Bpf,
    links: Option<PerfEventLinks>,
    sample_rate_hz: u64,
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
        })
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
            loader::attach_cpu_profiler(&mut self.bpf, self.sample_rate_hz)
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
    pub fn bpf_mut(&mut self) -> &mut Bpf {
        &mut self.bpf
    }

    /// Check if the profiler is currently running
    pub fn is_running(&self) -> bool {
        self.links.is_some()
    }
}

impl Drop for CpuProfiler {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::error!("Error stopping profiler: {}", e);
        }
    }
}
