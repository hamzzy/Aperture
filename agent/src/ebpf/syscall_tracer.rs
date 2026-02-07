//! Syscall tracer eBPF program management
//!
//! Handles the lifecycle of the syscall tracing eBPF program

use anyhow::{Context, Result};
use aya::Ebpf;
use tracing::{info, warn};

use super::loader::{self, RawTracepointLinks};

/// Syscall tracer manager
pub struct SyscallTracer {
    bpf: Ebpf,
    links: Option<RawTracepointLinks>,
    target_pid: Option<i32>,
}

impl SyscallTracer {
    /// Create a new syscall tracer
    pub fn new() -> Result<Self> {
        info!("Initializing syscall tracer");

        // Load eBPF program
        let bpf = loader::load_syscall_tracer().context("Failed to load syscall tracer eBPF")?;

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

    /// Start tracing
    pub fn start(&mut self) -> Result<()> {
        info!("Starting syscall tracing");

        if self.links.is_some() {
            warn!("Syscall tracer already started");
            return Ok(());
        }

        // Attach eBPF program to raw tracepoints
        let links =
            loader::attach_syscall_tracer(&mut self.bpf, self.target_pid)
                .context("Failed to attach syscall tracer")?;

        self.links = Some(links);
        info!("Syscall tracing started successfully");

        Ok(())
    }

    /// Stop tracing
    pub fn stop(&mut self) {
        info!("Stopping syscall tracing");

        if let Some(_links) = self.links.take() {
            // Links are dropped here
            info!("Syscall tracing stopped");
        } else {
            warn!("Syscall tracer was not running");
        }
    }

    /// Get mutable reference to the BPF object for map access
    pub fn bpf_mut(&mut self) -> &mut Ebpf {
        &mut self.bpf
    }
}

impl Drop for SyscallTracer {
    fn drop(&mut self) {
        self.stop();
    }
}
