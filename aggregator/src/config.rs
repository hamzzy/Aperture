//! Aggregator configuration (Phase 5+)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorConfig {
    /// Listen address for gRPC server
    pub listen_addr: String,

    /// Storage backend configuration
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    ClickHouse { endpoint: String, database: String },
    Scylla { endpoints: Vec<String>, keyspace: String },
    InMemory,
}
