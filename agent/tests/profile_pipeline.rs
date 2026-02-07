//! Integration test: full profile pipeline (collect → build → output)
//!
//! Tests the entire data path from sample collection through profile building
//! to flamegraph and JSON output, without requiring eBPF or root.

use aperture_agent::collector::cpu::CpuCollector;
use aperture_shared::types::events::CpuSample;
use aperture_shared::types::profile::Stack;

/// Simulate a realistic profiling session and verify the pipeline.
#[test]
fn test_full_pipeline_collect_build_output() {
    let mut collector = CpuCollector::new(10_101_010); // ~99 Hz

    // Simulate samples from two different stacks
    let hot_path = vec![0x400000, 0x400100, 0x400200]; // main → compute → inner
    let cold_path = vec![0x400000, 0x400300]; // main → io_wait

    for i in 0..80 {
        collector.add_sample(CpuSample {
            timestamp: 1_000_000_000 + i * 10_101_010,
            pid: 1234,
            tid: 1234,
            cpu_id: (i % 4) as u32,
            user_stack: hot_path.clone(),
            kernel_stack: vec![0xffffffff81000000],
            comm: "myapp".to_string(),
        });
    }
    for i in 0..20 {
        collector.add_sample(CpuSample {
            timestamp: 1_000_000_000 + (80 + i) * 10_101_010,
            pid: 1234,
            tid: 1235,
            cpu_id: 0,
            user_stack: cold_path.clone(),
            kernel_stack: vec![],
            comm: "myapp".to_string(),
        });
    }

    assert_eq!(collector.sample_count(), 100);

    // Build profile
    let profile = collector.build_profile().unwrap();
    assert_eq!(profile.total_samples, 100);
    assert_eq!(profile.samples.len(), 2); // 2 unique stacks

    // Verify the hot stack has 80 samples
    let hot_stack = Stack::from_ips(
        &[hot_path.as_slice(), &[0xffffffff81000000]].concat(),
    );
    assert_eq!(*profile.samples.get(&hot_stack).unwrap(), 80);

    // Generate flamegraph
    let temp_dir = tempfile::tempdir().unwrap();
    let svg_path = temp_dir.path().join("profile.svg");
    aperture_agent::output::flamegraph::generate_flamegraph(
        &profile,
        svg_path.to_str().unwrap(),
    )
    .unwrap();
    assert!(svg_path.exists());
    let svg_content = std::fs::read_to_string(&svg_path).unwrap();
    assert!(svg_content.contains("<svg"));
    assert!(svg_content.contains("samples"));

    // Generate JSON
    let json_path = temp_dir.path().join("profile.json");
    aperture_agent::output::json::generate_json(
        &profile,
        json_path.to_str().unwrap(),
    )
    .unwrap();
    assert!(json_path.exists());

    // Verify JSON structure
    let json_str = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["total_samples"], 100);
    assert_eq!(parsed["samples"].as_array().unwrap().len(), 2);
}
