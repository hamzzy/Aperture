//! eBPF Profiler Agent Library
//!
//! This library provides the core functionality for the profiling agent,
//! including eBPF program loading, event collection, and symbol resolution.

pub mod collector;
pub mod config;
pub mod ebpf;
pub mod output;

pub use config::Config;
