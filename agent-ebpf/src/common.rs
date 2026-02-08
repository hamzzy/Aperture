//! Common BPF helpers and utilities
//!
//! Shared code for all eBPF programs

#![allow(dead_code)]

/// Maximum process name length
pub const TASK_COMM_LEN: usize = 16;

/// BPF helper flags
pub const BPF_F_USER_STACK: u64 = 1 << 8;
pub const BPF_F_FAST_STACK_CMP: u64 = 1 << 9;
pub const BPF_F_REUSE_STACKID: u64 = 1 << 10;

/// PID filtering is now done via TARGET_PID BPF Array maps in each
/// eBPF program (lock_profiler, syscall_tracer). CPU profiling uses
/// perf_event_open scope for PID filtering instead.

/// Futex operations
pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_LOCK_PI: u32 = 6;
pub const FUTEX_WAIT_BITSET: u32 = 9;
pub const FUTEX_CMD_MASK: u32 = !128; // ~(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME)

/// Map sizes
pub const MAX_TRACKED_TIDS: u32 = 16384;
