//! gRPC service implementation (Phase 5)

use crate::buffer::InMemoryBuffer;
use crate::metrics;
use crate::storage::BatchStore;
use aperture_shared::protocol::wire::Message;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("aperture.aggregator.v1");
}

use proto::{
    aggregator_server::{Aggregator, AggregatorServer},
    AggregateRequest, AggregateResponse, BatchInfo, DiffRequest, DiffResponse, PushRequest,
    PushResponse, QueryRequest, QueryResponse, QueryStorageRequest,
};

// Re-export for main to use with_interceptor
pub use proto::aggregator_server::AggregatorServer as GrpcAggregatorServer;

/// gRPC server state
pub struct AggregatorService {
    buffer: Arc<InMemoryBuffer>,
    batch_store: Option<Arc<dyn BatchStore>>,
    auth_token: Option<std::sync::Arc<str>>,
}

impl AggregatorService {
    pub fn new(buffer: Arc<InMemoryBuffer>) -> Self {
        Self {
            buffer,
            batch_store: None,
            auth_token: None,
        }
    }

    pub fn with_batch_store(mut self, store: Arc<dyn BatchStore>) -> Self {
        self.batch_store = Some(store);
        self
    }

    pub fn with_auth_token(mut self, token: Option<String>) -> Self {
        self.auth_token = token.map(|s| s.into());
        self
    }

    pub fn into_server(self) -> AggregatorServer<Self> {
        AggregatorServer::new(self)
    }

    fn check_auth<T>(&self, request: &Request<T>) -> Result<(), Status> {
        let Some(ref expected) = self.auth_token else {
            return Ok(());
        };
        match request.metadata().get("authorization") {
            Some(val) => {
                let val_str = val.to_str().map_err(|_| {
                    crate::audit::grpc_auth_failure("invalid authorization header encoding");
                    Status::unauthenticated("Invalid authorization header encoding")
                })?;
                let token = val_str.strip_prefix("Bearer ").ok_or_else(|| {
                    crate::audit::grpc_auth_failure("missing Bearer prefix");
                    Status::unauthenticated("Missing Bearer prefix")
                })?;
                if token == expected.as_ref() {
                    crate::audit::grpc_auth_success();
                    Ok(())
                } else {
                    crate::audit::grpc_auth_failure("invalid token");
                    Err(Status::unauthenticated("Invalid token"))
                }
            }
            None => {
                crate::audit::grpc_auth_failure("missing authorization header");
                Err(Status::unauthenticated("Missing authorization header"))
            }
        }
    }
}

