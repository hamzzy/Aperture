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
    BatchInfo, PushRequest, PushResponse, QueryRequest, QueryResponse, QueryStorageRequest,
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

    /// Enable Phase 6 persistent storage (e.g. ClickHouse).
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
}
