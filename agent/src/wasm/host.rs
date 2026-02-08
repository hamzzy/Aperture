//! Host functions available to WASM filters

use wasmtime::*;

/// Register host functions with the WASM linker
pub fn register_host_functions(linker: &mut Linker<()>) -> Result<(), Error> {
    // Log function for debugging filters
    linker.func_wrap(
        "env",
        "log",
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            // Read string from memory
            if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                let mut buf = vec![0u8; len as usize];
                if memory.read(&caller, ptr as usize, &mut buf).is_ok() {
                    if let Ok(msg) = String::from_utf8(buf) {
                        tracing::debug!("[WASM Filter] {}", msg);
                    }
                }
            }
        },
    )?;

    // Get current timestamp
    linker.func_wrap("env", "get_timestamp", || -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    })?;

    Ok(())
}
