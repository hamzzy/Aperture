//! Event type definitions for profiling data
//!
//! These types represent the raw events collected by eBPF programs and
//! processed by the agent.

use serde::{Deserialize, Serialize};

/// Timestamp in nanoseconds since boot
pub type Timestamp = u64;

/// Process ID
pub type Pid = i32;

/// Thread ID
pub type Tid = i32;

/// CPU core number
pub type CpuId = u32;

/// Stack trace represented as an array of instruction pointers
pub type StackTrace = Vec<u64>;

/// CPU profiling sample event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    /// Timestamp when the sample was taken
    pub timestamp: Timestamp,

    /// Process ID
    pub pid: Pid,

    /// Thread ID
    pub tid: Tid,

    /// CPU core where the sample was taken
    pub cpu_id: CpuId,

    /// User-space stack trace
    pub user_stack: StackTrace,

    /// Kernel-space stack trace
    pub kernel_stack: StackTrace,

    /// Process name (comm)
    pub comm: String,
}

/// Lock contention event (Phase 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEvent {
    pub timestamp: Timestamp,
    pub pid: Pid,
    pub tid: Tid,
    pub lock_addr: u64,
    pub hold_time_ns: u64,
    pub wait_time_ns: u64,
    pub stack_trace: StackTrace,
}

/// Syscall event (Phase 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallEvent {
    pub timestamp: Timestamp,
    pub pid: Pid,
    pub tid: Tid,
    pub syscall_id: u32,
    pub duration_ns: u64,
    pub return_value: i64,
}

/// GPU kernel execution event (Phase 4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuKernelEvent {
    pub timestamp: Timestamp,
    pub pid: Pid,
    pub kernel_name: String,
    pub duration_ns: u64,
    pub grid_size: (u32, u32, u32),
    pub block_size: (u32, u32, u32),
}

/// Unified profiling event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileEvent {
    CpuSample(CpuSample),
    Lock(LockEvent),
    Syscall(SyscallEvent),
    GpuKernel(GpuKernelEvent),
}

impl ProfileEvent {
    /// Get the timestamp of any event type
    pub fn timestamp(&self) -> Timestamp {
        match self {
            ProfileEvent::CpuSample(e) => e.timestamp,
            ProfileEvent::Lock(e) => e.timestamp,
            ProfileEvent::Syscall(e) => e.timestamp,
            ProfileEvent::GpuKernel(e) => e.timestamp,
        }
    }

    /// Get the process ID of any event type
    pub fn pid(&self) -> Pid {
        match self {
            ProfileEvent::CpuSample(e) => e.pid,
            ProfileEvent::Lock(e) => e.pid,
            ProfileEvent::Syscall(e) => e.pid,
            ProfileEvent::GpuKernel(e) => e.pid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_sample_serialization() {
        let sample = CpuSample {
            timestamp: 1234567890,
            pid: 1000,
            tid: 1001,
            cpu_id: 0,
            user_stack: vec![0x400000, 0x400100],
            kernel_stack: vec![],
            comm: "test".to_string(),
        };

        let json = serde_json::to_string(&sample).unwrap();
        let deserialized: CpuSample = serde_json::from_str(&json).unwrap();

        assert_eq!(sample.pid, deserialized.pid);
        assert_eq!(sample.timestamp, deserialized.timestamp);
    }
}
