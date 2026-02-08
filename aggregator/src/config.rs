//! Aggregator configuration

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
    InMemory,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            listen_addr: std::env::var("APERTURE_AGGREGATOR_LISTEN")
                .unwrap_or_else(|_| "0.0.0.0:50051".to_string()),
            storage: StorageConfig::InMemory,
        }
    }
}
