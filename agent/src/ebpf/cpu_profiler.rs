//! CPU profiler eBPF program management
//!
//! Handles the lifecycle of the CPU profiling eBPF program

use anyhow::{Context, Result};
use aya::{
    maps::{AsyncPerfEventArray, StackTraceMap},
    Bpf,
};
use tracing::{info, warn};

use super::loader::{self, PerfEventLinks};

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

    /// Get the perf event array for reading samples
    pub fn get_events(&mut self) -> Result<AsyncPerfEventArray<'_>> {
        let events: AsyncPerfEventArray<_> = self
            .bpf
            .take_map("EVENTS")
            .context("Failed to get EVENTS map")?
            .try_into()
            .context("Map is not a PerfEventArray")?;

        Ok(events)
    }

    /// Get the stack trace map for reading stack traces
    pub fn get_stacks(&mut self) -> Result<StackTraceMap<'_>> {
        let stacks: StackTraceMap<_> = self
            .bpf
            .map("STACKS")
            .context("Failed to get STACKS map")?
            .try_into()
            .context("Map is not a StackTraceMap")?;

        Ok(stacks)
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
