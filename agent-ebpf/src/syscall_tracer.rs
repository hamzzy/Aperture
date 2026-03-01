#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, raw_tracepoint},
    maps::{Array, HashMap, PerfEventArray},
    programs::RawTracePointContext,
    EbpfContext,
};
use aya_ebpf_bindings::helpers::bpf_get_ns_current_pid_tgid;

#[map]
static SYSCALL_EVENTS: PerfEventArray<SyscallEventBpf> = PerfEventArray::new(0);

#[map]
static SYSCALL_ENTRIES: HashMap<u32, SyscallEntry> = HashMap::with_max_entries(1024, 0);

/// PID_FILTER[0] = target_pid (0 = trace all)
/// PID_FILTER[1] = pidns device number
/// PID_FILTER[2] = pidns inode number
#[map]
static PID_FILTER: Array<u64> = Array::with_max_entries(3, 0);

#[repr(C)]
pub struct SyscallEventBpf {
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub syscall_id: u32,
    pub duration_ns: u64,
    pub return_value: i64,
    pub comm: [u8; 16],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SyscallEntry {
    pub timestamp: u64,
    pub syscall_id: u32,
}

/// Check if the current process matches the PID filter.
/// Returns true if the event should be processed.
#[inline(always)]
fn should_trace() -> bool {
    let target = match PID_FILTER.get(0) {
        Some(&v) => v as u32,
        None => return true, // no filter configured
    };
    if target == 0 {
        return true; // 0 = trace all
    }

    let ns_dev = match PID_FILTER.get(1) {
        Some(&v) => v,
        None => return false,
    };
    let ns_ino = match PID_FILTER.get(2) {
        Some(&v) => v,
        None => return false,
    };

    // Use namespace-aware PID lookup
    let mut nsinfo = aya_ebpf_bindings::bindings::bpf_pidns_info { pid: 0, tgid: 0 };
    let ret = unsafe {
        bpf_get_ns_current_pid_tgid(
            ns_dev,
            ns_ino,
            &mut nsinfo as *mut _,
            core::mem::size_of::<aya_ebpf_bindings::bindings::bpf_pidns_info>() as u32,
        )
    };
    if ret != 0 {
        return false; // helper failed, skip
    }

    nsinfo.tgid == target
}

#[raw_tracepoint(tracepoint = "sys_enter")]
pub fn sys_enter(ctx: RawTracePointContext) -> i32 {
    try_sys_enter(&ctx).unwrap_or_default()
}

fn try_sys_enter(ctx: &RawTracePointContext) -> Result<i32, i64> {
    if !should_trace() {
        return Ok(0);
    }

    let pid_tgid = bpf_get_current_pid_tgid();
    let tid = pid_tgid as u32;

    // ctx.as_ptr() points to struct bpf_raw_tracepoint_args { __u64 args[0]; }
    // args[1] is the syscall ID
    let args = ctx.as_ptr() as *const u64;
    let syscall_id = unsafe { *args.offset(1) } as u32;

    let timestamp = unsafe { bpf_ktime_get_ns() };

    let entry = SyscallEntry {
        timestamp,
        syscall_id,
    };

    SYSCALL_ENTRIES.insert(&tid, &entry, 0).map_err(|_| 1i64)?;

    Ok(0)
}

#[raw_tracepoint(tracepoint = "sys_exit")]
pub fn sys_exit(ctx: RawTracePointContext) -> i32 {
    try_sys_exit(&ctx).unwrap_or_default()
}

fn try_sys_exit(ctx: &RawTracePointContext) -> Result<i32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let tid = pid_tgid as u32;
    let pid = (pid_tgid >> 32) as u32;

    let entry = unsafe {
        match SYSCALL_ENTRIES.get(&tid) {
            Some(e) => e,
            None => return Ok(0),
        }
    };

    // args[1] is the return value
    let args = ctx.as_ptr() as *const u64;
    let return_value = unsafe { *args.offset(1) } as i64;

    let now = unsafe { bpf_ktime_get_ns() };
    let duration_ns = now - entry.timestamp;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    let event = SyscallEventBpf {
        timestamp: entry.timestamp,
        pid,
        tid,
        syscall_id: entry.syscall_id,
        duration_ns,
        return_value,
        comm,
    };

    SYSCALL_EVENTS.output(ctx, &event, 0);

    // Cleanup
    SYSCALL_ENTRIES.remove(&tid).map_err(|_| 1i64)?;

    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
