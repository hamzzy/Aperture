//! CUPTI event collector

use crate::{GpuMetric, GpuProfiler};
use anyhow::Result;

/// CUPTI-based GPU profiler
pub struct CuptiProfiler {
    // TODO: Store CUPTI subscriber handle, activity buffers, etc.
}

impl CuptiProfiler {
    /// Create a new CUPTI profiler
    pub fn new() -> Result<Self> {
        // TODO: Initialize CUPTI
        // 1. Register callbacks
        // 2. Enable activity kinds (kernel, memcpy, etc.)
        // 3. Allocate activity buffers

        Ok(Self {})
    }
}

impl GpuProfiler for CuptiProfiler {
    fn start(&mut self) -> Result<()> {
        // TODO: Start CUPTI profiling
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // TODO: Stop CUPTI profiling and flush buffers
        Ok(())
    }

    fn collect_metrics(&self) -> Result<Vec<GpuMetric>> {
        // TODO: Parse CUPTI activity records into GpuMetric
        Ok(Vec::new())
    }
}

impl Default for CuptiProfiler {
    fn default() -> Self {
        Self::new().expect("Failed to create CUPTI profiler")
    }
}
