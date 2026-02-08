//! Diff command implementation (Phase 6)

use anyhow::{Context, Result};
use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
use aperture_aggregator::server::grpc::proto::DiffRequest;
use clap::Args;
use tonic::transport::Channel;

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Aggregator gRPC endpoint
    #[arg(short, long, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,

    /// Event type to diff: cpu, lock, or syscall
    #[arg(short = 't', long)]
    pub event_type: String,

    /// Baseline agent ID (optional, defaults to all)
    #[arg(long)]
    pub baseline_agent: Option<String>,

    /// Baseline start time (Unix epoch nanoseconds)
    #[arg(long)]
    pub baseline_start: Option<i64>,

    /// Baseline end time (Unix epoch nanoseconds)
    #[arg(long)]
    pub baseline_end: Option<i64>,

    /// Comparison agent ID (optional, defaults to all)
    #[arg(long)]
    pub comparison_agent: Option<String>,

    /// Comparison start time (Unix epoch nanoseconds)
    #[arg(long)]
    pub comparison_start: Option<i64>,

    /// Comparison end time (Unix epoch nanoseconds)
    #[arg(long)]
    pub comparison_end: Option<i64>,

    /// Max batches per window
    #[arg(short, long, default_value = "1000")]
    pub limit: u32,
}

pub async fn run(args: DiffArgs) -> Result<()> {
    let mut client = AggregatorClient::<Channel>::connect(args.endpoint.clone())
        .await
        .context("Failed to connect to aggregator")?;

    let request = DiffRequest {
        baseline_agent_id: args.baseline_agent.clone(),
        baseline_start_ns: args.baseline_start,
        baseline_end_ns: args.baseline_end,
        comparison_agent_id: args.comparison_agent.clone(),
        comparison_start_ns: args.comparison_start,
        comparison_end_ns: args.comparison_end,
        event_type: args.event_type.clone(),
        limit: args.limit,
    };

    let response = client
        .diff(tonic::Request::new(request))
        .await
        .context("Diff RPC failed")?;

    let res = response.into_inner();
    if !res.error.is_empty() {
        anyhow::bail!("Aggregator error: {}", res.error);
    }

    if res.result_json.is_empty() {
        println!("No diff data returned.");
        return Ok(());
    }

    match args.event_type.as_str() {
        "cpu" => print_cpu_diff(&res.result_json)?,
        "lock" => print_lock_diff(&res.result_json)?,
        "syscall" => print_syscall_diff(&res.result_json)?,
        other => anyhow::bail!("Unknown event type: {}", other),
    }

    Ok(())
}

fn print_cpu_diff(json: &str) -> Result<()> {
    let diff: aperture_shared::types::diff::CpuDiff =
        serde_json::from_str(json).context("parse CpuDiff")?;

    println!("=== CPU Diff ===");
    println!(
        "  Baseline: {} samples | Comparison: {} samples",
        diff.baseline_total, diff.comparison_total
    );
    println!(
        "\n  {:>8} {:>8} {:>8} {:>7}  STACK",
        "BASE", "COMP", "DELTA", "%"
    );

    for s in diff.stacks.iter().take(20) {
        let label = s
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
        let sign = if s.delta >= 0 { "+" } else { "" };
        println!(
            "  {:>8} {:>8} {:>+8} {:>6.1}%  {}",
            s.baseline_count, s.comparison_count, s.delta, s.delta_pct, label
        );
        let _ = sign; // consumed via {:>+8} format
    }

    Ok(())
}

fn print_syscall_diff(json: &str) -> Result<()> {
    let diff: aperture_shared::types::diff::SyscallDiff =
        serde_json::from_str(json).context("parse SyscallDiff")?;

    println!("=== Syscall Diff ===");
    println!(
        "  Baseline: {} events | Comparison: {} events",
        diff.baseline_total, diff.comparison_total
    );
    println!(
        "\n  {:>20} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10}",
        "SYSCALL", "B.COUNT", "C.COUNT", "DELTA", "B.AVG(us)", "C.AVG(us)", "D.AVG(us)"
    );

    for s in diff.syscalls.iter().take(20) {
        println!(
            "  {:>20} {:>8} {:>8} {:>+8} {:>10.1} {:>10.1} {:>+10.1}",
            s.name,
            s.baseline_count,
            s.comparison_count,
            s.delta_count,
            s.baseline_avg_ns / 1000.0,
            s.comparison_avg_ns / 1000.0,
            s.delta_avg_ns / 1000.0
        );
    }

    Ok(())
}

fn print_lock_diff(json: &str) -> Result<()> {
    let diff: aperture_shared::types::diff::LockDiff =
        serde_json::from_str(json).context("parse LockDiff")?;

    println!("=== Lock Contention Diff ===");
    println!(
        "  Baseline: {} events | Comparison: {} events",
        diff.baseline_total, diff.comparison_total
    );
    println!(
        "\n  {:>18} {:>8} {:>8} {:>12}",
        "LOCK_ADDR", "B.COUNT", "C.COUNT", "DELTA_WAIT"
    );

    for c in diff.contentions.iter().take(20) {
        println!(
            "  0x{:016x} {:>8} {:>8} {:>+10.2}ms",
            c.lock_addr,
            c.baseline_count,
            c.comparison_count,
            c.delta_wait_ns as f64 / 1_000_000.0
        );
    }

    Ok(())
}
