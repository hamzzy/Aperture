//! Aggregator Service (Phase 5+)
//!
//! Receives profiling data from multiple agents and stores it in a centralized database

use anyhow::Result;
use tracing::info;

mod config;
mod server;
mod storage;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting eBPF profiler aggregator");

    println!("TODO Phase 5: Implement aggregator service");
    println!("This service will:");
    println!("  1. Accept connections from profiling agents");
    println!("  2. Receive profiling data via gRPC");
    println!("  3. Store data in ClickHouse or ScyllaDB");
    println!("  4. Provide query API for CLI/UI");

    Ok(())
}
