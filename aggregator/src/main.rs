//! Aggregator Service (Phase 5+)
//!
//! Receives profiling data from multiple agents via gRPC and buffers it in memory.
//! Optional Phase 6: persist to ClickHouse when APERTURE_CLICKHOUSE_ENDPOINT is set.

use anyhow::{Context, Result};
use aperture_aggregator::{
    buffer::InMemoryBuffer,
    config::{AggregatorConfig, StorageConfig},
    server::grpc,
    storage::BatchStore,
};
use std::sync::Arc;
use tokio::signal;
use tonic::codec::CompressionEncoding;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = load_config();
    info!("Starting Aperture aggregator on {}", config.listen_addr);

    let buffer = Arc::new(InMemoryBuffer::new(config.max_buffer_batches));
    #[allow(unused_mut)]
    let mut service = grpc::AggregatorService::new(buffer.clone());

    #[cfg(feature = "clickhouse-storage")]
    let store_handle: Option<Arc<dyn BatchStore>> = if let StorageConfig::ClickHouse {
        ref endpoint,
        ref database,
    } = &config.storage
    {
        let store =
            aperture_aggregator::storage::clickhouse::ClickHouseStore::new(endpoint, database)
                .await
                .context("ClickHouse connection failed")?;
        info!("ClickHouse storage enabled: {} / {}", endpoint, database);
        let store_arc = Arc::new(store);
        service = service.with_batch_store(store_arc.clone());
        Some(store_arc)
    } else {
        None
    };
    #[cfg(not(feature = "clickhouse-storage"))]
    let store_handle: Option<Arc<dyn BatchStore>> = None;

    let service = service.with_auth_token(config.auth_token.clone());

    let (_, health_svc) = tonic_health::server::health_reporter();
    let addr = config
        .listen_addr
        .parse()
        .context("Invalid listen address")?;
    let admin_addr = config
        .admin_addr
        .parse()
        .context("Invalid admin listen address")?;

    let store_for_admin = store_handle.clone();
    let admin_handle = tokio::spawn(async move {
        if let Err(e) =
            aperture_aggregator::server::http::serve_admin(admin_addr, buffer, store_for_admin)
                .await
        {
            tracing::error!("Admin HTTP server error: {}", e);
        }
    });

    let grpc_svc = grpc::GrpcAggregatorServer::new(service)
        .max_decoding_message_size(config.max_message_size)
        .max_encoding_message_size(config.max_message_size)
        .accept_compressed(CompressionEncoding::Gzip)
        .send_compressed(CompressionEncoding::Gzip);
    let result = Server::builder()
        .add_service(health_svc)
        .add_service(grpc_svc)
        .serve_with_shutdown(addr, shutdown_signal())
        .await;
    result.context("gRPC server error")?;

    admin_handle.abort();
    let _ = admin_handle.await;

    if let Some(store) = store_handle {
        if let Err(e) = store.shutdown().await {
            tracing::warn!("Store shutdown: {}", e);
        }
    }

    info!("Aggregator shut down cleanly");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);

    if std::env::var("APERTURE_LOG_FORMAT").as_deref() == Ok("json") {
        registry.with(fmt::layer().json().with_target(true)).init();
    } else {
        registry.with(fmt::layer().with_target(false)).init();
    }
}
