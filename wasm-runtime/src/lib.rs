//! WASM Filter Runtime
//!
//! Allows users to write custom filtering logic in any language that compiles
//! to WebAssembly. Filters can decide which events to keep or discard.
//!
//! # Filter API
//!
//! A WASM filter module must export:
//!
//! - `filter(ptr: i32, len: i32) -> i32` — receives a pointer to an `EventContext`
//!   struct in linear memory and returns 1 to keep the event or 0 to discard.
//!
//! The host provides:
//!
//! - `env.log(ptr: i32, len: i32)` — log a message from the filter (optional)
//! - `env.memory` — shared linear memory
//!
//! # Example (Rust, compiled to wasm32-unknown-unknown)
//!
//! ```rust,ignore
//! #[repr(C)]
//! struct EventContext {
//!     event_type: u32,  // 0=CPU, 1=Lock, 2=Syscall, 3=GPU
//!     pid: i32,
//!     tid: i32,
//!     // ... (see filter_api::EventContext for full layout)
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn filter(ptr: i32, _len: i32) -> i32 {
//!     let ctx = unsafe { &*(ptr as *const EventContext) };
//!     // Keep only events from PID 1234
//!     if ctx.pid == 1234 { 1 } else { 0 }
//! }
//! ```

pub mod filter_api;
pub mod runtime;

// Re-export key types
pub use filter_api::EventContext;
pub use runtime::{WasmFilter, WasmRuntime};
