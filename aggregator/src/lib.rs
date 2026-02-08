//! Aggregator service library

/// Maximum number of batches to fetch for aggregate/diff. Keeps JSON response size
/// under V8/browser string limit (~0x1fffffe8 chars). Increase only with pagination/streaming.
pub const MAX_AGGREGATE_BATCH_LIMIT: u32 = 500;

pub mod aggregate;
pub mod audit;
pub mod buffer;
pub mod config;
pub mod metrics;
pub mod server;
pub mod storage;
