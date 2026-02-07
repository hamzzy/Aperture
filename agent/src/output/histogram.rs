//! Histogram generation
//!
//! Generates text-based histograms for latency analysis

use anyhow::{Context, Result};
use aperture_shared::types::profile::SyscallProfile;
use std::fs::File;
use std::io::{BufWriter, Write};
use tracing::info;

/// Generate a text histogram from syscall profile data
pub fn generate_syscall_histogram(profile: &SyscallProfile, output_path: &str) -> Result<()> {
    info!("Generating syscall histogram: {}", output_path);

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create histogram file: {}", output_path))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "Syscall Latency Profile")?;
    writeln!(writer, "=======================")?;
    
    let duration_secs = profile.end_time.saturating_sub(profile.start_time) as f64 / 1_000_000_000.0;
    writeln!(writer, "Total Duration: {:.3} s", duration_secs)?;
    writeln!(writer, "Total Events:   {}", profile.total_events)?;

    if profile.total_events == 0 {
        writeln!(writer, "\nNo syscall events collected.")?;
        return Ok(());
    }

    // Sort syscalls by total duration descending
    let mut syscalls: Vec<_> = profile.syscalls.values().collect();
    syscalls.sort_by(|a, b| b.total_duration_ns.cmp(&a.total_duration_ns));

    writeln!(
        writer, 
        "\n{:<20} {:>10} {:>12} {:>12} {:>12} {:>12} {:>10} {:>8}", 
        "Syscall", "Count", "Avg(ns)", "P50(ns)", "P99(ns)", "Max(ns)", "Errors", "ErrRate"
    )?;
    writeln!(writer, "{:-<120}", "")?;

    for s in syscalls {
        let avg = if s.count > 0 { s.total_duration_ns / s.count } else { 0 };
        let err_rate = if s.count > 0 { 
            (s.error_count as f64 / s.count as f64) * 100.0 
        } else { 
            0.0 
        };

        let p50 = estimate_percentile(&s.latency_histogram, s.count, 0.50);
        let p99 = estimate_percentile(&s.latency_histogram, s.count, 0.99);

        writeln!(
            writer, 
            "{:<20} {:>10} {:>12} {:>12} {:>12} {:>12} {:>10} {:>7.1}%",
            s.name, s.count, avg, p50, p99, s.max_duration_ns, s.error_count, err_rate
        )?;
    }

    info!("Histogram generated successfully: {}", output_path);
    Ok(())
}

fn estimate_percentile(histogram: &[u64], total: u64, percentile: f64) -> u64 {
    if total == 0 {
        return 0;
    }
    
    let target = (total as f64 * percentile) as u64;
    let mut current = 0;

    for (i, &count) in histogram.iter().enumerate() {
        current += count;
        if current >= target {
            // Bucket i covers 2^i to 2^(i+1)-1
            // Use upper bound as conservative estimate
            return 1u64 << (i + 1);
        }
    }
    
    // Fallback to highest bucket upper bound
    1u64 << histogram.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_percentile() {
        // Histogram buckets:
        // 0: 0..1
        // 1: 2..3
        // 2: 4..7
        // 3: 8..15
        
        let mut histogram = vec![0; 4];
        
        // Add 10 samples
        // 5 samples in bucket 0
        histogram[0] = 5;
        // 5 samples in bucket 2
        histogram[2] = 5;
        
        // P50 should be in bucket 0 (upper bound 2)
        assert_eq!(estimate_percentile(&histogram, 10, 0.50), 2);
        
        // P60 should be in bucket 2 (upper bound 8)
        assert_eq!(estimate_percentile(&histogram, 10, 0.60), 8);
        
        // P100 should be in bucket 2 (upper bound 8)
        assert_eq!(estimate_percentile(&histogram, 10, 1.0), 8);
    }
}
