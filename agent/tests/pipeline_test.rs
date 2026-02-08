use anyhow::Result;
use aperture_agent::collector::lock::LockCollector;
use aperture_agent::collector::syscall::SyscallCollector;
use aperture_shared::types::events::{LockEvent, SyscallEvent};
use aperture_agent::output::{flamegraph, histogram, json};
use tempfile::NamedTempFile;

#[test]
fn test_lock_pipeline() -> Result<()> {
    let mut collector = LockCollector::new();
    let event = LockEvent {
        timestamp: 1000,
        pid: 1,
        tid: 1,
        lock_addr: 0x1000,
        hold_time_ns: 0,
        wait_time_ns: 500,
        stack_trace: vec![0x400000],
        comm: "test".to_string(),
        stack_symbols: vec![],
    };
    collector.add_event(event);

    let profile = collector.build_profile()?;
    assert_eq!(profile.total_events, 1);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path().to_str().unwrap();

    // Test output generation
    flamegraph::generate_lock_flamegraph(&profile, path)?;
    
    let json_path = format!("{}.json", path);
    json::generate_lock_json(&profile, &json_path)?;

    Ok(())
}

#[test]
fn test_syscall_pipeline() -> Result<()> {
    let mut collector = SyscallCollector::new();
    let event = SyscallEvent {
        timestamp: 1000,
        pid: 1,
        tid: 1,
        syscall_id: 0,
        duration_ns: 100,
        return_value: 0,
        comm: "test".to_string(),
    };
    collector.add_event(event);

    let profile = collector.build_profile()?;
    assert_eq!(profile.total_events, 1);

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path().to_str().unwrap();

    // Test output generation
    histogram::generate_syscall_histogram(&profile, path)?;

    let json_path = format!("{}.json", path);
    json::generate_syscall_json(&profile, &json_path)?;

    Ok(())
}
