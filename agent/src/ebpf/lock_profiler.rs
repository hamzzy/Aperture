//! Lock profiler eBPF program management
//!
//! Handles the lifecycle of the lock profiling eBPF program

use anyhow::{Context, Result};
use aya::Ebpf;
use tracing::{info, warn};

use super::loader::{self, TracepointLinks};

/// Lock profiler manager
pub struct LockProfiler {
    bpf: Ebpf,
    links: Option<TracepointLinks>,
    target_pid: Option<i32>,
}

impl LockProfiler {
    /// Create a new lock profiler
    pub fn new() -> Result<Self> {
        info!("Initializing lock profiler");

        // Load eBPF program
        let bpf = loader::load_lock_profiler().context("Failed to load lock profiler eBPF")?;

        Ok(Self {
            bpf,
            links: None,
            target_pid: None,
        })
    }

    /// Set target PID filter
    pub fn set_target_pid(&mut self, pid: Option<i32>) {
        if let Some(p) = pid {
            info!("Will filter for PID {}", p);
        }
        self.target_pid = pid;
    }

    /// Start profiling
    pub fn start(&mut self) -> Result<()> {
        info!("Starting lock profiling");

        if self.links.is_some() {
            warn!("Lock profiler already started");
            return Ok(());
        }

        // Attach eBPF program to tracepoints
        let links = loader::attach_lock_profiler(&mut self.bpf, self.target_pid)
            .context("Failed to attach lock profiler")?;

        self.links = Some(links);
        info!("Lock profiling started successfully");

        Ok(())
    }

    /// Stop profiling
    pub fn stop(&mut self) {
        info!("Stopping lock profiling");

        if let Some(_links) = self.links.take() {
            // Links are dropped here
            info!("Lock profiling stopped");
        } else {
            warn!("Lock profiler was not running");
        }
    }

    /// Get mutable reference to the BPF object for map access
    pub fn bpf_mut(&mut self) -> &mut Ebpf {
        &mut self.bpf
    }
}

impl Drop for LockProfiler {
    fn drop(&mut self) {
        self.stop();
    }
}
