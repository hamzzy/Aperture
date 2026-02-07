#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::{HashMap, PerfEventArray, StackTrace},
    programs::TracePointContext,
    EbpfContext,
};

mod common;
use common::{FUTEX_WAIT, FUTEX_LOCK_PI, FUTEX_WAIT_BITSET, FUTEX_CMD_MASK};

#[map]
static LOCK_EVENTS: PerfEventArray<LockEventBpf> = PerfEventArray::new(0);

#[map]
static LOCK_STACKS: StackTrace = StackTrace::with_max_entries(1024, 0);

#[map]
static FUTEX_ENTRIES: HashMap<u32, FutexEntry> = HashMap::with_max_entries(1024, 0);

#[repr(C)]
pub struct LockEventBpf {
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub lock_addr: u64,
    pub wait_time_ns: u64,
    pub user_stack_id: i64,
    pub kernel_stack_id: i64,
    pub comm: [u8; 16],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FutexEntry {
    pub timestamp: u64,
    pub uaddr: u64,
}

#[tracepoint(name = "sys_enter_futex", category = "syscalls")]
pub fn sys_enter_futex(ctx: TracePointContext) -> i64 {
    match try_sys_enter_futex(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sys_enter_futex(ctx: &TracePointContext) -> Result<i64, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let tid = pid_tgid as u32;
    
    // Read arguments
    // sys_enter_futex(u32 *uaddr, int op, u32 val, struct timespec *utime, u32 *uaddr2, u32 val3)
    // Offset 16: uaddr, Offset 24: op
    let uaddr: u64 = unsafe { ctx.read_at(16).map_err(|_| 1i64)? };
    let op: u32 = unsafe { ctx.read_at(24).map_err(|_| 1i64)? };
    
    // Check if it's a wait operation
    let cmd = op & FUTEX_CMD_MASK;
    if cmd != FUTEX_WAIT && cmd != FUTEX_LOCK_PI && cmd != FUTEX_WAIT_BITSET {
        return Ok(0);
    }
    
    let timestamp = unsafe { bpf_ktime_get_ns() };
    
    let entry = FutexEntry {
        timestamp,
        uaddr,
    };
    
    FUTEX_ENTRIES.insert(&tid, &entry, 0).map_err(|_| 1i64)?;
    
    Ok(0)
}

#[tracepoint(name = "sys_exit_futex", category = "syscalls")]
pub fn sys_exit_futex(ctx: TracePointContext) -> i64 {
    match try_sys_exit_futex(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sys_exit_futex(ctx: &TracePointContext) -> Result<i64, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let tid = pid_tgid as u32;
    let pid = (pid_tgid >> 32) as u32;
    
    // Check if we are tracking this thread
    let entry = unsafe {
        match FUTEX_ENTRIES.get(&tid) {
            Some(e) => e,
            None => return Ok(0),
        }
    };
    
    // Calculate wait time
    let now = unsafe { bpf_ktime_get_ns() };
    let wait_time_ns = now - entry.timestamp;
    
    // Capture stacks - ensure we use the context properly
    let kernel_stack_id = unsafe { LOCK_STACKS.get_stackid(ctx, 0) }.unwrap_or(-1);
    
    // BPF_F_USER_STACK = 1 << 8
    let user_stack_id = unsafe { 
        LOCK_STACKS.get_stackid(ctx, 256) 
    }.unwrap_or(-1);
    
    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
    
    let event = LockEventBpf {
        timestamp: entry.timestamp,
        pid,
        tid,
        lock_addr: entry.uaddr,
        wait_time_ns,
        user_stack_id,
        kernel_stack_id,
        comm,
    };
    
    LOCK_EVENTS.output(ctx, &event, 0);
    
    // Cleanup
    FUTEX_ENTRIES.remove(&tid).map_err(|_| 1i64)?;
    
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
