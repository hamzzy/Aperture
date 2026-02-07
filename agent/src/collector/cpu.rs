//! CPU event collector
//!
//! Collects CPU profiling samples from eBPF and builds profile data

use anyhow::Result;
use aperture_shared::types::events::CpuSample;
use aperture_shared::types::profile::{Profile, Stack};
use aya::maps::StackTraceMap;
use tracing::{debug, info};

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
            timestamp: aperture_shared::utils::time::boot_time_to_system_time(event.timestamp),
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

    #[test]
    fn test_build_profile_aggregates_stacks() {
        let mut collector = CpuCollector::new(10_000_000);

        // Add two identical stacks — should aggregate to count=2
        for _ in 0..2 {
            collector.add_sample(CpuSample {
                timestamp: 100,
                pid: 1,
                tid: 1,
                cpu_id: 0,
                user_stack: vec![0x1000, 0x2000],
                kernel_stack: vec![],
                comm: "test".to_string(),
            });
        }

        // Add one different stack
        collector.add_sample(CpuSample {
            timestamp: 200,
            pid: 1,
            tid: 1,
            cpu_id: 0,
            user_stack: vec![0x3000],
            kernel_stack: vec![],
            comm: "test".to_string(),
        });

        let profile = collector.build_profile().unwrap();
        assert_eq!(profile.total_samples, 3);
        assert_eq!(profile.samples.len(), 2); // 2 unique stacks
    }

    #[test]
    fn test_build_profile_skips_empty_stacks() {
        let mut collector = CpuCollector::new(10_000_000);

        // Sample with no stack frames at all
        collector.add_sample(CpuSample {
            timestamp: 100,
            pid: 1,
            tid: 1,
            cpu_id: 0,
            user_stack: vec![],
            kernel_stack: vec![],
            comm: "test".to_string(),
        });

        // Sample with frames
        collector.add_sample(CpuSample {
            timestamp: 200,
            pid: 1,
            tid: 1,
            cpu_id: 0,
            user_stack: vec![0x1000],
            kernel_stack: vec![],
            comm: "test".to_string(),
        });

        let profile = collector.build_profile().unwrap();
        assert_eq!(profile.total_samples, 1); // Only the one with frames
    }

    #[test]
    fn test_build_profile_combines_user_and_kernel_stacks() {
        let mut collector = CpuCollector::new(10_000_000);

        collector.add_sample(CpuSample {
            timestamp: 100,
            pid: 1,
            tid: 1,
            cpu_id: 0,
            user_stack: vec![0x400000],
            kernel_stack: vec![0xffffffff81000000],
            comm: "test".to_string(),
        });

        let profile = collector.build_profile().unwrap();
        assert_eq!(profile.total_samples, 1);
        assert_eq!(profile.samples.len(), 1);

        // The single stack should have 2 frames (user + kernel)
        let (stack, count) = profile.samples.iter().next().unwrap();
        assert_eq!(*count, 1);
        assert_eq!(stack.frames.len(), 2);
    }
}
