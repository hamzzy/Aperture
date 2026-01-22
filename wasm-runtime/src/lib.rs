//! WASM Filter Runtime (Phase 3+)
//!
//! Allows users to write custom filtering logic in any language that compiles
//! to WebAssembly. Filters can decide which events to keep or discard.

pub mod filter_api;
pub mod runtime;

use anyhow::Result;
use shared::types::events::ProfileEvent;

/// WASM filter trait
pub trait WasmFilter {
    /// Filter an event, returning true to keep it, false to discard
    fn filter_event(&mut self, event: &ProfileEvent) -> Result<bool>;
}

// TODO Phase 3: Implement WASM runtime for event filtering
// - Load WASM modules compiled from user code
// - Provide API for filtering events
// - Handle security and sandboxing
// - Support common filter patterns (PID, stack depth, function names, etc.)
