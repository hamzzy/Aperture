#![no_std]
#![no_main]

//! CPU profiler eBPF program
//!
//! This program samples stack traces from running processes at a configurable
//! frequency using the perf event subsystem.
//!
//! Implementation strategy:
//! 1. Attach to perf events (CPU cycles or timer-based)
//! 2. On each sample, capture stack trace using bpf_get_stackid()
//! 3. Store stack trace in a BPF map for userspace to read
//! 4. Include process context (PID, TID, CPU, timestamp)

use aya_ebpf::{
    macros::{map, perf_event},
    maps::{HashMap, PerfEventArray, StackTrace as BpfStackTrace},
    programs::PerfEventContext,
};
use aya_log_ebpf::info;

// TODO: Import generated vmlinux types
// use crate::vmlinux::*;

/// Maximum number of stack frames to capture
const MAX_STACK_DEPTH: u32 = 127;

/// Stack trace storage
#[map]
static STACKS: BpfStackTrace = BpfStackTrace::with_max_entries(10000, 0);

/// Per-CPU event buffer for sending samples to userspace
#[map]
static EVENTS: PerfEventArray<SampleEvent> = PerfEventArray::with_max_entries(0, 0);

/// Per-process configuration (placeholder for Phase 2 filtering)
#[map]
static CONFIG: HashMap<u32, ProfileConfig> = HashMap::with_max_entries(1024, 0);

/// Sample event sent to userspace
#[repr(C)]
pub struct SampleEvent {
    /// Timestamp in nanoseconds
    pub timestamp: u64,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// CPU core
    pub cpu: u32,
    /// User stack ID (-1 if failed)
    pub user_stack_id: i32,
    /// Kernel stack ID (-1 if failed)
    pub kernel_stack_id: i32,
    /// Process name
    pub comm: [u8; 16],
}

/// Configuration for profiling (Phase 2+)
#[repr(C)]
pub struct ProfileConfig {
    pub enabled: u32,
    pub sample_period: u64,
}

/// Main entry point for CPU profiling
///
/// This function is called on each perf event trigger (e.g., CPU cycle or timer).
#[perf_event]
pub fn cpu_profiler(ctx: PerfEventContext) -> u32 {
    match try_cpu_profiler(ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_cpu_profiler(ctx: PerfEventContext) -> Result<(), i64> {
    // Get current process context
    let pid = ctx.pid();
    let tid = ctx.tgid();

    // TODO Phase 2: Check if this PID is enabled in CONFIG map
    // For Phase 1, profile everything

    // Capture user-space stack trace
    let user_stack_id = unsafe {
        STACKS.get_stackid(&ctx, aya_ebpf::bindings::BPF_F_USER_STACK.into())
    };

    // Capture kernel-space stack trace
    let kernel_stack_id = unsafe { STACKS.get_stackid(&ctx, 0) };

    // Get process name
    let mut comm: [u8; 16] = [0; 16];
    if let Ok(name) = ctx.command() {
        let len = name.len().min(16);
        comm[..len].copy_from_slice(&name[..len]);
    }

    // Create sample event
    let event = SampleEvent {
        timestamp: unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() },
        pid,
        tid,
        cpu: ctx.cpu(),
        user_stack_id: user_stack_id.unwrap_or(-1) as i32,
        kernel_stack_id: kernel_stack_id.unwrap_or(-1) as i32,
        comm,
    };

    // Send event to userspace
    unsafe {
        EVENTS.output(&ctx, &event, 0);
    }

    info!(
        &ctx,
        "Sample captured: pid={} tid={} cpu={}", pid, tid, event.cpu
    );

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
