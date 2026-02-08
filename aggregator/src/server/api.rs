//! REST API for the web UI.
//! Serves /api/aggregate, /api/diff, /api/batches with JSON and CORS.

use crate::aggregate;
use crate::buffer::InMemoryBuffer;
use crate::MAX_AGGREGATE_BATCH_LIMIT;
use crate::storage::BatchStore;
use aperture_shared::types::diff;
use aperture_shared::types::profile::{LockProfile, Profile, SyscallProfile};
use hyper::{body::to_bytes, Body, Request, Response, StatusCode};
use std::sync::Arc;
use std::time::Duration;

fn json_response(body: &str, status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body.to_string()))
        .expect("response build")
}

fn cors_preflight() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Max-Age", "86400")
        .body(Body::empty())
        .expect("response build")
}

fn add_cors_headers(mut res: Response<Body>) -> Response<Body> {
    let headers = res.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    res
}

#[derive(serde::Deserialize)]
struct AggregateRequest {
    agent_id: Option<String>,
    time_start_ns: Option<i64>,
    time_end_ns: Option<i64>,
    limit: Option<u32>,
    event_type: Option<String>,
}

#[derive(serde::Deserialize)]
struct DiffRequest {
    baseline_agent_id: Option<String>,
    baseline_start_ns: Option<i64>,
    baseline_end_ns: Option<i64>,
    comparison_agent_id: Option<String>,
    comparison_start_ns: Option<i64>,
    comparison_end_ns: Option<i64>,
    event_type: Option<String>,
    limit: Option<u32>,
}

