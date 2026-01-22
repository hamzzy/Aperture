//! CPU event collector
//!
//! Collects CPU profiling samples from eBPF and builds profile data

use anyhow::Result;
use shared::types::events::CpuSample;
use shared::types::profile::Profile;
use std::collections::HashMap;
use tracing::{debug, info};

/// CPU event collector
pub struct CpuCollector {
    /// Collected samples
    samples: Vec<CpuSample>,

    /// Start time
    start_time: u64,

    /// Sample period in nanoseconds
    sample_period_ns: u64,
}

impl CpuCollector {
    /// Create a new CPU collector
    pub fn new(sample_period_ns: u64) -> Self {
        Self {
            samples: Vec::new(),
            start_time: shared::utils::time::system_time_nanos(),
            sample_period_ns,
        }
    }

    /// Add a sample to the collector
    pub fn add_sample(&mut self, sample: CpuSample) {
        debug!(
            "Collected sample: pid={} tid={} cpu={}",
            sample.pid, sample.tid, sample.cpu_id
        );
        self.samples.push(sample);
    }

    /// Build aggregated profile from collected samples
    pub fn build_profile(&self) -> Result<Profile> {
        info!("Building profile from {} samples", self.samples.len());

        let end_time = shared::utils::time::system_time_nanos();

        let mut profile = Profile::new(self.start_time, end_time, self.sample_period_ns);

        // TODO Phase 1: Implement profile building
        // 1. For each sample, create a Stack from the stack trace
        // 2. Add the stack to the profile
        // 3. Handle deduplication (same stack multiple times)

        info!("Profile built: {} total samples", profile.total_samples);

        Ok(profile)
    }

    /// Get the number of collected samples
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Get samples grouped by process
    pub fn samples_by_pid(&self) -> HashMap<i32, Vec<&CpuSample>> {
        let mut by_pid: HashMap<i32, Vec<&CpuSample>> = HashMap::new();

        for sample in &self.samples {
            by_pid.entry(sample.pid).or_default().push(sample);
        }

        by_pid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_creation() {
        let collector = CpuCollector::new(10_000_000);
        assert_eq!(collector.sample_count(), 0);
    }

    #[test]
    fn test_add_sample() {
        let mut collector = CpuCollector::new(10_000_000);

        let sample = CpuSample {
            timestamp: 1234567890,
            pid: 1000,
            tid: 1001,
            cpu_id: 0,
            user_stack: vec![0x400000, 0x400100],
            kernel_stack: vec![],
            comm: "test".to_string(),
        };

        collector.add_sample(sample);
        assert_eq!(collector.sample_count(), 1);
    }
}
