//! Syscall event collector
//!
//! Collects syscall events from eBPF and builds profile data

use anyhow::Result;
use aperture_shared::types::events::{ProfileEvent, SyscallEvent};
use aperture_shared::types::profile::SyscallProfile;
use aperture_shared::utils::syscalls::syscall_name;
use tracing::info;

/// Raw syscall event from eBPF (must match agent-ebpf/src/syscall_tracer.rs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SyscallEventBpf {
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub syscall_id: u32,
    pub duration_ns: u64,
    pub return_value: i64,
    pub comm: [u8; 16],
}

// Implement traits for reading from perf buffer
unsafe impl aya::Pod for SyscallEventBpf {}

/// Syscall event collector
#[derive(Debug)]
pub struct SyscallCollector {
    /// Collected events
    events: Vec<SyscallEvent>,

    /// Start time
    start_time: u64,

    /// Index of first event not yet pushed to aggregator
    push_cursor: usize,
}

impl Default for SyscallCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallCollector {
    /// Create a new syscall collector
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            start_time: aperture_shared::utils::time::system_time_nanos(),
            push_cursor: 0,
        }
    }

    /// Add an event to the collector
    pub fn add_event(&mut self, event: SyscallEvent) {
        self.events.push(event);
    }

    /// Process a raw eBPF event and convert to SyscallEvent
    pub fn process_event(&mut self, event: &SyscallEventBpf) -> Result<()> {
        // Convert comm bytes to string
        let comm = std::str::from_utf8(&event.comm)
            .unwrap_or("<unknown>")
            .trim_end_matches('\0')
            .to_string();

        let syscall_event = SyscallEvent {
            timestamp: aperture_shared::utils::time::boot_time_to_system_time(event.timestamp),
            pid: event.pid as i32,
            tid: event.tid as i32,
            syscall_id: event.syscall_id,
            duration_ns: event.duration_ns,
            return_value: event.return_value,
            comm,
        };

        self.add_event(syscall_event);
        Ok(())
    }

    /// Build aggregated profile from collected events
    pub fn build_profile(&self) -> Result<SyscallProfile> {
        info!("Building syscall profile from {} events", self.events.len());

        let mut profile = SyscallProfile::new(self.start_time);

        // Update end time
        profile.end_time = aperture_shared::utils::time::system_time_nanos();

        for event in &self.events {
            let name = syscall_name(event.syscall_id);
            profile.add_syscall(
                event.syscall_id,
                name,
                event.duration_ns,
                event.return_value,
            );
        }

        info!(
            "Syscall profile built: {} total events, {} unique syscalls",
            profile.total_events,
            profile.syscalls.len()
        );

        Ok(profile)
    }

    /// All events for a final push to the aggregator.
    pub fn profile_events(&self) -> Vec<ProfileEvent> {
        self.events
            .iter()
            .cloned()
            .map(ProfileEvent::Syscall)
            .collect()
    }

    /// Return events accumulated since the last call and advance the cursor.
    pub fn take_pending_events(&mut self) -> Vec<ProfileEvent> {
        let events: Vec<ProfileEvent> = self.events[self.push_cursor..]
            .iter()
            .cloned()
            .map(ProfileEvent::Syscall)
            .collect();
        self.push_cursor = self.events.len();
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_collector() {
        let mut collector = SyscallCollector::new();

        let event1 = SyscallEvent {
            timestamp: 1000,
            pid: 123,
            tid: 123,
            syscall_id: 0, // read
            duration_ns: 100,
            return_value: 0,
            comm: "test".to_string(),
        };

        let event2 = SyscallEvent {
            timestamp: 2000,
            pid: 123,
            tid: 123,
            syscall_id: 0, // read
            duration_ns: 200,
            return_value: 0,
            comm: "test".to_string(),
        };

        let event3 = SyscallEvent {
            timestamp: 3000,
            pid: 123,
            tid: 123,
            syscall_id: 1, // write
            duration_ns: 150,
            return_value: -1, // error
            comm: "test".to_string(),
        };

        collector.add_event(event1);
        collector.add_event(event2);
        collector.add_event(event3);

        let profile = collector.build_profile().unwrap();

        assert_eq!(profile.total_events, 3);
        assert_eq!(profile.syscalls.len(), 2);

        // Check read (id 0)
        let read_stats = profile.syscalls.get(&0).unwrap();
        assert_eq!(read_stats.count, 2);
        assert_eq!(read_stats.total_duration_ns, 300);
        assert_eq!(read_stats.error_count, 0);

        // Check write (id 1)
        let write_stats = profile.syscalls.get(&1).unwrap();
        assert_eq!(write_stats.count, 1);
        assert_eq!(write_stats.total_duration_ns, 150);
        assert_eq!(write_stats.error_count, 1);
    }
}
