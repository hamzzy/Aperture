//! Profile data structures
//!
//! These types represent aggregated profiling data, suitable for storage
//! and visualization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single frame in a stack trace
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// Instruction pointer
    pub ip: u64,

    /// Function name (if resolved)
    pub function: Option<String>,

    /// File name (if available)
    pub file: Option<String>,

    /// Line number (if available)
    pub line: Option<u32>,

    /// Module/library name
    pub module: Option<String>,
}

impl Frame {
    /// Create a new unresolved frame
    pub fn new_unresolved(ip: u64) -> Self {
        Self {
            ip,
            function: None,
            file: None,
            line: None,
            module: None,
        }
    }

    /// Check if the frame has been symbolized
    pub fn is_symbolized(&self) -> bool {
        self.function.is_some()
    }
}

/// A complete stack trace with symbol information
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Stack {
    /// Frames from innermost (top) to outermost (bottom)
    pub frames: Vec<Frame>,
}

impl Stack {
    /// Create a new stack from instruction pointers
    pub fn from_ips(ips: &[u64]) -> Self {
        Self {
            frames: ips.iter().map(|&ip| Frame::new_unresolved(ip)).collect(),
        }
    }
}

/// Aggregated profile data for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Start timestamp of the profile period
    pub start_time: u64,

    /// End timestamp of the profile period
    pub end_time: u64,

    /// Sample counts per stack trace
    pub samples: HashMap<Stack, u64>,

    /// Total number of samples
    pub total_samples: u64,

    /// Sampling period in nanoseconds
    pub sample_period_ns: u64,
}

impl Profile {
    /// Create a new empty profile
    pub fn new(start_time: u64, end_time: u64, sample_period_ns: u64) -> Self {
        Self {
            start_time,
            end_time,
            samples: HashMap::new(),
            total_samples: 0,
            sample_period_ns,
        }
    }

    /// Add a sample to the profile
    pub fn add_sample(&mut self, stack: Stack) {
        *self.samples.entry(stack).or_insert(0) += 1;
        self.total_samples += 1;
    }

    /// Get the duration of the profile in nanoseconds
    pub fn duration_ns(&self) -> u64 {
        self.end_time.saturating_sub(self.start_time)
    }

    /// Get the sampling rate in Hz
    pub fn sampling_rate_hz(&self) -> f64 {
        if self.sample_period_ns == 0 {
            0.0
        } else {
            1_000_000_000.0 / self.sample_period_ns as f64
        }
    }
}

/// Statistics for lock contention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockContentionStats {
    pub count: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub min_wait_ns: u64,
}

impl Default for LockContentionStats {
    fn default() -> Self {
        Self {
            count: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
            min_wait_ns: u64::MAX,
        }
    }
}

/// Profile of lock contention events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockProfile {
    pub start_time: u64,
    pub end_time: u64,
    // (lock_addr, stack) -> stats
    pub contentions: HashMap<(u64, Stack), LockContentionStats>,
    pub total_events: u64,
}

impl LockProfile {
    pub fn new(start_time: u64) -> Self {
        Self {
            start_time,
            end_time: 0,
            contentions: HashMap::new(),
            total_events: 0,
        }
    }

    pub fn add_contention(&mut self, lock_addr: u64, stack: Stack, wait_ns: u64) {
        let stats = self
            .contentions
            .entry((lock_addr, stack))
            .or_default();
        
        stats.count += 1;
        stats.total_wait_ns += wait_ns;
        stats.max_wait_ns = stats.max_wait_ns.max(wait_ns);
        stats.min_wait_ns = stats.min_wait_ns.min(wait_ns);
        self.total_events += 1;
    }

    pub fn as_weighted_stacks(&self) -> HashMap<Stack, u64> {
        let mut stacks = HashMap::new();
        for ((_, stack), stats) in &self.contentions {
            *stacks.entry(stack.clone()).or_insert(0) += stats.total_wait_ns;
        }
        stacks
    }
}

/// Statistics for system calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallStats {
    pub syscall_id: u32,
    pub name: String,
    pub count: u64,
    pub total_duration_ns: u64,
    pub max_duration_ns: u64,
    pub min_duration_ns: u64,
    pub error_count: u64,
    // Power-of-2 buckets from 1ns to ~1s (30 buckets)
    pub latency_histogram: Vec<u64>,
}

