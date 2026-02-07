//! Lock event collector
//!
//! Collects lock contention events from eBPF and builds profile data

use anyhow::Result;
use aperture_shared::types::events::LockEvent;
use aperture_shared::types::profile::{LockProfile, Stack};
use aya::maps::StackTraceMap;
use tracing::{debug, info};

/// Raw lock event from eBPF (must match agent-ebpf/src/lock_profiler.rs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
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

// Implement traits for reading from perf buffer
unsafe impl aya::Pod for LockEventBpf {}

/// Lock event collector
#[derive(Debug)]
pub struct LockCollector {
    /// Collected events
    events: Vec<LockEvent>,

    /// Start time
    start_time: u64,
}

impl LockCollector {
    /// Create a new lock collector
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            start_time: aperture_shared::utils::time::system_time_nanos(),
        }
    }

    /// Add an event to the collector
    pub fn add_event(&mut self, event: LockEvent) {
        self.events.push(event);
    }

    /// Process a raw eBPF event and convert to LockEvent
    pub fn process_event(
        &mut self,
        event: &LockEventBpf,
        stacks: &StackTraceMap<aya::maps::MapData>,
    ) -> Result<()> {
        // Convert comm bytes to string
        let comm = std::str::from_utf8(&event.comm)
            .unwrap_or("<unknown>")
            .trim_end_matches('\0')
            .to_string();

        // Get user-space stack trace
        let mut frames = Vec::new();
        
        if event.user_stack_id >= 0 {
            match stacks.get(&(event.user_stack_id as u32), 0) {
                Ok(trace) => {
                    frames.extend(trace.frames().iter().map(|f| f.ip));
                }
                Err(e) => {
                    debug!("Failed to get user stack {}: {}", event.user_stack_id, e);
                }
            }
        }

        // Get kernel-space stack trace
        if event.kernel_stack_id >= 0 {
            match stacks.get(&(event.kernel_stack_id as u32), 0) {
                Ok(trace) => {
                    frames.extend(trace.frames().iter().map(|f| f.ip));
                }
                Err(e) => {
                    debug!("Failed to get kernel stack {}: {}", event.kernel_stack_id, e);
                }
            }
        }

        let lock_event = LockEvent {
            timestamp: aperture_shared::utils::time::boot_time_to_system_time(event.timestamp),
            pid: event.pid as i32,
            tid: event.tid as i32,
            lock_addr: event.lock_addr,
            hold_time_ns: 0, // Not captured yet
            wait_time_ns: event.wait_time_ns,
            stack_trace: frames,
            comm,
        };

        self.add_event(lock_event);
        Ok(())
    }

    /// Build aggregated profile from collected events
    pub fn build_profile(&self) -> Result<LockProfile> {
        info!("Building lock profile from {} events", self.events.len());

        let mut profile = LockProfile::new(self.start_time);
        
        // Update end time
        profile.end_time = aperture_shared::utils::time::system_time_nanos();

        for event in &self.events {
            if event.stack_trace.is_empty() {
                continue;
            }
            
            let stack = Stack::from_ips(&event.stack_trace);
            profile.add_contention(event.lock_addr, stack, event.wait_time_ns);
        }

        info!(
            "Lock profile built: {} total events, {} unique contentions",
            profile.total_events,
            profile.contentions.len()
        );

        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_shared::types::profile::{Frame, Stack};

    #[test]
    fn test_lock_collector() {
        let mut collector = LockCollector::new();

        let event1 = LockEvent {
            timestamp: 1000,
            pid: 123,
            tid: 123,
            lock_addr: 0x1000,
            hold_time_ns: 0,
            wait_time_ns: 500,
            stack_trace: vec![0x400000, 0x400100],
            comm: "test".to_string(),
        };

        let event2 = LockEvent {
            timestamp: 2000,
            pid: 123,
            tid: 123,
            lock_addr: 0x1000,
            hold_time_ns: 0,
            wait_time_ns: 300,
            stack_trace: vec![0x400000, 0x400100], // Same stack
            comm: "test".to_string(),
        };

        let event3 = LockEvent {
            timestamp: 3000,
            pid: 124,
            tid: 124,
            lock_addr: 0x2000,
            hold_time_ns: 0,
            wait_time_ns: 1000,
            stack_trace: vec![0x500000],
            comm: "other".to_string(),
        };

        collector.add_event(event1);
        collector.add_event(event2);
        collector.add_event(event3);

        let profile = collector.build_profile().unwrap();

        assert_eq!(profile.total_events, 3);
        assert_eq!(profile.contentions.len(), 2);

        // Check contention for 0x1000
        let stack1 = Stack::from_ips(&[0x400000, 0x400100]);
        let stats1 = profile.contentions.get(&(0x1000, stack1)).unwrap();
        assert_eq!(stats1.count, 2);
        assert_eq!(stats1.total_wait_ns, 800);
        assert_eq!(stats1.max_wait_ns, 500);
        assert_eq!(stats1.min_wait_ns, 300);

        // Check contention for 0x2000
        let stack2 = Stack::from_ips(&[0x500000]);
        let stats2 = profile.contentions.get(&(0x2000, stack2)).unwrap();
        assert_eq!(stats2.count, 1);
        assert_eq!(stats2.total_wait_ns, 1000);
    }
}
