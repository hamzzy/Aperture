//! WASM filter API — defines host functions available to WASM filter modules.
//!
//! WASM filters receive serialized event data via shared memory and return
//! a boolean verdict (keep = 1, discard = 0).

use aperture_shared::types::events::ProfileEvent;

/// Event context passed to WASM filters via shared memory.
/// The filter module reads fields from this flat struct.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct EventContext {
    /// 0 = CpuSample, 1 = Lock, 2 = Syscall, 3 = GpuKernel
    pub event_type: u32,
    /// Process ID
    pub pid: i32,
    /// Thread ID
    pub tid: i32,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// CPU ID (CpuSample only)
    pub cpu_id: u32,
    /// User stack depth (CpuSample only)
    pub user_stack_depth: u32,
    /// Kernel stack depth (CpuSample only)
    pub kernel_stack_depth: u32,
    /// Lock address (Lock only)
    pub lock_addr: u64,
    /// Wait time in nanoseconds (Lock only)
    pub wait_time_ns: u64,
    /// Syscall ID (Syscall only)
    pub syscall_id: u32,
    /// Syscall duration in nanoseconds (Syscall only)
    pub duration_ns: u64,
    /// Syscall return value (Syscall only)
    pub return_value: i64,
    /// Length of comm string (stored after this struct in memory)
    pub comm_len: u32,
}

impl EventContext {
    /// Size of the struct in bytes (for WASM memory layout)
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Build an EventContext from a ProfileEvent
    pub fn from_event(event: &ProfileEvent) -> (Self, String) {
        match event {
            ProfileEvent::CpuSample(e) => (
                Self {
                    event_type: 0,
                    pid: e.pid,
                    tid: e.tid,
                    timestamp: e.timestamp,
                    cpu_id: e.cpu_id,
                    user_stack_depth: e.user_stack.len() as u32,
                    kernel_stack_depth: e.kernel_stack.len() as u32,
                    comm_len: e.comm.len() as u32,
                    ..Default::default()
                },
                e.comm.clone(),
            ),
            ProfileEvent::Lock(e) => (
                Self {
                    event_type: 1,
                    pid: e.pid,
                    tid: e.tid,
                    timestamp: e.timestamp,
                    lock_addr: e.lock_addr,
                    wait_time_ns: e.wait_time_ns,
                    comm_len: e.comm.len() as u32,
                    ..Default::default()
                },
                e.comm.clone(),
            ),
            ProfileEvent::Syscall(e) => (
                Self {
                    event_type: 2,
                    pid: e.pid,
                    tid: e.tid,
                    timestamp: e.timestamp,
                    syscall_id: e.syscall_id,
                    duration_ns: e.duration_ns,
                    return_value: e.return_value,
                    comm_len: e.comm.len() as u32,
                    ..Default::default()
                },
                e.comm.clone(),
            ),
            ProfileEvent::GpuKernel(e) => (
                Self {
                    event_type: 3,
                    pid: e.pid,
                    tid: 0,
                    timestamp: e.timestamp,
                    duration_ns: e.duration_ns,
                    comm_len: e.kernel_name.len() as u32,
                    ..Default::default()
                },
                e.kernel_name.clone(),
            ),
        }
    }

    /// Serialize to bytes for writing into WASM linear memory
    pub fn to_bytes(&self) -> Vec<u8> {
        let ptr = self as *const Self as *const u8;
        unsafe { std::slice::from_raw_parts(ptr, Self::SIZE).to_vec() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_shared::types::events::CpuSample;

    #[test]
    fn test_event_context_from_cpu_sample() {
        let sample = CpuSample {
            timestamp: 12345,
            pid: 100,
            tid: 101,
            cpu_id: 2,
            user_stack: vec![0x1000, 0x2000, 0x3000],
            kernel_stack: vec![0xffff0000],
            comm: "myapp".to_string(),
            user_stack_symbols: vec![],
            kernel_stack_symbols: vec![],
        };
        let event = ProfileEvent::CpuSample(sample);
        let (ctx, comm) = EventContext::from_event(&event);
        assert_eq!(ctx.event_type, 0);
        assert_eq!(ctx.pid, 100);
        assert_eq!(ctx.user_stack_depth, 3);
        assert_eq!(ctx.kernel_stack_depth, 1);
        assert_eq!(comm, "myapp");
    }

    #[test]
    fn test_event_context_to_bytes_roundtrip() {
        let ctx = EventContext {
            event_type: 2,
            pid: 42,
            syscall_id: 200,
            ..Default::default()
        };
        let bytes = ctx.to_bytes();
        assert_eq!(bytes.len(), EventContext::SIZE);
    }
}
