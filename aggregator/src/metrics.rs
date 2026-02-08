//! Prometheus metrics for the aggregator service (Phase 7)

use once_cell::sync::Lazy;
use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_histogram,
    Counter, CounterVec, Encoder, Gauge, Histogram, TextEncoder,
};

// ── Push RPC metrics ─────────────────────────────────────────────────────────

pub static PUSH_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("aperture_push_total", "Total push RPCs received", &["status"]).unwrap()
});

pub static PUSH_EVENTS_TOTAL: Lazy<Counter> = Lazy::new(|| {
    register_counter!("aperture_push_events_total", "Total events ingested via push").unwrap()
});

pub static PUSH_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aperture_push_duration_seconds",
        "Push RPC latency",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .unwrap()
});

// ── Buffer metrics ───────────────────────────────────────────────────────────

pub static BUFFER_SIZE: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aperture_buffer_batches",
        "Current number of batches in the in-memory buffer"
    )
    .unwrap()
});

pub static BUFFER_DROPS: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aperture_buffer_drops_total",
        "Batches dropped from buffer due to capacity"
    )
    .unwrap()
});

// ── ClickHouse metrics ───────────────────────────────────────────────────────

pub static CH_FLUSH_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "aperture_clickhouse_flush_total",
        "ClickHouse flush attempts",
        &["status"]
    )
    .unwrap()
});

pub static CH_FLUSH_ROWS: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aperture_clickhouse_flush_rows_total",
        "Rows flushed to ClickHouse"
    )
    .unwrap()
});

pub static CH_FLUSH_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aperture_clickhouse_flush_duration_seconds",
        "ClickHouse flush latency",
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .unwrap()
});

pub static CH_PENDING_ROWS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aperture_clickhouse_pending_rows",
        "Rows currently pending flush to ClickHouse"
    )
    .unwrap()
});

/// Render all registered metrics to Prometheus text format.
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
