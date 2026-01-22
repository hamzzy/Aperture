//! Time-related utilities

use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current system time in nanoseconds since UNIX epoch
pub fn system_time_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_nanos() as u64
}

/// Get the current system time in seconds since UNIX epoch
pub fn system_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_secs()
}

/// Convert boot time (from eBPF) to system time
///
/// TODO: Implement proper boot time conversion using /proc/uptime
pub fn boot_time_to_system_time(boot_time_ns: u64) -> u64 {
    boot_time_ns // Placeholder implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_time() {
        let nanos = system_time_nanos();
        let secs = system_time_secs();

        // Basic sanity check
        assert!(nanos > 0);
        assert!(secs > 1_600_000_000); // After 2020
    }
}
