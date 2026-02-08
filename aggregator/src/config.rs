//! Aggregator configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorConfig {
    /// Listen address for gRPC server
    pub listen_addr: String,

    /// Admin HTTP listen address (health checks + metrics)
    pub admin_addr: String,

    /// Storage backend configuration
    pub storage: StorageConfig,

    /// Max batches in the in-memory ring buffer
    pub max_buffer_batches: usize,

    /// Max gRPC message size in bytes
    pub max_message_size: usize,

    /// Optional bearer token for gRPC authentication
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    ClickHouse { endpoint: String, database: String },
    InMemory,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        let max_message_mb: usize = std::env::var("APERTURE_MAX_MESSAGE_SIZE_MB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16);

        Self {
            listen_addr: std::env::var("APERTURE_AGGREGATOR_LISTEN")
                .unwrap_or_else(|_| "0.0.0.0:50051".to_string()),
            admin_addr: std::env::var("APERTURE_ADMIN_LISTEN")
                .unwrap_or_else(|_| "0.0.0.0:9090".to_string()),
            storage: StorageConfig::InMemory,
            max_buffer_batches: std::env::var("APERTURE_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
            max_message_size: max_message_mb * 1024 * 1024,
            auth_token: std::env::var("APERTURE_AUTH_TOKEN").ok(),
        }
    }
}
