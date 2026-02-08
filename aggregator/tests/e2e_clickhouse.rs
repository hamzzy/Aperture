//! E2E test: Push → Query (buffer) → QueryStorage (ClickHouse).
//!
//! Requires the aggregator running with ClickHouse. Run via:
//!   ./scripts/e2e-clickhouse.sh

use aperture_aggregator::server::grpc::proto::aggregator_client::AggregatorClient;
use aperture_aggregator::server::grpc::proto::{PushRequest, QueryRequest, QueryStorageRequest};
use aperture_shared::protocol::wire::Message;
use aperture_shared::types::events::{CpuSample, ProfileEvent};
use std::time::Duration;
use tonic::transport::Channel;

const GRPC_ENDPOINT: &str = "http://127.0.0.1:50051";

fn endpoint() -> String {
    std::env::var("APERTURE_GRPC_ENDPOINT").unwrap_or_else(|_| GRPC_ENDPOINT.to_string())
}

#[tokio::test]
#[ignore] // Run explicitly via: cargo test --test e2e_clickhouse -- --ignored --nocapture
async fn e2e_push_query_and_storage() {
    let endpoint = endpoint();
    let mut client = AggregatorClient::<Channel>::connect(endpoint.clone())
        .await
        .expect("connect to aggregator");

    let message = Message::new(
        1,
        vec![ProfileEvent::CpuSample(CpuSample {
            timestamp: 123_000_000_000,
            pid: 1000,
            tid: 1001,
            cpu_id: 0,
            user_stack: vec![0x400000, 0x400100],
            kernel_stack: vec![],
            comm: "e2e-test".to_string(),
            user_stack_symbols: vec![],
            kernel_stack_symbols: vec![],
        })],
    );
    let payload = message.to_bytes().expect("serialize message");

    let push_req = PushRequest {
        agent_id: "e2e-agent".to_string(),
        sequence: 1,
        payload,
    };
    let push_res = client
        .push(tonic::Request::new(push_req))
        .await
        .expect("push");
    assert!(push_res.into_inner().ok, "push should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let query_req = QueryRequest {
        agent_id: Some("e2e-agent".to_string()),
        limit: 10,
    };
    let query_res = client
        .query(tonic::Request::new(query_req))
        .await
        .expect("query");
    let query_inner = query_res.into_inner();
    assert!(
        query_inner.error.is_empty(),
        "query error: {}",
        query_inner.error
    );
    assert!(
        !query_inner.batches.is_empty(),
        "buffer should contain at least one batch"
    );
    assert_eq!(query_inner.batches[0].agent_id, "e2e-agent");
    assert_eq!(query_inner.batches[0].sequence, 1);
    assert_eq!(query_inner.batches[0].event_count, 1);

    let storage_req = QueryStorageRequest {
        agent_id: Some("e2e-agent".to_string()),
        time_start_ns: Some(0),
        time_end_ns: Some(i64::MAX),
        limit: 10,
    };
    let storage_res = client
        .query_storage(tonic::Request::new(storage_req))
        .await
        .expect("query_storage");
    let storage_inner = storage_res.into_inner();
    assert!(
        storage_inner.error.is_empty(),
        "query_storage error (is ClickHouse configured with correct password?): {}",
        storage_inner.error
    );
    assert!(
        !storage_inner.batches.is_empty(),
        "storage should contain at least one batch"
    );
    assert_eq!(storage_inner.batches[0].agent_id, "e2e-agent");
    assert_eq!(storage_inner.batches[0].sequence, 1);
}
