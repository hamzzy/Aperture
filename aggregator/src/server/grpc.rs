//! gRPC service implementation (Phase 5)

use crate::buffer::InMemoryBuffer;
use crate::storage::BatchStore;
use aperture_shared::protocol::wire::Message;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("aperture.aggregator.v1");
}

use proto::{
    aggregator_server::{Aggregator, AggregatorServer},
    AggregateRequest, AggregateResponse, BatchInfo, DiffRequest, DiffResponse, PushRequest,
    PushResponse, QueryRequest, QueryResponse, QueryStorageRequest,
};

/// gRPC server state
pub struct AggregatorService {
    buffer: Arc<InMemoryBuffer>,
    batch_store: Option<Arc<dyn BatchStore>>,
}

impl AggregatorService {
    pub fn new(buffer: Arc<InMemoryBuffer>) -> Self {
        Self {
            buffer,
            batch_store: None,
        }
    }

    pub fn with_batch_store(mut self, store: Arc<dyn BatchStore>) -> Self {
        self.batch_store = Some(store);
        self
    }

    pub fn into_server(self) -> AggregatorServer<Self> {
        AggregatorServer::new(self)
    }
}

#[tonic::async_trait]
impl Aggregator for AggregatorService {
    async fn push(&self, request: Request<PushRequest>) -> Result<Response<PushResponse>, Status> {
        let req = request.into_inner();
        let agent_id = if req.agent_id.is_empty() {
            "unknown".to_string()
        } else {
            req.agent_id
        };

        match self.buffer.push(agent_id.clone(), req.sequence, req.payload.clone()) {
            Ok(()) => {}
            Err(e) => {
                return Ok(Response::new(PushResponse {
                    ok: false,
                    error: e,
                }))
            }
        }

        if let Some(store) = &self.batch_store {
            let event_count = Message::from_bytes(&req.payload)
                .map(|m| m.events.len() as u32)
                .unwrap_or(0);
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
                    &req.payload,
                )
                .await
            {
                tracing::warn!("Batch store write failed: {}", e);
            }
        }

        Ok(Response::new(PushResponse {
            ok: true,
            error: String::new(),
        }))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
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
        let req = request.into_inner();
        let agent_filter = req
            .agent_id
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let limit = if req.limit == 0 { 1000 } else { req.limit };

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

        let mut result = match crate::aggregate::aggregate_batches(&payloads) {
            Ok(r) => r,
            Err(e) => {
                return Ok(Response::new(AggregateResponse {
                    result_json: String::new(),
                    total_events: 0,
                    error: format!("aggregation failed: {}", e),
                }))
            }
        };

        crate::aggregate::filter_by_type(&mut result, &req.event_type);

        let json_view = result.to_json();
        let json =
            serde_json::to_string(&json_view).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(AggregateResponse {
            total_events: result.total_events,
            result_json: json,
            error: String::new(),
        }))
    }

    async fn diff(
        &self,
        request: Request<DiffRequest>,
    ) -> Result<Response<DiffResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 { 1000 } else { req.limit };

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
        let baseline = crate::aggregate::aggregate_batches(&baseline_payloads)
            .map_err(|e| Status::internal(format!("baseline aggregation: {}", e)))?;

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
        let comparison = crate::aggregate::aggregate_batches(&comparison_payloads)
            .map_err(|e| Status::internal(format!("comparison aggregation: {}", e)))?;

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
