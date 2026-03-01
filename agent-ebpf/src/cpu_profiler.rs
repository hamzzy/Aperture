#![no_std]
#![no_main]

//! CPU profiler eBPF program
//!
//! Captures stack traces and sends sample events to userspace via perf buffer.

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_smp_processor_id, bpf_ktime_get_ns},
    macros::{map, perf_event},
    maps::{PerfEventArray, StackTrace},
    programs::PerfEventContext,
    EbpfContext,
};

#[no_mangle]
#[link_section = "license"]
pub static LICENSE: [u8; 4] = *b"GPL\0";

const MAX_STACK_DEPTH: u32 = 127;

/// Perf buffer for sending sample events to userspace
#[map]
static EVENTS: PerfEventArray<SampleEvent> = PerfEventArray::new(0);

/// Stack trace storage
#[map]
static STACKS: StackTrace = StackTrace::with_max_entries(MAX_STACK_DEPTH * 256, 0);

/// Sample event sent to userspace
#[repr(C)]
pub struct SampleEvent {
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub user_stack_id: i32,
    pub kernel_stack_id: i32,
    pub comm: [u8; 16],
}

#[perf_event]
pub fn cpu_profiler(ctx: PerfEventContext) -> i64 {
    match try_cpu_profiler(&ctx) {
        Ok(ret) => ret,
        Err(_) => 1,
    }
}

#[inline(always)]
fn try_cpu_profiler(ctx: &PerfEventContext) -> Result<i64, i64> {
    let pid_tgid = ctx.pid();
    let tgid = ctx.tgid();

    // Skip kernel threads (pid 0)
    if tgid == 0 {
        return Ok(0);
    }

    // Get timestamp and CPU id
    let timestamp = unsafe { bpf_ktime_get_ns() };
    let cpu = unsafe { bpf_get_smp_processor_id() };

    // Capture kernel stack trace
    let kernel_stack_id = unsafe { STACKS.get_stackid(ctx, 0) }.unwrap_or(-1);

    // Capture user stack trace
    let user_stack_id =
        unsafe { STACKS.get_stackid(ctx, aya_ebpf::bindings::BPF_F_USER_STACK as u64) }
            .unwrap_or(-1);

    // Get process name
    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    let event = SampleEvent {
        timestamp,
        pid: tgid,
        tid: pid_tgid,
        cpu,
        user_stack_id: user_stack_id as i32,
        kernel_stack_id: kernel_stack_id as i32,
        comm,
    };

    EVENTS.output(ctx, &event, 0);

    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
