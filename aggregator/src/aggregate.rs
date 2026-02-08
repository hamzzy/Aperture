//! Aggregation logic for profile batches
//!
//! Deserializes stored payloads and merges events into aggregated profile types.

use anyhow::{Context, Result};
use aperture_shared::protocol::wire::Message;
use aperture_shared::types::events::ProfileEvent;
use aperture_shared::types::profile::{LockProfile, Profile, Stack, SyscallProfile};
use aperture_shared::utils::syscalls::syscall_name;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};

/// Result of aggregating multiple batches of profile events.
///
/// Note: The inner profile types use non-string HashMap keys (Stack, (u64, Stack))
/// which cannot be serialized directly to JSON. Use `to_json_value()` for JSON output.
pub struct AggregateResult {
    pub cpu: Option<Profile>,
    pub lock: Option<LockProfile>,
    pub syscall: Option<SyscallProfile>,
    pub total_events: u64,
}

/// JSON-safe representation of an aggregated CPU profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfileJson {
    pub start_time: u64,
    pub end_time: u64,
    pub total_samples: u64,
    pub sample_period_ns: u64,
    pub stacks: Vec<StackCountJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackCountJson {
    pub stack: Stack,
    pub count: u64,
}

/// JSON-safe representation of lock contention profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockProfileJson {
    pub start_time: u64,
    pub end_time: u64,
    pub total_events: u64,
    pub contentions: Vec<LockContentionJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockContentionJson {
    pub lock_addr: u64,
    pub stack: Stack,
    pub count: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub min_wait_ns: u64,
}

/// JSON-safe aggregate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResultJson {
    pub cpu: Option<CpuProfileJson>,
    pub lock: Option<LockProfileJson>,
    pub syscall: Option<SyscallProfile>,
    pub total_events: u64,
}

impl AggregateResult {
    /// Convert to a JSON-serializable representation.
    /// Profile types with non-string HashMap keys are flattened to arrays.
    pub fn to_json(&self) -> AggregateResultJson {
        let cpu = self.cpu.as_ref().map(|p| {
            let mut stacks: Vec<StackCountJson> = p
                .samples
                .iter()
                .map(|(stack, &count)| StackCountJson {
                    stack: stack.clone(),
                    count,
                })
                .collect();
            stacks.sort_by(|a, b| b.count.cmp(&a.count));
            CpuProfileJson {
                start_time: p.start_time,
                end_time: p.end_time,
                total_samples: p.total_samples,
                sample_period_ns: p.sample_period_ns,
                stacks,
            }
        });

        let lock = self.lock.as_ref().map(|p| {
            let mut contentions: Vec<LockContentionJson> = p
                .contentions
                .iter()
                .map(|((addr, stack), stats)| LockContentionJson {
                    lock_addr: *addr,
                    stack: stack.clone(),
                    count: stats.count,
                    total_wait_ns: stats.total_wait_ns,
                    max_wait_ns: stats.max_wait_ns,
                    min_wait_ns: stats.min_wait_ns,
                })
                .collect();
            contentions.sort_by(|a, b| b.total_wait_ns.cmp(&a.total_wait_ns));
            LockProfileJson {
                start_time: p.start_time,
                end_time: p.end_time,
                total_events: p.total_events,
                contentions,
            }
        });

        AggregateResultJson {
            cpu,
            lock,
            syscall: self.syscall.clone(),
            total_events: self.total_events,
        }
    }
}

