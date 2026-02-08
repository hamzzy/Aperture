//! Aggregator service library

/// Maximum number of batches to fetch for aggregate/diff. Combined with
/// `MAX_JSON_STACKS` in aggregate.rs, this keeps JSON response size under
/// the V8/browser string limit (~512 MB).
pub const MAX_AGGREGATE_BATCH_LIMIT: u32 = 100;

pub mod aggregate;
pub mod alerts;
pub mod audit;
pub mod buffer;
pub mod config;
pub mod metrics;
pub mod server;
pub mod storage;
