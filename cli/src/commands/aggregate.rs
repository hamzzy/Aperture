//! Aggregate command implementation

use anyhow::{Context, Result};
use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
use aperture_aggregator::server::grpc::proto::AggregateRequest;
use clap::Args;
use tonic::transport::Channel;

#[derive(Args, Debug)]
pub struct AggregateArgs {
    /// Aggregator gRPC endpoint
    #[arg(short, long, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,

    /// Filter by agent ID
    #[arg(long)]
    pub agent_id: Option<String>,

    /// Start time (Unix epoch nanoseconds)
    #[arg(long)]
    pub start: Option<i64>,

    /// End time (Unix epoch nanoseconds)
    #[arg(long)]
    pub end: Option<i64>,

    /// Max batches to aggregate
    #[arg(short, long, default_value = "1000")]
    pub limit: u32,

    /// Event type: cpu, lock, syscall, or all
    #[arg(short = 't', long, default_value = "")]
    pub event_type: String,
}

pub async fn run(args: AggregateArgs) -> Result<()> {
    let mut client = AggregatorClient::<Channel>::connect(args.endpoint.clone())
        .await
        .context("Failed to connect to aggregator")?;

    let request = AggregateRequest {
        agent_id: args.agent_id.clone(),
        time_start_ns: args.start,
        time_end_ns: args.end,
        limit: args.limit,
        event_type: args.event_type.clone(),
    };

    let response = client
        .aggregate(tonic::Request::new(request))
        .await
        .context("Aggregate RPC failed")?;

    let res = response.into_inner();
    if !res.error.is_empty() {
        anyhow::bail!("Aggregator error: {}", res.error);
    }

    if res.total_events == 0 {
        println!("No events found in the specified range.");
        return Ok(());
    }

    println!("Aggregated {} events:", res.total_events);

    // Parse the JSON-safe result
    let result: aperture_aggregator::aggregate::AggregateResultJson =
        serde_json::from_str(&res.result_json).context("Failed to parse aggregate result")?;

    if let Some(cpu) = &result.cpu {
        println!("\n=== CPU Profile ===");
        println!("  Total samples: {}", cpu.total_samples);
        println!("  Unique stacks: {}", cpu.stacks.len());
        for sc in cpu.stacks.iter().take(10) {
            let label = sc
                .stack
                .frames
                .iter()
                .map(|f| {
                    f.function
                        .as_deref()
                        .unwrap_or(&format!("0x{:x}", f.ip))
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(";");
            println!("  [{:>5}] {}", sc.count, label);
        }
    }

    if let Some(lock) = &result.lock {
        println!("\n=== Lock Contention ===");
        println!("  Total events: {}", lock.total_events);
        println!("  Unique contentions: {}", lock.contentions.len());
        for c in lock.contentions.iter().take(10) {
            println!(
                "  lock=0x{:x} count={} total_wait={:.2}ms max_wait={:.2}ms",
                c.lock_addr,
                c.count,
                c.total_wait_ns as f64 / 1_000_000.0,
                c.max_wait_ns as f64 / 1_000_000.0
            );
        }
    }

    if let Some(syscall) = &result.syscall {
        println!("\n=== Syscall Profile ===");
        println!("  Total events: {}", syscall.total_events);
        let mut sorted: Vec<_> = syscall.syscalls.values().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        println!(
            "  {:>20} {:>8} {:>12} {:>12} {:>8}",
            "SYSCALL", "COUNT", "AVG (us)", "MAX (us)", "ERRORS"
        );
        for stats in sorted.iter().take(20) {
            let avg_us = if stats.count > 0 {
                stats.total_duration_ns as f64 / stats.count as f64 / 1000.0
            } else {
                0.0
            };
            let max_us = stats.max_duration_ns as f64 / 1000.0;
            println!(
                "  {:>20} {:>8} {:>12.1} {:>12.1} {:>8}",
                stats.name, stats.count, avg_us, max_us, stats.error_count
            );
        }
    }

    Ok(())
}
