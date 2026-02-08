//! Export module for the aggregator.
//!
//! Provides endpoints to export profiling data in standard formats:
//!
//! - **JSON** (`/api/export/json`) — full aggregate as downloadable JSON
//! - **Collapsed stacks** (`/api/export/collapsed`) — Brendan Gregg format,
//!   compatible with `flamegraph.pl`, `speedscope`, Grafana Pyroscope, etc.
//! - **Prometheus** (`/metrics`) — already handled in metrics.rs

use crate::aggregate;
use crate::buffer::InMemoryBuffer;
use crate::storage::BatchStore;
use hyper::{Body, Response, StatusCode};
use std::sync::Arc;

fn cors_headers(mut res: Response<Body>) -> Response<Body> {
    res.headers_mut()
        .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    res
}

/// Generate a JSON export of aggregated profiling data.
pub async fn export_json(
    buffer: &InMemoryBuffer,
    store: Option<&Arc<dyn BatchStore>>,
    event_type: Option<&str>,
    limit: u32,
) -> Response<Body> {
    let payloads = fetch_payloads(buffer, store, limit).await;

    let out = match aggregate::aggregate_batches(&payloads) {
        Ok(o) => o,
        Err(e) => {
            return cors_headers(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(r#"{{"error":"{}"}}"#, e)))
                    .unwrap(),
            );
        }
    };

    let mut result = out.result;
    if let Some(et) = event_type {
        aggregate::filter_by_type(&mut result, et);
    }
    let json = result.to_json();
    let body = serde_json::to_string_pretty(&json).unwrap_or_default();

    cors_headers(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header(
                "Content-Disposition",
                "attachment; filename=\"aperture-profile.json\"",
            )
            .body(Body::from(body))
            .unwrap(),
    )
}

/// Generate collapsed-stack format output (Brendan Gregg format).
///
/// Each line: `frame1;frame2;...;frameN count`
///
/// Compatible with:
/// - `flamegraph.pl` (original Brendan Gregg tool)
/// - speedscope (`speedscope collapsed.txt`)
/// - Grafana Pyroscope ingestion
/// - pprof conversion tools
pub async fn export_collapsed(
    buffer: &InMemoryBuffer,
    store: Option<&Arc<dyn BatchStore>>,
    limit: u32,
) -> Response<Body> {
    let payloads = fetch_payloads(buffer, store, limit).await;

    let out = match aggregate::aggregate_batches(&payloads) {
        Ok(o) => o,
        Err(e) => {
            return cors_headers(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Aggregation error: {}", e)))
                    .unwrap(),
            );
        }
    };

    let result = out.result;
    let profile = match result.cpu {
        Some(p) => p,
        None => {
            return cors_headers(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("No CPU profile data available"))
                    .unwrap(),
            );
        }
    };

    let mut collapsed = String::new();
    for (stack, count) in &profile.samples {
        let frames: Vec<String> = stack
            .frames
            .iter()
            .rev()
            .map(|f| {
                f.function
                    .clone()
                    .unwrap_or_else(|| format!("0x{:x}", f.ip))
            })
            .collect();
        if !frames.is_empty() {
            collapsed.push_str(&frames.join(";"));
            collapsed.push(' ');
            collapsed.push_str(&count.to_string());
            collapsed.push('\n');
        }
    }

    cors_headers(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header(
                "Content-Disposition",
                "attachment; filename=\"aperture-collapsed.txt\"",
            )
            .body(Body::from(collapsed))
            .unwrap(),
    )
}

/// Fetch payload strings from storage or buffer.
async fn fetch_payloads(
    buffer: &InMemoryBuffer,
    store: Option<&Arc<dyn BatchStore>>,
    limit: u32,
) -> Vec<String> {
    if let Some(s) = store {
        match s.fetch_payload_strings(None, None, None, limit).await {
            Ok(p) if !p.is_empty() => p,
            _ => buffer.payload_strings(None, limit).unwrap_or_default(),
        }
    } else {
        buffer.payload_strings(None, limit).unwrap_or_default()
    }
}
