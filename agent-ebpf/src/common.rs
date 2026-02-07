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

/// Check if a PID should be profiled
///
/// TODO Phase 2: Implement filtering logic
#[inline(always)]
pub fn should_profile_pid(_pid: u32) -> bool {
    true // Profile everything in Phase 1
}

/// Check if we're in kernel context
#[inline(always)]
pub fn in_kernel_context() -> bool {
    // TODO: Implement kernel context detection
    false
}

/// Futex operations
pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_LOCK_PI: u32 = 6;
pub const FUTEX_WAIT_BITSET: u32 = 9;
pub const FUTEX_CMD_MASK: u32 = !128; // ~(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME)

/// Map sizes
pub const MAX_TRACKED_TIDS: u32 = 16384;

