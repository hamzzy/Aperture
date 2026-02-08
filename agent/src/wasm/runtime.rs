//! WASM runtime for executing filter plugins

use anyhow::{Context, Result};
use std::path::Path;
use wasmtime::*;

use aperture_shared::wasm::{FilterInput, FilterResult};

/// WASM runtime for executing filters
pub struct WasmRuntime {
    engine: Engine,
    module: Module,
    store: Store<()>,
}

impl WasmRuntime {
    /// Create a new WASM runtime from a filter file
    pub fn new(filter_path: &Path) -> Result<Self> {
        // Create engine with default configuration
        let mut config = Config::new();
        config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        config.consume_fuel(true); // Enable fuel for timeout protection

        let engine = Engine::new(&config)?;

        // Load and compile the WASM module
        let module =
            Module::from_file(&engine, filter_path).context("Failed to load WASM module")?;

        // Create store with fuel limit (prevents infinite loops)
        let mut store = Store::new(&engine, ());
        store.set_fuel(1_000_000)?; // Limit execution time

        Ok(Self {
            engine,
            module,
            store,
        })
    }

    /// Execute the filter on an input event
    pub fn execute(&mut self, input: &FilterInput) -> Result<FilterResult> {
        // Serialize input
        let input_bytes = bincode::serialize(input).context("Failed to serialize filter input")?;

        // Create instance
        let instance = Instance::new(&mut self.store, &self.module, &[])
            .context("Failed to create WASM instance")?;

        // Get memory
        let memory = instance
            .get_memory(&mut self.store, "memory")
            .context("Failed to get WASM memory")?;

        // Allocate memory for input
        let alloc = instance
            .get_typed_func::<u32, u32>(&mut self.store, "alloc")
            .context("Failed to get alloc function")?;

        let input_ptr = alloc
            .call(&mut self.store, input_bytes.len() as u32)
            .context("Failed to allocate memory")?;

        // Write input to memory
        memory
            .write(&mut self.store, input_ptr as usize, &input_bytes)
            .context("Failed to write input to memory")?;

        // Call filter function
        let filter = instance
            .get_typed_func::<(u32, u32), u32>(&mut self.store, "filter")
            .context("Failed to get filter function")?;

        let output_ptr = filter
            .call(&mut self.store, (input_ptr, input_bytes.len() as u32))
            .context("Failed to call filter function")?;

        // Read output length (first 4 bytes)
        let mut len_bytes = [0u8; 4];
        memory
            .read(&self.store, output_ptr as usize, &mut len_bytes)
            .context("Failed to read output length")?;
        let output_len = u32::from_le_bytes(len_bytes) as usize;

        // Read output data
        let mut output_bytes = vec![0u8; output_len];
        memory
            .read(&self.store, (output_ptr + 4) as usize, &mut output_bytes)
            .context("Failed to read output data")?;

        // Deserialize output
        let result: FilterResult =
            bincode::deserialize(&output_bytes).context("Failed to deserialize filter output")?;

        // Deallocate memory
        let dealloc = instance
            .get_typed_func::<(u32, u32), ()>(&mut self.store, "dealloc")
            .context("Failed to get dealloc function")?;

        dealloc
            .call(&mut self.store, (input_ptr, input_bytes.len() as u32))
            .context("Failed to deallocate input")?;

        dealloc
            .call(&mut self.store, (output_ptr, output_len as u32 + 4))
            .context("Failed to deallocate output")?;

        Ok(result)
    }

    /// Reset fuel for next execution
    pub fn reset_fuel(&mut self) -> Result<()> {
        self.store.set_fuel(1_000_000)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires compiled WASM filter
    fn test_runtime_creation() {
        // This test requires a compiled WASM filter
        let result = WasmRuntime::new(Path::new("test_filter.wasm"));
        assert!(result.is_err()); // Expected to fail until we have a filter
    }
}