/// Deserialize base64-encoded payloads and aggregate all events by type.
///
/// Each payload is a base64-encoded bincode `Message` containing `Vec<ProfileEvent>`.
/// Events are routed to the appropriate profile builder based on their variant.
pub fn aggregate_batches(payloads: &[String]) -> Result<AggregateResult> {
    let mut cpu: Option<Profile> = None;
    let mut lock: Option<LockProfile> = None;
    let mut syscall: Option<SyscallProfile> = None;
    let mut total_events: u64 = 0;

    for payload_b64 in payloads {
        let bytes = BASE64
            .decode(payload_b64)
            .context("base64 decode payload")?;
        let msg = Message::from_bytes(&bytes).context("bincode decode message")?;

        for event in msg.events {
            total_events += 1;
            match event {
                ProfileEvent::CpuSample(sample) => {
                    let profile = cpu.get_or_insert_with(|| {
                        Profile::new(sample.timestamp, sample.timestamp, 0)
                    });
                    if sample.timestamp < profile.start_time {
                        profile.start_time = sample.timestamp;
                    }
                    if sample.timestamp > profile.end_time {
                        profile.end_time = sample.timestamp;
                    }
                    // Combine user + kernel stacks with pre-resolved symbols
                    let mut ips = Vec::new();
                    let mut symbols: Vec<Option<String>> = Vec::new();
                    ips.extend_from_slice(&sample.user_stack);
                    symbols.extend_from_slice(&sample.user_stack_symbols);
                    while symbols.len() < ips.len() {
                        symbols.push(None);
                    }
                    ips.extend_from_slice(&sample.kernel_stack);
                    symbols.extend_from_slice(&sample.kernel_stack_symbols);
                    while symbols.len() < ips.len() {
                        symbols.push(None);
                    }
                    if !ips.is_empty() {
                        let has_symbols = symbols.iter().any(|s| s.is_some());
                        let stack = if has_symbols {
                            Stack::from_ips_with_symbols(&ips, &symbols)
                        } else {
                            Stack::from_ips(&ips)
                        };
                        profile.add_sample(stack);
                    }
                }
                ProfileEvent::Lock(ev) => {
                    let profile = lock.get_or_insert_with(|| LockProfile::new(ev.timestamp));
                    if ev.timestamp < profile.start_time {
                        profile.start_time = ev.timestamp;
                    }
                    if ev.timestamp > profile.end_time {
                        profile.end_time = ev.timestamp;
                    }
                    if !ev.stack_trace.is_empty() {
                        let has_symbols = ev.stack_symbols.iter().any(|s| s.is_some());
                        let stack = if has_symbols {
                            Stack::from_ips_with_symbols(&ev.stack_trace, &ev.stack_symbols)
                        } else {
                            Stack::from_ips(&ev.stack_trace)
                        };
                        profile.add_contention(ev.lock_addr, stack, ev.wait_time_ns);
                    }
                }
                ProfileEvent::Syscall(ev) => {
                    let profile = syscall.get_or_insert_with(|| SyscallProfile::new(ev.timestamp));
                    if ev.timestamp < profile.start_time {
                        profile.start_time = ev.timestamp;
                    }
                    if ev.timestamp > profile.end_time {
                        profile.end_time = ev.timestamp;
                    }
                    let name = syscall_name(ev.syscall_id);
                    profile.add_syscall(ev.syscall_id, name, ev.duration_ns, ev.return_value);
                }
                ProfileEvent::GpuKernel(_) => {
                    // GPU profiling not yet supported in aggregation
                }
            }
        }
    }

    Ok(AggregateResult {
        cpu,
        lock,
        syscall,
        total_events,
    })
}

