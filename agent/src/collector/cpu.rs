//! CPU event collector
//!
//! Collects CPU profiling samples from eBPF and builds profile data

use anyhow::{Context, Result};
use aperture_shared::types::events::CpuSample;
use aperture_shared::types::profile::{Profile, Stack};
use aya::maps::StackTraceMap;
use bytes::BytesMut;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Raw sample event from eBPF (must match agent-ebpf/src/cpu_profiler.rs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SampleEvent {
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub user_stack_id: i32,
    pub kernel_stack_id: i32,
    pub comm: [u8; 16],
}

// Implement traits for reading from perf buffer
unsafe impl aya::Pod for SampleEvent {}

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
            start_time: aperture_shared::utils::time::system_time_nanos(),
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

    /// Process a raw eBPF event and convert to CpuSample
    pub fn process_event(
        &mut self,
        event: &SampleEvent,
        stacks: &StackTraceMap<aya::maps::MapData>,
    ) -> Result<()> {
        // Convert comm bytes to string
        let comm = std::str::from_utf8(&event.comm)
            .unwrap_or("<unknown>")
            .trim_end_matches('\0')
            .to_string();

        // Get user-space stack trace
        let user_stack = if event.user_stack_id >= 0 {
            match stacks.get(&(event.user_stack_id as u32), 0) {
                Ok(trace) => {
                    let frames: Vec<u64> = trace.frames().iter().map(|f| f.ip).collect();
                    frames
                }
                Err(e) => {
                    debug!("Failed to get user stack {}: {}", event.user_stack_id, e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // Get kernel-space stack trace
        let kernel_stack = if event.kernel_stack_id >= 0 {
            match stacks.get(&(event.kernel_stack_id as u32), 0) {
                Ok(trace) => {
                    let frames: Vec<u64> = trace.frames().iter().map(|f| f.ip).collect();
                    frames
                }
                Err(e) => {
                    debug!("Failed to get kernel stack {}: {}", event.kernel_stack_id, e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let sample = CpuSample {
            timestamp: event.timestamp,
            pid: event.pid as i32,
            tid: event.tid as i32,
            cpu_id: event.cpu,
            user_stack,
            kernel_stack,
            comm,
        };

        self.add_sample(sample);
        Ok(())
    }

    /// Build aggregated profile from collected samples
    pub fn build_profile(&self) -> Result<Profile> {
        info!("Building profile from {} samples", self.samples.len());

        let end_time = aperture_shared::utils::time::system_time_nanos();

        let mut profile = Profile::new(self.start_time, end_time, self.sample_period_ns);

        // Build profile by aggregating stacks
        for sample in &self.samples {
            // Combine kernel and user stacks
            let mut combined_ips = Vec::new();

            // Add user stack first (innermost frames)
            combined_ips.extend_from_slice(&sample.user_stack);

            // Add kernel stack (outer frames)
            combined_ips.extend_from_slice(&sample.kernel_stack);

            // Skip empty stacks
            if combined_ips.is_empty() {
                continue;
            }

            // Create stack and add to profile
            let stack = Stack::from_ips(&combined_ips);
            profile.add_sample(stack);
        }

        info!(
            "Profile built: {} total samples, {} unique stacks",
            profile.total_samples,
            profile.samples.len()
        );

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
