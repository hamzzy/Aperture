//! JSON output
//!
//! Exports profile data in JSON format for further analysis

use anyhow::{Context, Result};
use aperture_shared::types::profile::Profile;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use tracing::info;

/// JSON-serializable profile representation
#[derive(Serialize)]
struct JsonProfile<'a> {
    start_time: u64,
    end_time: u64,
    total_samples: u64,
    sample_period_ns: u64,
    samples: Vec<JsonSample<'a>>,
}

/// A single aggregated stack sample
#[derive(Serialize)]
struct JsonSample<'a> {
    count: u64,
    frames: Vec<&'a aperture_shared::types::profile::Frame>,
}

/// Generate JSON output from profile data
pub fn generate_json(profile: &Profile, output_path: &str) -> Result<()> {
    info!("Generating JSON output: {}", output_path);

    let samples: Vec<JsonSample> = profile
        .samples
        .iter()
        .map(|(stack, count)| JsonSample {
            count: *count,
            frames: stack.frames.iter().collect(),
        })
        .collect();

    let json_profile = JsonProfile {
        start_time: profile.start_time,
        end_time: profile.end_time,
        total_samples: profile.total_samples,
        sample_period_ns: profile.sample_period_ns,
        samples,
    };

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &json_profile)
        .context("Failed to serialize profile to JSON")?;

    info!("JSON output written to {}", output_path);

    Ok(())
}

/// JSON-serializable lock profile
#[derive(Serialize)]
struct JsonLockProfile<'a> {
    start_time: u64,
    end_time: u64,
    total_events: u64,
    contentions: Vec<JsonLockContention<'a>>,
}

#[derive(Serialize)]
struct JsonLockContention<'a> {
    lock_addr: String,
    stack: Vec<&'a aperture_shared::types::profile::Frame>,
    count: u64,
    total_wait_ns: u64,
    max_wait_ns: u64,
    min_wait_ns: u64,
}

/// Generate JSON output from lock profile data
pub fn generate_lock_json(profile: &aperture_shared::types::profile::LockProfile, output_path: &str) -> Result<()> {
    info!("Generating lock profile JSON: {}", output_path);

    let contentions: Vec<JsonLockContention> = profile
        .contentions
        .iter()
        .map(|((addr, stack), stats)| JsonLockContention {
            lock_addr: format!("0x{:x}", addr),
            stack: stack.frames.iter().collect(),
            count: stats.count,
            total_wait_ns: stats.total_wait_ns,
            max_wait_ns: stats.max_wait_ns,
            min_wait_ns: stats.min_wait_ns,
        })
        .collect();

    let json_profile = JsonLockProfile {
        start_time: profile.start_time,
        end_time: profile.end_time,
        total_events: profile.total_events,
        contentions,
    };

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &json_profile)
        .context("Failed to serialize lock profile to JSON")?;

    info!("JSON output written to {}", output_path);
    Ok(())
}

/// JSON-serializable syscall profile
#[derive(Serialize)]
struct JsonSyscallProfile {
    start_time: u64,
    end_time: u64,
    total_events: u64,
    syscalls: Vec<JsonSyscallStats>,
}

#[derive(Serialize)]
struct JsonSyscallStats {
    id: u32,
    name: String,
    count: u64,
    total_duration_ns: u64,
    max_duration_ns: u64,
    min_duration_ns: u64,
    error_count: u64,
    latency_histogram: Vec<u64>,
}

/// Generate JSON output from syscall profile data
pub fn generate_syscall_json(profile: &aperture_shared::types::profile::SyscallProfile, output_path: &str) -> Result<()> {
    info!("Generating syscall profile JSON: {}", output_path);

    let syscalls: Vec<JsonSyscallStats> = profile
        .syscalls
        .values()
        .map(|stats| JsonSyscallStats {
            id: stats.syscall_id,
            name: stats.name.clone(),
            count: stats.count,
            total_duration_ns: stats.total_duration_ns,
            max_duration_ns: stats.max_duration_ns,
            min_duration_ns: stats.min_duration_ns,
            error_count: stats.error_count,
            latency_histogram: stats.latency_histogram.clone(),
        })
        .collect();

    let json_profile = JsonSyscallProfile {
        start_time: profile.start_time,
        end_time: profile.end_time,
        total_events: profile.total_events,
        syscalls,
    };

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &json_profile)
        .context("Failed to serialize syscall profile to JSON")?;

    info!("JSON output written to {}", output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_json() {
        let profile = Profile::new(0, 1000, 10_000_000);

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test.json");

        let result = generate_json(&profile, output_path.to_str().unwrap());
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());

        // Verify valid JSON
        let contents = std::fs::read_to_string(output_path).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    }

    #[test]
    fn test_json_contains_profile_metadata() {
        use aperture_shared::types::profile::{Frame, Stack};

        let mut profile = Profile::new(1000, 2000, 10_000_000);
        let stack = Stack {
            frames: vec![Frame {
                ip: 0x400000,
                function: Some("main".to_string()),
                file: None,
                line: None,
                module: None,
            }],
        };
        profile.add_sample(stack);

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test.json");
        generate_json(&profile, output_path.to_str().unwrap()).unwrap();

        let contents = std::fs::read_to_string(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert_eq!(parsed["start_time"], 1000);
        assert_eq!(parsed["end_time"], 2000);
        assert_eq!(parsed["total_samples"], 1);
        assert_eq!(parsed["sample_period_ns"], 10_000_000);
        assert_eq!(parsed["samples"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["samples"][0]["count"], 1);
    }
}