impl SyscallStats {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            syscall_id: id,
            name,
            count: 0,
            total_duration_ns: 0,
            max_duration_ns: 0,
            min_duration_ns: u64::MAX,
            error_count: 0,
            latency_histogram: vec![0; 30],
        }
    }
}

/// Profile of system calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallProfile {
    pub start_time: u64,
    pub end_time: u64,
    pub syscalls: HashMap<u32, SyscallStats>,
    pub total_events: u64,
}

impl SyscallProfile {
    pub fn new(start_time: u64) -> Self {
        Self {
            start_time,
            end_time: 0,
            syscalls: HashMap::new(),
            total_events: 0,
        }
    }

    pub fn add_syscall(&mut self, id: u32, name: &str, duration_ns: u64, return_value: i64) {
        let stats = self
            .syscalls
            .entry(id)
            .or_insert_with(|| SyscallStats::new(id, name.to_string()));

        stats.count += 1;
        stats.total_duration_ns += duration_ns;
        stats.max_duration_ns = stats.max_duration_ns.max(duration_ns);
        stats.min_duration_ns = stats.min_duration_ns.min(duration_ns);

        if return_value < 0 {
            stats.error_count += 1;
        }

        // Calculate bucket: log2(duration_ns)
        // 0..1ns -> 0
        // 2..3ns -> 1
        // ...
        let bucket = if duration_ns == 0 {
            0
        } else {
            (63 - duration_ns.leading_zeros()).min(29) as usize
        };
        stats.latency_histogram[bucket] += 1;
        
        self.total_events += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_add_sample() {
        let mut profile = Profile::new(0, 1000, 10_000_000);

        let stack = Stack::from_ips(&[0x400000, 0x400100]);
        profile.add_sample(stack.clone());
        profile.add_sample(stack.clone());

        assert_eq!(profile.total_samples, 2);
        assert_eq!(*profile.samples.get(&stack).unwrap(), 2);
    }

    #[test]
    fn test_sampling_rate_calculation() {
        let profile = Profile::new(0, 1000, 10_000_000); // 10ms period
        assert!((profile.sampling_rate_hz() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_sampling_rate_zero_period() {
        let profile = Profile::new(0, 1000, 0);
        assert_eq!(profile.sampling_rate_hz(), 0.0);
    }

    #[test]
    fn test_duration_ns() {
        let profile = Profile::new(1000, 5000, 0);
        assert_eq!(profile.duration_ns(), 4000);
    }

    #[test]
    fn test_frame_new_unresolved() {
        let frame = Frame::new_unresolved(0xdeadbeef);
        assert_eq!(frame.ip, 0xdeadbeef);
        assert!(!frame.is_symbolized());
    }

    #[test]
    fn test_frame_is_symbolized() {
        let mut frame = Frame::new_unresolved(0x1000);
        assert!(!frame.is_symbolized());
        frame.function = Some("main".to_string());
        assert!(frame.is_symbolized());
    }

    #[test]
    fn test_stack_from_ips() {
        let stack = Stack::from_ips(&[0x1000, 0x2000, 0x3000]);
        assert_eq!(stack.frames.len(), 3);
        assert_eq!(stack.frames[0].ip, 0x1000);
        assert_eq!(stack.frames[2].ip, 0x3000);
        assert!(stack.frames.iter().all(|f| !f.is_symbolized()));
    }

    #[test]
    fn test_profile_multiple_unique_stacks() {
        let mut profile = Profile::new(0, 1000, 10_000_000);

        profile.add_sample(Stack::from_ips(&[0x1000]));
        profile.add_sample(Stack::from_ips(&[0x2000]));
        profile.add_sample(Stack::from_ips(&[0x1000])); // duplicate

        assert_eq!(profile.total_samples, 3);
        assert_eq!(profile.samples.len(), 2);
        assert_eq!(*profile.samples.get(&Stack::from_ips(&[0x1000])).unwrap(), 2);
        assert_eq!(*profile.samples.get(&Stack::from_ips(&[0x2000])).unwrap(), 1);
    }
}
