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

/// Compute the offset between CLOCK_MONOTONIC and CLOCK_REALTIME.
///
/// `bpf_ktime_get_ns()` returns CLOCK_MONOTONIC nanoseconds.
/// Adding this offset converts to wall-clock (UNIX epoch) nanoseconds.
#[cfg(target_os = "linux")]
fn boot_time_offset_ns() -> u64 {
    let mut mono = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let mut real = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    // SAFETY: passing valid pointers to clock_gettime
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut mono);
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut real);
    }

    let mono_ns = mono.tv_sec as u64 * 1_000_000_000 + mono.tv_nsec as u64;
    let real_ns = real.tv_sec as u64 * 1_000_000_000 + real.tv_nsec as u64;

    real_ns.saturating_sub(mono_ns)
}

/// Convert boot time (from eBPF `bpf_ktime_get_ns()`) to system time
/// (nanoseconds since UNIX epoch).
#[cfg(target_os = "linux")]
pub fn boot_time_to_system_time(boot_time_ns: u64) -> u64 {
    boot_time_ns + boot_time_offset_ns()
}

/// Fallback for non-Linux: return the value unchanged.
#[cfg(not(target_os = "linux"))]
pub fn boot_time_to_system_time(boot_time_ns: u64) -> u64 {
    boot_time_ns
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
