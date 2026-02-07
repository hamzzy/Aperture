# Aperture

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](#license)

> Production-grade distributed profiler for CPU & GPU workloads built with Rust and eBPF

## Overview

Aperture is a high-performance profiling system that uses eBPF for low-overhead performance monitoring. The project aims to provide CPU, GPU, lock contention, and syscall profiling in a distributed architecture.


## Features (Planned)

- **CPU Profiling**: Continuous stack trace sampling with <1% overhead using eBPF
- **Lock Contention**: Detect mutex/futex bottlenecks in production
- **Syscall Tracing**: Latency histograms for system calls
- **WASM Filters**: Programmable event filtering
- **GPU Profiling**: CUDA kernel tracing and memory analysis
- **Distributed Architecture**: Agent-aggregator model with scalable storage

## Repository Structure

```
aperture/
├── agent/           # Main profiling agent
├── agent-ebpf/      # eBPF programs (kernel space)
├── shared/          # Common types and utilities
├── cli/             # Command-line interface
├── aggregator/      # Central aggregation service (future)
├── wasm-runtime/    # WASM filter runtime (future)
└── gpu-profiler/    # GPU profiling support (future)
```

## Prerequisites

- Linux kernel 5.8+ with BPF support
- Rust 1.75+ (stable for userspace, nightly for eBPF programs)
- LLVM 14+ and Clang
- Linux headers for your kernel

## Getting Started

```bash
# Clone the repository
git clone https://github.com/yourusername/aperture.git
cd aperture

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Check code quality
cargo clippy --workspace --all-targets
cargo fmt --check
```

## Development

This project is in early development. See [docs/roadmap.md](docs/roadmap.md) for the development plan.

### Workspace Crates

- **agent**: Main profiling agent that loads eBPF programs and collects performance data
- **agent-ebpf**: eBPF programs compiled to BPF bytecode
- **shared**: Common types, protocols, and utilities shared across crates
- **cli**: Command-line interface for profiling operations
- **aggregator**: Distributed aggregation service (future phase)
- **wasm-runtime**: WASM-based event filtering (future phase)
- **gpu-profiler**: GPU profiling capabilities (future phase)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) once available.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgments

Built with:

