//! Query command implementation

use anyhow::{Context, Result};
use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
use aperture_aggregator::server::grpc::proto::QueryRequest;
use clap::Args;
use tonic::transport::Channel;

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Aggregator gRPC endpoint
    #[arg(short, long, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,

    /// Filter by agent ID (optional)
    #[arg(long)]
    pub agent_id: Option<String>,

    /// Max batches to return
    #[arg(short, long, default_value = "100")]
    pub limit: u32,
}

pub async fn run(args: QueryArgs) -> Result<()> {
    let mut client = AggregatorClient::<Channel>::connect(args.endpoint.clone())
        .await
        .context("Failed to connect to aggregator")?;

    let request = QueryRequest {
        agent_id: args.agent_id.clone(),
        limit: args.limit,
    };

    let response = client
        .query(tonic::Request::new(request))
        .await
        .context("Query failed")?;

    let res = response.into_inner();
    if !res.error.is_empty() {
        anyhow::bail!("Aggregator error: {}", res.error);
    }

    if res.batches.is_empty() {
        println!("No batches in buffer.");
        return Ok(());
    }

    println!("{} batch(es):", res.batches.len());
    for b in res.batches {
        println!(
            "  agent_id={} sequence={} events={} received_at_ns={}",
            b.agent_id, b.sequence, b.event_count, b.received_at_ns
        );
    }

    Ok(())
}