/// Filter an AggregateResult to only include the requested event type.
pub fn filter_by_type(result: &mut AggregateResult, event_type: &str) {
    match event_type {
        "cpu" => {
            result.lock = None;
            result.syscall = None;
        }
        "lock" => {
            result.cpu = None;
            result.syscall = None;
        }
        "syscall" => {
            result.cpu = None;
            result.lock = None;
        }
        _ => {} // "" or "all" — keep everything
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_shared::types::events::{CpuSample, LockEvent, SyscallEvent};

    fn make_payload(events: Vec<ProfileEvent>) -> String {
        let msg = Message::new(1, events);
        let bytes = msg.to_bytes().unwrap();
        BASE64.encode(bytes)
    }

    fn cpu(ts: u64, user: Vec<u64>, kernel: Vec<u64>) -> ProfileEvent {
        ProfileEvent::CpuSample(CpuSample {
            timestamp: ts, pid: 1, tid: 1, cpu_id: 0,
            user_stack: user, kernel_stack: kernel,
            comm: "test".to_string(),
            user_stack_symbols: vec![], kernel_stack_symbols: vec![],
        })
    }

    fn lock_ev(ts: u64, addr: u64, wait_ns: u64, stack: Vec<u64>) -> ProfileEvent {
        ProfileEvent::Lock(LockEvent {
            timestamp: ts, pid: 1, tid: 1, lock_addr: addr,
            hold_time_ns: 0, wait_time_ns: wait_ns,
            stack_trace: stack, comm: "test".to_string(),
            stack_symbols: vec![],
        })
    }

    #[test]
    fn test_aggregate_cpu_samples() {
        let payload = make_payload(vec![
            cpu(1000, vec![0x1000, 0x2000], vec![]),
            cpu(2000, vec![0x1000, 0x2000], vec![]),
            cpu(3000, vec![0x3000], vec![]),
        ]);
        let result = aggregate_batches(&[payload]).unwrap();
        assert_eq!(result.total_events, 3);
        let cpu = result.cpu.unwrap();
        assert_eq!(cpu.total_samples, 3);
        assert_eq!(cpu.samples.len(), 2);
        assert!(result.lock.is_none());
        assert!(result.syscall.is_none());
    }

    #[test]
    fn test_aggregate_mixed_events() {
        let payload = make_payload(vec![
            cpu(1000, vec![0x1000], vec![]),
            ProfileEvent::Syscall(SyscallEvent {
                timestamp: 2000, pid: 1, tid: 1, syscall_id: 0,
                duration_ns: 100, return_value: 0, comm: "test".to_string(),
            }),
            lock_ev(3000, 0x1000, 500, vec![0x4000]),
        ]);
        let result = aggregate_batches(&[payload]).unwrap();
        assert_eq!(result.total_events, 3);
        assert!(result.cpu.is_some());
        assert!(result.lock.is_some());
        assert!(result.syscall.is_some());
    }

    #[test]
    fn test_aggregate_multiple_batches() {
        let p1 = make_payload(vec![cpu(1000, vec![0x1000], vec![])]);
        let p2 = make_payload(vec![cpu(2000, vec![0x1000], vec![])]);
        let result = aggregate_batches(&[p1, p2]).unwrap();
        assert_eq!(result.total_events, 2);
        let cpu = result.cpu.unwrap();
        assert_eq!(cpu.total_samples, 2);
        assert_eq!(cpu.samples.len(), 1);
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate_batches(&[]).unwrap();
        assert_eq!(result.total_events, 0);
        assert!(result.cpu.is_none());
        assert!(result.lock.is_none());
        assert!(result.syscall.is_none());
    }

    #[test]
    fn test_filter_by_type() {
        let payload = make_payload(vec![
            cpu(1000, vec![0x1000], vec![]),
            ProfileEvent::Syscall(SyscallEvent {
                timestamp: 2000, pid: 1, tid: 1, syscall_id: 0,
                duration_ns: 100, return_value: 0, comm: "test".to_string(),
            }),
        ]);
        let mut result = aggregate_batches(&[payload]).unwrap();
        filter_by_type(&mut result, "cpu");
        assert!(result.cpu.is_some());
        assert!(result.syscall.is_none());
    }

    #[test]
    fn test_aggregate_with_symbols() {
        let payload = make_payload(vec![ProfileEvent::CpuSample(CpuSample {
            timestamp: 1000, pid: 1, tid: 1, cpu_id: 0,
            user_stack: vec![0x1000, 0x2000],
            kernel_stack: vec![],
            comm: "test".to_string(),
            user_stack_symbols: vec![Some("main".to_string()), Some("compute".to_string())],
            kernel_stack_symbols: vec![],
        })]);
        let result = aggregate_batches(&[payload]).unwrap();
        let cpu = result.cpu.unwrap();
        let (stack, _) = cpu.samples.iter().next().unwrap();
        assert_eq!(stack.frames[0].function.as_deref(), Some("main"));
        assert_eq!(stack.frames[1].function.as_deref(), Some("compute"));
    }
}
