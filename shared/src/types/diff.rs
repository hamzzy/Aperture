//! Differential profiling types and computation
//!
//! Compares two profiles (baseline vs comparison) and computes per-entry deltas.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::profile::{LockProfile, Profile, Stack, SyscallProfile};

// ── CPU diff ────────────────────────────────────────────────────────────────

/// Diff of two CPU profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuDiff {
    pub baseline_total: u64,
    pub comparison_total: u64,
    /// Per-stack diffs sorted by |delta| descending.
    pub stacks: Vec<StackDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackDiff {
    pub stack: Stack,
    pub baseline_count: u64,
    pub comparison_count: u64,
    /// comparison - baseline
    pub delta: i64,
    /// delta / baseline * 100 (0.0 if baseline is 0)
    pub delta_pct: f64,
}

/// Compare two CPU profiles stack-by-stack.
pub fn diff_cpu(baseline: &Profile, comparison: &Profile) -> CpuDiff {
    let all_stacks: HashSet<&Stack> = baseline
        .samples
        .keys()
        .chain(comparison.samples.keys())
        .collect();

    let mut stacks: Vec<StackDiff> = all_stacks
        .into_iter()
        .map(|stack| {
            let b = baseline.samples.get(stack).copied().unwrap_or(0);
            let c = comparison.samples.get(stack).copied().unwrap_or(0);
            let delta = c as i64 - b as i64;
            let delta_pct = if b > 0 {
                delta as f64 / b as f64 * 100.0
            } else {
                0.0
            };
            StackDiff {
                stack: stack.clone(),
                baseline_count: b,
                comparison_count: c,
                delta,
                delta_pct,
            }
        })
        .collect();

    stacks.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));

    CpuDiff {
        baseline_total: baseline.total_samples,
        comparison_total: comparison.total_samples,
        stacks,
    }
}

// ── Syscall diff ────────────────────────────────────────────────────────────

/// Diff of two syscall profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallDiff {
    pub baseline_total: u64,
    pub comparison_total: u64,
    pub syscalls: Vec<SyscallStatsDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallStatsDiff {
    pub syscall_id: u32,
    pub name: String,
    pub baseline_count: u64,
    pub comparison_count: u64,
    pub delta_count: i64,
    pub baseline_avg_ns: f64,
    pub comparison_avg_ns: f64,
    pub delta_avg_ns: f64,
}

/// Compare two syscall profiles per-syscall.
pub fn diff_syscall(baseline: &SyscallProfile, comparison: &SyscallProfile) -> SyscallDiff {
    let all_ids: HashSet<u32> = baseline
        .syscalls
        .keys()
        .chain(comparison.syscalls.keys())
        .copied()
        .collect();

    let mut syscalls: Vec<SyscallStatsDiff> = all_ids
        .into_iter()
        .map(|id| {
            let b = baseline.syscalls.get(&id);
            let c = comparison.syscalls.get(&id);

            let b_count = b.map_or(0, |s| s.count);
            let c_count = c.map_or(0, |s| s.count);
            let b_avg = b.map_or(0.0, |s| {
                if s.count > 0 {
                    s.total_duration_ns as f64 / s.count as f64
                } else {
                    0.0
                }
            });
            let c_avg = c.map_or(0.0, |s| {
                if s.count > 0 {
                    s.total_duration_ns as f64 / s.count as f64
                } else {
                    0.0
                }
            });
            let name = b
                .map(|s| s.name.clone())
                .or_else(|| c.map(|s| s.name.clone()))
                .unwrap_or_default();

            SyscallStatsDiff {
                syscall_id: id,
                name,
                baseline_count: b_count,
                comparison_count: c_count,
                delta_count: c_count as i64 - b_count as i64,
                baseline_avg_ns: b_avg,
                comparison_avg_ns: c_avg,
                delta_avg_ns: c_avg - b_avg,
            }
        })
        .collect();

    syscalls.sort_by(|a, b| {
        b.delta_count
            .unsigned_abs()
            .cmp(&a.delta_count.unsigned_abs())
    });

    SyscallDiff {
        baseline_total: baseline.total_events,
        comparison_total: comparison.total_events,
        syscalls,
    }
}

// ── Lock diff ───────────────────────────────────────────────────────────────

/// Diff of two lock contention profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockDiff {
    pub baseline_total: u64,
    pub comparison_total: u64,
    pub contentions: Vec<LockContentionDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockContentionDiff {
    pub lock_addr: u64,
    pub stack: Stack,
    pub baseline_count: u64,
    pub comparison_count: u64,
    pub delta_wait_ns: i64,
}

