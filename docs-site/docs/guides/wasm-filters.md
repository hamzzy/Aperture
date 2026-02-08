---
sidebar_position: 3
title: WASM Filters
---

# WASM Filters

Aperture supports programmable event filtering using WebAssembly (WASM) modules. Filters run in a sandboxed wasmtime runtime and can selectively keep or discard profiling events before they are stored or transmitted.

## How It Works

```
ProfileEvent → EventContext (flat C struct) → WASM linear memory
                                                    │
                                              filter(ptr, len) → i32
                                                    │
                                              1 = keep, 0 = discard
```

Each profiling event is converted to an `EventContext` struct and written into WASM linear memory at a fixed offset. The filter's exported `filter` function is called with the pointer and length, returning `1` to keep the event or `0` to discard it.

## Security Model

- **Fuel-limited execution:** Each filter call gets ~1M fuel units (roughly 1M instructions). Infinite loops are terminated.
- **Bounded memory:** Maximum 1MB of WASM linear memory (16 pages of 64 KiB).
- **No threads:** Multi-threading is disabled in the WASM engine.
- **No network/filesystem:** Filters cannot access the host system.

## EventContext

The `EventContext` struct is the data passed to WASM filters:

```rust
#[repr(C)]
pub struct EventContext {
    pub event_type: u32,      // 0=CPU, 1=Lock, 2=Syscall
    pub pid: u32,
    pub tid: u32,
    pub timestamp_ns: u64,

    // CPU-specific
    pub cpu_id: u32,
    pub user_stack_id: i64,
    pub kernel_stack_id: i64,

    // Lock-specific
    pub lock_addr: u64,
    pub wait_ns: u64,

    // Syscall-specific
    pub syscall_id: u32,
    pub duration_ns: u64,
    pub return_value: i64,
}
```

## Writing a Filter

Filters must be compiled to WASM and export a `filter` function:

```rust
// Example: Only keep events from PID 1234
#[no_mangle]
pub extern "C" fn filter(ptr: *const u8, len: usize) -> i32 {
    if len < 12 {
        return 0;
    }
    unsafe {
        // pid is at offset 4 (after event_type u32)
        let pid = *(ptr.add(4) as *const u32);
        if pid == 1234 { 1 } else { 0 }
    }
}
```

Compile with:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

The output `.wasm` file can be loaded by the filter runtime.

## Host Functions

Filters have access to one host function for debugging:

| Function | Signature | Description |
|----------|-----------|-------------|
| `env.log` | `(ptr: i32, len: i32)` | Log a UTF-8 string to the agent's log output |

## Example Filters

### Filter by Event Type

Only keep CPU profiling events:

```rust
#[no_mangle]
pub extern "C" fn filter(ptr: *const u8, _len: usize) -> i32 {
    unsafe {
        let event_type = *(ptr as *const u32);
        if event_type == 0 { 1 } else { 0 }  // 0 = CPU
    }
}
```

### Filter by Syscall Duration

Only keep syscalls longer than 1ms:

```rust
#[no_mangle]
pub extern "C" fn filter(ptr: *const u8, len: usize) -> i32 {
    if len < 64 {
        return 0;
    }
    unsafe {
        let event_type = *(ptr as *const u32);
        if event_type != 2 {
            return 1;  // Not a syscall, keep it
        }
        // duration_ns is at a known offset in EventContext
        let duration_ns = *(ptr.add(56) as *const u64);
        if duration_ns >= 1_000_000 { 1 } else { 0 }  // 1ms threshold
    }
}
```

## API Usage

```rust
use aperture_wasm::{WasmRuntime, WasmFilter};

// Create runtime
let runtime = WasmRuntime::new()?;

// Load filter from WASM bytes
let mut filter = runtime.load_filter(&wasm_bytes)?;

// Filter individual events
let keep = filter.filter_event(&profile_event)?;

// Filter a batch (removes discarded events in-place)
filter.filter_batch(&mut events)?;
```