- [Aya](https://aya-rs.dev/) - Rust eBPF library
- [Tokio](https://tokio.rs/) - Async runtime
- [Inferno](https://github.com/jonhoo/inferno) - Flamegraph generation



Design the Phase 3 WASM Filter Engine implementation plan for Aperture.

## Context

Aperture is an eBPF profiler (CPU, lock, syscall). Phase 3 adds programmable WASM filtering of events.

## What Already Exists (READ CAREFULLY - much is already built)

### Shared types (`shared/src/wasm/mod.rs`) — 100% COMPLETE
```rust
pub struct FilterInput {
    pub event_type: String,
    pub pid: u32, pub tid: u32, pub timestamp: u64,
    pub comm: String,
    pub stack_trace: Vec<u64>,
    pub event_data: String,  // event-specific JSON
}
pub enum FilterResult { Keep, Drop, Transform(FilterInput) }
pub const FILTER_API_VERSION: u32 = 1;
```

### WASM runtime (`agent/src/wasm/runtime.rs`) — 95% COMPLETE
- `WasmRuntime::new(filter_path: &Path)` — loads WASM module, creates wasmtime engine+store
- `execute(&mut self, input: &FilterInput) -> Result<FilterResult>` — full implementation:
  - Serializes input with bincode, allocates WASM memory, calls filter(ptr, len)
  - Reads output, deserializes, deallocates
- Fuel-based timeout protection (1M fuel per execution)
- `reset_fuel()` method
- BUT: creates Instance with empty imports (`&[]`) — host functions NOT registered

### Host functions (`agent/src/wasm/host.rs`) — EXISTS but disconnected
- `register_host_functions(linker: &mut Linker<()>)` — registers `log` and `get_timestamp`
- NEVER called from runtime.rs

### Filter SDK (`wasm/src/lib.rs`) — HAS COMPILATION ERRORS
- Name collision: defines `pub extern "C" fn alloc(size: u32)` but also imports `std::alloc::alloc`
- `filter_fn!` macro — works in principle but needs alloc/dealloc fix
- `log()` and `get_timestamp()` host function wrappers
- Examples: process_filter.rs, stack_filter.rs, sampling_filter.rs

### Config (`agent/src/config.rs`)
- `filter_path: Option<PathBuf>` field exists but always set to `None`
- No CLI flag for it

### wasm-runtime crate (`wasm-runtime/`) — 5% stub, REDUNDANT
- Duplicates what `agent/src/wasm/` already has better
- References `ProfileEvent` not `FilterInput`
- Should be left alone (not used anywhere critical)

## What Needs To Be Done

### 1. Fix aperture-filter SDK compilation (`wasm/src/lib.rs`)
The alloc/dealloc name collision:
```rust
use std::alloc::{alloc, dealloc, Layout}; // imports alloc
#[no_mangle]
pub extern "C" fn alloc(size: u32) -> *mut u8 { // conflicts!
```
Fix by aliasing imports: `use std::alloc::{alloc as std_alloc, dealloc as std_dealloc, Layout};`
Then use `std_alloc(layout)` and `std_dealloc(ptr, layout)` internally.

### 2. Wire host functions into WasmRuntime (`agent/src/wasm/runtime.rs`)
Currently: `Instance::new(&mut self.store, &self.module, &[])` — empty imports
Need: Create a `Linker`, call `register_host_functions(&mut linker)`, then use `linker.instantiate()` instead of `Instance::new`.

### 3. Add --filter CLI flag
- `agent/src/main.rs`: Add `#[arg(long)] filter: Option<PathBuf>` and pass to config
- `cli/src/commands/profile.rs`: Same

### 4. Integrate filtering into the profiler pipeline (`agent/src/lib.rs`)
The key integration point: inside the per-CPU reader async tasks, after reading raw events, before calling `collector.process_event()`. 

For each profiler function (run_cpu_profiler, run_lock_profiler, run_syscall_profiler):
- If filter_path is Some, create WasmRuntime, wrap in Arc<Mutex<>>
- Pass filter to per-CPU tasks
- Build FilterInput from the raw event, execute filter, skip on Drop

Converting raw events to FilterInput:
- CPU: event_type="cpu", pid/tid from SampleEvent, comm from bytes, stack from StackTraceMap, event_data="{\"cpu_id\":N}"
- Lock: event_type="lock", pid/tid from LockEventBpf, comm, stack from StackTraceMap, event_data="{\"lock_addr\":\"0x...\",\"wait_time_ns\":N}"  
- Syscall: event_type="syscall", pid/tid from SyscallEventBpf, comm, no stack, event_data="{\"syscall_id\":N,\"duration_ns\":N,\"return_value\":N}"

### 5. Build and test the filter SDK (cross-compilation)
The SDK compiles to `wasm32-unknown-unknown` target:
```
cargo build --target wasm32-unknown-unknown --release -p aperture-filter --examples
```
This produces .wasm files in `target/wasm32-unknown-unknown/release/examples/`

### 6. E2E test
Run the profiler with a compiled filter:
```
sudo ./aperture-agent --mode syscall --duration 3s --filter process_filter.wasm --output /tmp/filtered.txt
```

## Design Decisions

### Where to filter: at collection time (inside per-CPU tasks)
- Filter BEFORE aggregation to save memory and CPU
- FilterInput is built from raw eBPF event + stack trace map lookup
- Drop events early, before they're added to collector

### Thread safety: Arc<Mutex<WasmRuntime>>
- WasmRuntime is not Send+Sync (wasmtime Store is not)
- Wrap in Mutex for shared access across per-CPU tasks
- Alternatively: create one WasmRuntime per CPU (more memory, less contention)
- Recommendation: single Arc<Mutex<WasmRuntime>> — filter execution is fast, contention minimal

### Transform handling
- FilterResult::Transform returns a modified FilterInput
- Convert back to CpuSample/LockEvent/SyscallEvent and add to collector
- For MVP, Transform can be treated as Keep (ignore modifications)

### Error handling
- Filter execution errors → log warning, keep event (fail-open)
- Filter load errors → return error to user (fail-fast)

## Implementation Order
1. Fix SDK compilation (wasm/src/lib.rs)
2. Wire host functions into runtime (agent/src/wasm/runtime.rs)
3. Add CLI flag (main.rs, profile.rs)
4. Add FilterInput conversion helpers
5. Integrate filter into profiler pipeline (lib.rs) 
6. Build SDK examples to wasm32
7. Run tests
8. E2E verification

Please produce a detailed, step-by-step implementation plan.