pub async fn handle_api(
    req: Request<Body>,
    buffer: &InMemoryBuffer,
    store: Option<Arc<dyn BatchStore>>,
) -> Result<Response<Body>, hyper::Error> {
    if req.method() == hyper::Method::OPTIONS {
        return Ok(cors_preflight());
    }

    let (path, method) = (req.uri().path().to_string(), req.method().clone());

    if path == "/api/health" && method == hyper::Method::GET {
        let buf_len = buffer.len().unwrap_or(0);
        let utilization = buffer.utilization();
        let storage_enabled = store.is_some();
        let metrics_text = crate::metrics::encode_metrics();

        // Parse key metrics from prometheus text for the UI
        let parse_metric = |name: &str| -> f64 {
            for line in metrics_text.lines() {
                if line.starts_with(name) && !line.starts_with('#') {
                    if let Some(val) = line.split_whitespace().last() {
                        return val.parse::<f64>().unwrap_or(0.0);
                    }
                }
            }
            0.0
        };

        let body = serde_json::json!({
            "status": if utilization < 0.95 { "healthy" } else { "degraded" },
            "buffer_batches": buf_len,
            "buffer_utilization": utilization,
            "storage_enabled": storage_enabled,
            "push_total_ok": parse_metric("aperture_push_total{status=\"ok\"}"),
            "push_total_error": parse_metric("aperture_push_total{status=\"error\"}"),
            "push_events_total": parse_metric("aperture_push_events_total"),
            "clickhouse_flush_ok": parse_metric("aperture_clickhouse_flush_total{status=\"ok\"}"),
            "clickhouse_flush_error": parse_metric("aperture_clickhouse_flush_total{status=\"error\"}"),
            "clickhouse_pending_rows": parse_metric("aperture_clickhouse_pending_rows"),
        }).to_string();
        let res = add_cors_headers(json_response(&body, StatusCode::OK));
        return Ok(res);
    }

    if path == "/api/batches" && method == hyper::Method::GET {
        let mut agent_id = None::<String>;
        let mut limit = 100u32;
        if let Some(q) = req.uri().query() {
            for part in q.split('&') {
                if let Some((k, v)) = part.split_once('=') {
                    match k {
                        "agent_id" => agent_id = Some(v.to_string()),
                        "limit" => if let Ok(n) = v.parse::<u32>() { limit = n },
                        _ => {}
                    }
                }
            }
        }
        match buffer.query(agent_id.as_ref().map(|s| s.as_str()), limit) {
            Ok(batches) => {
                let list: Vec<serde_json::Value> = batches
                    .into_iter()
                    .map(|(agent_id, sequence, event_count, received_at_ns)| {
                        serde_json::json!({
                            "agent_id": agent_id,
                            "sequence": sequence,
                            "event_count": event_count,
                            "received_at_ns": received_at_ns,
                        })
                    })
                    .collect();
                let body = serde_json::json!({ "batches": list, "error": "" }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::OK));
                return Ok(res);
            }
            Err(e) => {
                let body = serde_json::json!({ "batches": [], "error": e }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::OK));
                return Ok(res);
            }
        }
    }

    if path == "/api/aggregate" && method == hyper::Method::POST {
        let body_bytes = to_bytes(req.into_body()).await.map_err(hyper::Error::from)?;
        let api_req: AggregateRequest = match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return Ok(add_cors_headers(json_response(
                    &serde_json::json!({ "error": e.to_string() }).to_string(),
                    StatusCode::BAD_REQUEST,
                )));
            }
        };
        let agent_filter = api_req.agent_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
        let limit = api_req.limit.unwrap_or(500).min(MAX_AGGREGATE_BATCH_LIMIT);

        // Try ClickHouse first (with timeout), fall back to in-memory buffer
        let payloads = if let Some(ref s) = store {
            let ch_future = s.fetch_payload_strings(agent_filter, api_req.time_start_ns, api_req.time_end_ns, limit);
            match tokio::time::timeout(Duration::from_secs(5), ch_future).await {
                Ok(Ok(p)) if !p.is_empty() => p,
                Ok(Ok(_)) => {
                    // ClickHouse returned empty — fall back to buffer
                    buffer.payload_strings(agent_filter, limit).unwrap_or_default()
                }
                Ok(Err(e)) => {
                    tracing::warn!("ClickHouse query failed, using buffer: {}", e);
                    buffer.payload_strings(agent_filter, limit).unwrap_or_default()
                }
                Err(_) => {
                    tracing::warn!("ClickHouse query timed out (5s), using buffer");
                    buffer.payload_strings(agent_filter, limit).unwrap_or_default()
                }
            }
        } else {
            buffer.payload_strings(agent_filter, limit).unwrap_or_default()
        };

        let out = match aggregate::aggregate_batches(&payloads) {
            Ok(o) => o,
            Err(e) => {
                let body = serde_json::json!({ "error": e.to_string() }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::INTERNAL_SERVER_ERROR));
                return Ok(res);
            }
        };
        let mut result = out.result;
        let event_type = api_req.event_type.as_deref().unwrap_or("");
        aggregate::filter_by_type(&mut result, event_type);
        let json = result.to_json();
        let mut body_value = serde_json::to_value(&json).unwrap_or_default();
        if let Some(obj) = body_value.as_object_mut() {
            obj.insert("skipped_batches".to_string(), serde_json::json!(out.skipped_batches));
        }
        let body = body_value.to_string();
        let res = add_cors_headers(json_response(&body, StatusCode::OK));
        return Ok(res);
    }

    if path == "/api/diff" && method == hyper::Method::POST {
        let store = match &store {
            Some(s) => s,
            None => {
                let body = serde_json::json!({
                    "result_json": "{}",
                    "error": "storage not configured (enable ClickHouse)"
                }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::SERVICE_UNAVAILABLE));
                return Ok(res);
            }
        };
        let body_bytes = to_bytes(req.into_body()).await.map_err(hyper::Error::from)?;
        let api_req: DiffRequest = match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return Ok(add_cors_headers(json_response(
                    &serde_json::json!({ "error": e.to_string() }).to_string(),
                    StatusCode::BAD_REQUEST,
                )));
            }
        };
        let limit = api_req.limit.unwrap_or(500).min(MAX_AGGREGATE_BATCH_LIMIT);
        let baseline_agent = api_req.baseline_agent_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
        let comparison_agent = api_req.comparison_agent_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
        let baseline_payloads = match store
            .fetch_payload_strings(
                baseline_agent,
                api_req.baseline_start_ns,
                api_req.baseline_end_ns,
                limit,
            )
            .await
        {
            Ok(p) => p,
            Err(e) => {
                let body = serde_json::json!({ "result_json": "", "error": format!("baseline: {}", e) }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::INTERNAL_SERVER_ERROR));
                return Ok(res);
            }
        };
        let comparison_payloads = match store
            .fetch_payload_strings(
                comparison_agent,
                api_req.comparison_start_ns,
                api_req.comparison_end_ns,
                limit,
            )
            .await
        {
            Ok(p) => p,
            Err(e) => {
                let body = serde_json::json!({ "result_json": "", "error": format!("comparison: {}", e) }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::INTERNAL_SERVER_ERROR));
                return Ok(res);
            }
        };
        let baseline_out = match aggregate::aggregate_batches(&baseline_payloads) {
            Ok(o) => o,
            Err(e) => {
                let body = serde_json::json!({ "result_json": "", "error": e.to_string() }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::INTERNAL_SERVER_ERROR));
                return Ok(res);
            }
        };
        let comparison_out = match aggregate::aggregate_batches(&comparison_payloads) {
            Ok(o) => o,
            Err(e) => {
                let body = serde_json::json!({ "result_json": "", "error": e.to_string() }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::INTERNAL_SERVER_ERROR));
                return Ok(res);
            }
        };
        let baseline = baseline_out.result;
        let comparison = comparison_out.result;
        let event_type = api_req.event_type.as_deref().unwrap_or("cpu");
        let result_json = match event_type {
            "cpu" => {
                let b = baseline.cpu.unwrap_or_else(|| Profile::new(0, 0, 0));
                let c = comparison.cpu.unwrap_or_else(|| Profile::new(0, 0, 0));
                let d = diff::diff_cpu(&b, &c);
                serde_json::to_string(&d).unwrap()
            }
            "lock" => {
                let b = baseline.lock.unwrap_or_else(|| LockProfile::new(0));
                let c = comparison.lock.unwrap_or_else(|| LockProfile::new(0));
                let d = diff::diff_lock(&b, &c);
                serde_json::to_string(&d).unwrap()
            }
            "syscall" => {
                let b = baseline.syscall.unwrap_or_else(|| SyscallProfile::new(0));
                let c = comparison.syscall.unwrap_or_else(|| SyscallProfile::new(0));
                let d = diff::diff_syscall(&b, &c);
                serde_json::to_string(&d).unwrap()
            }
            _ => {
                let body = serde_json::json!({ "result_json": "", "error": format!("event_type must be cpu, lock, or syscall, got {}", event_type) }).to_string();
                let res = add_cors_headers(json_response(&body, StatusCode::BAD_REQUEST));
                return Ok(res);
            }
        };
        let body = serde_json::json!({ "result_json": result_json, "error": "" }).to_string();
        let res = add_cors_headers(json_response(&body, StatusCode::OK));
        return Ok(res);
    }

    Ok(add_cors_headers(
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    ))
}