/// Compare two lock contention profiles per (lock_addr, stack).
pub fn diff_lock(baseline: &LockProfile, comparison: &LockProfile) -> LockDiff {
    let all_keys: HashSet<&(u64, Stack)> = baseline
        .contentions
        .keys()
        .chain(comparison.contentions.keys())
        .collect();

    let mut contentions: Vec<LockContentionDiff> = all_keys
        .into_iter()
        .map(|key| {
            let b = baseline.contentions.get(key);
            let c = comparison.contentions.get(key);
            let b_wait = b.map_or(0, |s| s.total_wait_ns);
            let c_wait = c.map_or(0, |s| s.total_wait_ns);
            LockContentionDiff {
                lock_addr: key.0,
                stack: key.1.clone(),
                baseline_count: b.map_or(0, |s| s.count),
                comparison_count: c.map_or(0, |s| s.count),
                delta_wait_ns: c_wait as i64 - b_wait as i64,
            }
        })
        .collect();

    contentions.sort_by(|a, b| {
        b.delta_wait_ns
            .unsigned_abs()
            .cmp(&a.delta_wait_ns.unsigned_abs())
    });

    LockDiff {
        baseline_total: baseline.total_events,
        comparison_total: comparison.total_events,
        contentions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_cpu_basic() {
        let mut baseline = Profile::new(0, 1000, 10_000_000);
        let mut comparison = Profile::new(1000, 2000, 10_000_000);

        let stack_a = Stack::from_ips(&[0x1000, 0x2000]);
        let stack_b = Stack::from_ips(&[0x3000]);

        // baseline: A=10, B=5
        for _ in 0..10 {
            baseline.add_sample(stack_a.clone());
        }
        for _ in 0..5 {
            baseline.add_sample(stack_b.clone());
        }

        // comparison: A=7, B=8
        for _ in 0..7 {
            comparison.add_sample(stack_a.clone());
        }
        for _ in 0..8 {
            comparison.add_sample(stack_b.clone());
        }

        let diff = diff_cpu(&baseline, &comparison);
        assert_eq!(diff.baseline_total, 15);
        assert_eq!(diff.comparison_total, 15);
        assert_eq!(diff.stacks.len(), 2);

        // Sorted by |delta| descending: A has delta=-3, B has delta=+3 → either first
        let a_diff = diff.stacks.iter().find(|s| s.stack == stack_a).unwrap();
        assert_eq!(a_diff.baseline_count, 10);
        assert_eq!(a_diff.comparison_count, 7);
        assert_eq!(a_diff.delta, -3);
        assert!((a_diff.delta_pct - (-30.0)).abs() < 0.01);

        let b_diff = diff.stacks.iter().find(|s| s.stack == stack_b).unwrap();
        assert_eq!(b_diff.delta, 3);
        assert!((b_diff.delta_pct - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_diff_cpu_new_stack_in_comparison() {
        let baseline = Profile::new(0, 1000, 10_000_000);
        let mut comparison = Profile::new(1000, 2000, 10_000_000);

        let stack = Stack::from_ips(&[0x1000]);
        comparison.add_sample(stack.clone());

        let diff = diff_cpu(&baseline, &comparison);
        assert_eq!(diff.stacks.len(), 1);
        assert_eq!(diff.stacks[0].baseline_count, 0);
        assert_eq!(diff.stacks[0].comparison_count, 1);
        assert_eq!(diff.stacks[0].delta, 1);
        // delta_pct is 0 when baseline is 0
        assert_eq!(diff.stacks[0].delta_pct, 0.0);
    }

    #[test]
    fn test_diff_syscall_basic() {
        let mut baseline = SyscallProfile::new(0);
        let mut comparison = SyscallProfile::new(1000);

        // baseline: read=10 calls, 1000ns total
        for _ in 0..10 {
            baseline.add_syscall(0, "read", 100, 0);
        }
        // comparison: read=20 calls, 4000ns total
        for _ in 0..20 {
            comparison.add_syscall(0, "read", 200, 0);
        }

        let diff = diff_syscall(&baseline, &comparison);
        assert_eq!(diff.syscalls.len(), 1);
        let s = &diff.syscalls[0];
        assert_eq!(s.name, "read");
        assert_eq!(s.baseline_count, 10);
        assert_eq!(s.comparison_count, 20);
        assert_eq!(s.delta_count, 10);
        assert!((s.baseline_avg_ns - 100.0).abs() < 0.01);
        assert!((s.comparison_avg_ns - 200.0).abs() < 0.01);
        assert!((s.delta_avg_ns - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_diff_lock_basic() {
        let mut baseline = LockProfile::new(0);
        let mut comparison = LockProfile::new(1000);

        let stack = Stack::from_ips(&[0x400000]);

        baseline.add_contention(0x1000, stack.clone(), 500);
        baseline.add_contention(0x1000, stack.clone(), 300);

        comparison.add_contention(0x1000, stack.clone(), 1000);

        let diff = diff_lock(&baseline, &comparison);
        assert_eq!(diff.contentions.len(), 1);
        let c = &diff.contentions[0];
        assert_eq!(c.lock_addr, 0x1000);
        assert_eq!(c.baseline_count, 2);
        assert_eq!(c.comparison_count, 1);
        // baseline total_wait = 800, comparison = 1000
        assert_eq!(c.delta_wait_ns, 200);
    }
}
