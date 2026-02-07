//! Shared types and utilities for Aperture
//!
//! This crate contains common data structures, types, and utilities used across
//! the profiler agent, aggregator, and other components.

pub mod types;
pub mod utils;
pub mod wasm;

#[cfg(feature = "wire-protocol")]
pub mod protocol;

// Re-export commonly used types
pub use types::{events::*, profile::*};
