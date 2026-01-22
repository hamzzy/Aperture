//! GPU Profiling (Phase 4+)
//!
//! Profiles GPU workloads including CUDA kernels, memory transfers, and more

pub mod cupti;
pub mod metrics;

use anyhow::Result;

/// GPU profiler interface
pub trait GpuProfiler {
    /// Start profiling GPU activity
    fn start(&mut self) -> Result<()>;

    /// Stop profiling
    fn stop(&mut self) -> Result<()>;

    /// Collect profiling metrics
    fn collect_metrics(&self) -> Result<Vec<GpuMetric>>;
}

/// GPU metric types
#[derive(Debug, Clone)]
pub enum GpuMetric {
    KernelExecution {
        name: String,
        duration_ns: u64,
        grid_size: (u32, u32, u32),
        block_size: (u32, u32, u32),
    },
    MemoryTransfer {
        kind: MemoryTransferKind,
        bytes: u64,
        duration_ns: u64,
    },
}

#[derive(Debug, Clone)]
pub enum MemoryTransferKind {
    HostToDevice,
    DeviceToHost,
    DeviceToDevice,
}

// TODO Phase 4: Implement GPU profiling
// - CUDA support via CUPTI
// - ROCm support via rocProfiler
// - Correlate GPU events with CPU events
