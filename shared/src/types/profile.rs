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
