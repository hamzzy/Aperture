//! GPU metrics definitions and utilities (Phase 4+)

use crate::GpuMetric;

/// Aggregate GPU metrics
pub struct GpuMetricAggregator {
    metrics: Vec<GpuMetric>,
}

impl GpuMetricAggregator {
    /// Create a new aggregator
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    /// Add a metric
    pub fn add_metric(&mut self, metric: GpuMetric) {
        self.metrics.push(metric);
    }

    /// Get total GPU time
    pub fn total_gpu_time_ns(&self) -> u64 {
        // TODO Phase 4: Calculate total GPU time
        0
    }

    /// Get kernel execution count
    pub fn kernel_count(&self) -> usize {
        // TODO Phase 4: Count kernel executions
        0
    }
}

impl Default for GpuMetricAggregator {
    fn default() -> Self {
        Self::new()
    }
}
