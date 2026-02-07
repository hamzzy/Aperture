use aya::{EbpfLoader, programs::PerfEvent};
use aya::programs::perf_event::{PerfTypeId, PerfEventScope, SamplePolicy};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../target/bpfel-unknown-none/debug/minimal-test");

    println!("Loading eBPF program from: {:?}", path);

    let mut bpf = EbpfLoader::new()
        .allow_unsupported_maps()
        .load_file(&path)?;

    println!("Program loaded successfully!");

    // List programs
    for (name, _program) in bpf.programs() {
        println!("Found program: {}", name);
    }

    // Try to load and attach the program
    let program: &mut PerfEvent = bpf
        .program_mut("minimal_test")
        .ok_or("Failed to find minimal_test program")?
        .try_into()?;

    println!("Found minimal_test program, attempting to load...");

    program.load()?;

    println!("Program loaded into kernel successfully!");

    // Try to attach to CPU 0
    println!("Attempting to attach to CPU 0...");
    let _link = program.attach(
        PerfTypeId::Software,
        0,
        PerfEventScope::AllProcessesOneCpu { cpu: 0 },
        SamplePolicy::Frequency(99),
        false,
    )?;

    println!("SUCCESS! Program attached to CPU 0");

    Ok(())
}
