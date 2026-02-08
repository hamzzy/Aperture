//! WASM runtime implementation for event filtering.
//!
//! Loads user-supplied WASM modules that implement a `filter(ptr: i32, len: i32) -> i32`
//! export. The runtime writes an `EventContext` struct + comm string into the module's
//! linear memory, calls `filter()`, and interprets the return value as keep (1) or
//! discard (0).

use crate::filter_api::EventContext;
use anyhow::{Context, Result};
use aperture_shared::types::events::ProfileEvent;
use wasmtime::*;

/// Maximum WASM linear memory: 1 MB (16 pages of 64 KiB)
const MAX_MEMORY_PAGES: u32 = 16;

/// Fuel limit per filter invocation (roughly ~1M instructions)
const FUEL_PER_CALL: u64 = 1_000_000;

/// Base address where EventContext is written in WASM memory
const EVENT_BASE: i32 = 1024;

/// Compiled WASM filter, ready for execution.
pub struct WasmFilter {
    store: Store<()>,
    #[allow(dead_code)]
    instance: Instance,
    filter_fn: TypedFunc<(i32, i32), i32>,
    memory: Memory,
}

/// WASM runtime for loading and executing filter modules.
pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    /// Create a new WASM runtime with security-hardened configuration.
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_bulk_memory(true);
        config.wasm_multi_value(false);
        config.wasm_threads(false);

        let engine = Engine::new(&config).context("Failed to create WASM engine")?;
        Ok(Self { engine })
    }

    /// Compile and instantiate a WASM filter module.
    ///
    /// The module must export:
    ///   - `filter(ptr: i32, len: i32) -> i32` — returns 1 to keep, 0 to discard
    ///   - `memory` — linear memory export
    pub fn load_filter(&self, wasm_bytes: &[u8]) -> Result<WasmFilter> {
        let module =
            Module::new(&self.engine, wasm_bytes).context("Failed to compile WASM module")?;

        let mut store = Store::new(&self.engine, ());
        store.set_fuel(FUEL_PER_CALL).ok();

        // Create a linker with a host `log` function
        let mut linker = Linker::new(&self.engine);
        linker.func_wrap(
            "env",
            "log",
            |_caller: Caller<'_, ()>, _ptr: i32, _len: i32| {
                // In production, we could read the string from WASM memory and log it.
                // For now, silently ignore to prevent filter modules from spamming logs.
            },
        )?;

        // Provide a default memory if the module doesn't export one
        let memory_type = MemoryType::new(1, Some(MAX_MEMORY_PAGES));
        let default_memory = Memory::new(&mut store, memory_type)?;
        linker.define(&store, "env", "memory", default_memory)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .context("Failed to instantiate WASM module")?;

        // Get the exported memory (prefer module's own, fall back to provided)
        let memory = instance
            .get_memory(&mut store, "memory")
            .unwrap_or(default_memory);

        let filter_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "filter")
            .context("WASM module must export `filter(ptr: i32, len: i32) -> i32`")?;

        Ok(WasmFilter {
            store,
            instance: instance,
            filter_fn,
            memory,
        })
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create WASM runtime")
    }
}

impl WasmFilter {
    /// Run the filter on a single event. Returns `true` to keep, `false` to discard.
    pub fn filter_event(&mut self, event: &ProfileEvent) -> Result<bool> {
        // Reset fuel for this invocation
        self.store.set_fuel(FUEL_PER_CALL).ok();

        // Build the event context
        let (ctx, comm) = EventContext::from_event(event);
        let ctx_bytes = ctx.to_bytes();
        let total_len = ctx_bytes.len() + comm.len();

        // Write EventContext + comm string into WASM memory
        let mem_data = self.memory.data_mut(&mut self.store);
        let base = EVENT_BASE as usize;
        if base + total_len > mem_data.len() {
            anyhow::bail!(
                "Event data ({} bytes) exceeds WASM memory at offset {}",
                total_len,
                base
            );
        }
        mem_data[base..base + ctx_bytes.len()].copy_from_slice(&ctx_bytes);
        mem_data[base + ctx_bytes.len()..base + total_len].copy_from_slice(comm.as_bytes());

        // Call the filter function
        let result = self
            .filter_fn
            .call(&mut self.store, (EVENT_BASE, total_len as i32))
            .context("WASM filter execution failed")?;

        Ok(result != 0)
    }

    /// Filter a batch of events, returning only those the filter keeps.
    pub fn filter_batch(&mut self, events: Vec<ProfileEvent>) -> Result<Vec<ProfileEvent>> {
        let mut kept = Vec::with_capacity(events.len());
        for event in events {
            if self.filter_event(&event)? {
                kept.push(event);
            }
        }
        Ok(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = WasmRuntime::new().unwrap();
        drop(runtime);
    }

    // Note: Loading actual WASM modules requires compiled .wasm files.
    // Integration tests with real filters would go in tests/ directory.
}
