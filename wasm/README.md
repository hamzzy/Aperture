# Aperture WASM Filter SDK

Write custom filters for Aperture profiling events using WebAssembly.

## Quick Start

### 1. Create a new filter

```rust
use aperture_filter::*;

filter_fn!(my_filter, |input: &FilterInput| {
    // Only keep events from Python processes
    if input.comm.contains("python") {
        FilterResult::Keep
    } else {
        FilterResult::Drop
    }
});
```

### 2. Build the filter

```bash
cargo build --target wasm32-wasip1 --release --example process_filter
```

### 3. Use the filter

```bash
aperture-agent --mode cpu --duration 10s --output profile.svg \
    --filter target/wasm32-wasip1/release/examples/process_filter.wasm
```

## Filter API

### FilterInput

```rust
pub struct FilterInput {
    pub event_type: String,  // "cpu", "lock", "syscall"
    pub pid: u32,
    pub tid: u32,
    pub timestamp: u64,
    pub comm: String,         // Process name
    pub stack_trace: Vec<u64>,
    pub event_data: String,   // Event-specific JSON data
}
```

### FilterResult

```rust
pub enum FilterResult {
    Keep,                    // Keep the event as-is
    Drop,                    // Drop the event
    Transform(FilterInput),  // Transform the event
}
```

## Examples

See `examples/` for complete filter implementations:
- `process_filter.rs` - Filter by process name
- `stack_filter.rs` - Filter by stack depth
- `sampling_filter.rs` - Time-based sampling

## Performance

Filters should execute in <100μs per event to maintain low overhead.
Use the `log()` function sparingly as it impacts performance.
