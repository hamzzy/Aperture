//! WASM filter API definitions (Phase 3+)
//!
//! Defines the host functions available to WASM filter modules

// TODO Phase 3: Define filter API
// Example API functions:
// - get_event_type() -> EventType
// - get_pid() -> i32
// - get_stack_depth() -> u32
// - get_function_name(index: u32) -> String
// - log(message: String)

/// Filter API host functions
pub struct FilterApi {
    // TODO: Store event context for current filter invocation
}

impl FilterApi {
    /// Create a new filter API
    pub fn new() -> Self {
        Self {}
    }

    // TODO: Implement host functions that WASM modules can call
}

impl Default for FilterApi {
    fn default() -> Self {
        Self::new()
    }
}