#[tonic::async_trait]
impl Aggregator for AggregatorService {
    async fn push(&self, request: Request<PushRequest>) -> Result<Response<PushResponse>, Status> {
        self.check_auth(&request)?;
        let start = Instant::now();
        let req = request.into_inner();
        let agent_id = if req.agent_id.is_empty() {
            "unknown".to_string()
        } else {
            req.agent_id
        };

        let msg_res = Message::from_bytes(&req.payload);
        let event_count = match &msg_res {
            Ok(m) => m.events.len() as u32,
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent_id,
                    error = %e,
                    "Failed to decode push payload (schema mismatch?)"
                );
                0
            }
        };

        let payload = req.payload;
        match self
            .buffer
            .push(agent_id.clone(), req.sequence, event_count, payload.clone())
        {
            Ok(()) => {}
            Err(e) => {
                metrics::PUSH_TOTAL.with_label_values(&["error"]).inc();
                metrics::PUSH_DURATION.observe(start.elapsed().as_secs_f64());
                return Ok(Response::new(PushResponse {
                    ok: false,
                    error: e,
                    backpressure: false,
                }));
            }
        }

        if let Some(store) = &self.batch_store {
            let received_at_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64;
            if let Err(e) = store
                .write_batch(
                    &agent_id,
                    req.sequence,
                    received_at_ns,
                    event_count,
                    &payload,
                )
                .await
            {
                tracing::warn!("Batch store write failed: {}", e);
            }
        }

        metrics::PUSH_TOTAL.with_label_values(&["ok"]).inc();
        metrics::PUSH_EVENTS_TOTAL.inc_by(event_count as f64);
        metrics::PUSH_DURATION.observe(start.elapsed().as_secs_f64());

        let backpressure = self.buffer.utilization() > 0.8;

        Ok(Response::new(PushResponse {
            ok: true,
            error: String::new(),
            backpressure,
        }))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let agent_filter = req
            .agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let limit = if req.limit == 0 { 100 } else { req.limit };

        match self.buffer.query(agent_filter, limit) {
            Ok(batches) => {
                let batches = batches
                    .into_iter()
                    .map(|(agent_id, sequence, event_count, received_at_ns)| BatchInfo {
                        agent_id,
                        sequence,
                        event_count,
                        received_at_ns,
                    })
                    .collect();
                Ok(Response::new(QueryResponse {
                    batches,
                    error: String::new(),
                }))
            }
            Err(e) => Ok(Response::new(QueryResponse {
                batches: vec![],
                error: e,
            })),
        }
    }

    async fn query_storage(
        &self,
        request: Request<QueryStorageRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let agent_filter = req
            .agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let time_start = req.time_start_ns;
        let time_end = req.time_end_ns;
        let limit = if req.limit == 0 { 100 } else { req.limit };

        let batches = match &self.batch_store {
            Some(store) => store
                .query_batches(agent_filter, time_start, time_end, limit)
                .await
                .map_err(Status::internal)?,
            None => {
                return Ok(Response::new(QueryResponse {
                    batches: vec![],
                    error: "storage not configured (enable ClickHouse)".to_string(),
                }))
            }
        };

        let batches = batches
            .into_iter()
            .map(|(agent_id, sequence, event_count, received_at_ns)| BatchInfo {
                agent_id,
                sequence,
                event_count: event_count as u64,
                received_at_ns,
            })
            .collect();
        Ok(Response::new(QueryResponse {
            batches,
            error: String::new(),
        }))
    }

    async fn aggregate(
        &self,
        request: Request<AggregateRequest>,
    ) -> Result<Response<AggregateResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let agent_filter = req
            .agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let limit = (if req.limit == 0 { 500 } else { req.limit })
            .min(crate::MAX_AGGREGATE_BATCH_LIMIT);

        let payloads = match &self.batch_store {
            Some(store) => store
                .fetch_payload_strings(agent_filter, req.time_start_ns, req.time_end_ns, limit)
                .await
                .map_err(Status::internal)?,
            None => {
                return Ok(Response::new(AggregateResponse {
                    result_json: String::new(),
                    total_events: 0,
                    error: "storage not configured (enable ClickHouse)".to_string(),
                }))
            }
        };

        let out = match crate::aggregate::aggregate_batches(&payloads) {
            Ok(o) => o,
            Err(e) => {
                return Ok(Response::new(AggregateResponse {
                    result_json: String::new(),
                    total_events: 0,
                    error: format!("aggregation failed: {}", e),
                }))
            }
        };

        let mut result = out.result;
        crate::aggregate::filter_by_type(&mut result, &req.event_type);

        let json_view = result.to_json();
        let json =
            serde_json::to_string(&json_view).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(AggregateResponse {
            total_events: result.total_events,
            result_json: json,
            error: if out.skipped_batches > 0 {
                format!("{} batches skipped (invalid/corrupt data)", out.skipped_batches)
            } else {
                String::new()
            },
        }))
    }

    async fn diff(
        &self,
        request: Request<DiffRequest>,
    ) -> Result<Response<DiffResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let limit = (if req.limit == 0 { 500 } else { req.limit })
            .min(crate::MAX_AGGREGATE_BATCH_LIMIT);

        let store = match &self.batch_store {
            Some(s) => s,
            None => {
                return Ok(Response::new(DiffResponse {
                    result_json: String::new(),
                    error: "storage not configured (enable ClickHouse)".to_string(),
                }))
            }
        };

        let baseline_agent = req
            .baseline_agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let comparison_agent = req
            .comparison_agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });

        // Fetch + aggregate baseline
        let baseline_payloads = store
            .fetch_payload_strings(baseline_agent, req.baseline_start_ns, req.baseline_end_ns, limit)
            .await
            .map_err(Status::internal)?;
        let baseline_out = crate::aggregate::aggregate_batches(&baseline_payloads)
            .map_err(|e| Status::internal(format!("baseline aggregation: {}", e)))?;
        let baseline = baseline_out.result;

        // Fetch + aggregate comparison
        let comparison_payloads = store
            .fetch_payload_strings(
                comparison_agent,
                req.comparison_start_ns,
                req.comparison_end_ns,
                limit,
            )
            .await
            .map_err(Status::internal)?;
        let comparison_out = crate::aggregate::aggregate_batches(&comparison_payloads)
            .map_err(|e| Status::internal(format!("comparison aggregation: {}", e)))?;
        let comparison = comparison_out.result;

        use aperture_shared::types::diff;
        use aperture_shared::types::profile::{LockProfile, Profile, SyscallProfile};

        let json = match req.event_type.as_str() {
            "cpu" => {
                let b = baseline.cpu.unwrap_or_else(|| Profile::new(0, 0, 0));
                let c = comparison.cpu.unwrap_or_else(|| Profile::new(0, 0, 0));
                let d = diff::diff_cpu(&b, &c);
                serde_json::to_string(&d)
            }
            "lock" => {
                let b = baseline.lock.unwrap_or_else(|| LockProfile::new(0));
                let c = comparison.lock.unwrap_or_else(|| LockProfile::new(0));
                let d = diff::diff_lock(&b, &c);
                serde_json::to_string(&d)
            }
            "syscall" => {
                let b = baseline.syscall.unwrap_or_else(|| SyscallProfile::new(0));
                let c = comparison.syscall.unwrap_or_else(|| SyscallProfile::new(0));
                let d = diff::diff_syscall(&b, &c);
                serde_json::to_string(&d)
            }
            other => {
                return Ok(Response::new(DiffResponse {
                    result_json: String::new(),
                    error: format!(
                        "event_type must be 'cpu', 'lock', or 'syscall', got '{}'",
                        other
                    ),
                }))
            }
        }
        .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DiffResponse {
            result_json: json,
            error: String::new(),
        }))
    }
}
