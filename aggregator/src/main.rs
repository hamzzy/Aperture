//! Aggregator Service (Phase 5+)
//!
//! Receives profiling data from multiple agents via gRPC and buffers it in memory.
//! Optional Phase 6: persist to ClickHouse when APERTURE_CLICKHOUSE_ENDPOINT is set.

use anyhow::{Context, Result};
use aperture_aggregator::{
    buffer::InMemoryBuffer,
    config::{AggregatorConfig, StorageConfig},
    server::grpc,
};
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

fn load_config() -> AggregatorConfig {
    let mut config = AggregatorConfig::default();
    if let (Ok(endpoint), Ok(database)) = (
        std::env::var("APERTURE_CLICKHOUSE_ENDPOINT"),
        std::env::var("APERTURE_CLICKHOUSE_DATABASE"),
    ) {
        config.storage = StorageConfig::ClickHouse { endpoint, database };
    }
    config
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = load_config();
    info!("Starting Aperture aggregator on {}", config.listen_addr);

    let buffer = Arc::new(InMemoryBuffer::new(10_000));
    #[allow(unused_mut)]
    let mut service = grpc::AggregatorService::new(buffer);

    #[cfg(feature = "clickhouse-storage")]
    if let StorageConfig::ClickHouse { ref endpoint, ref database } = &config.storage {
        let store = aperture_aggregator::storage::clickhouse::ClickHouseStore::new(
            endpoint,
            database,
        )
        .await
        .context("ClickHouse connection failed")?;
        info!("ClickHouse storage enabled: {} / {}", endpoint, database);
        service = service.with_batch_store(Arc::new(store));
    }

    let service = service.into_server();

    let addr = config
        .listen_addr
        .parse()
        .context("Invalid listen address")?;

    Server::builder()
        .add_service(service)
        .serve(addr)
        .await
        .context("gRPC server error")?;

    Ok(())
}
