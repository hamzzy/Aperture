//! ClickHouse storage backend (Phase 6)
//!
//! Persists profile batches for time-range queries and aggregation.

//! ClickHouse storage backend (Phase 6)
//!
//! Persists profile batches for time-range queries and aggregation.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};

const TABLE_NAME: &str = "aperture_batches";
const DEFAULT_TABLE_ENGINE: &str = "MergeTree() ORDER BY (received_at_ms, agent_id, sequence)";

/// One row in the batches table (matches ClickHouse schema).
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct BatchRow {
    pub agent_id: String,
    pub sequence: u64,
    /// Milliseconds since Unix epoch (ClickHouse DateTime64(3)).
    pub received_at_ms: i64,
    pub event_count: u32,
    /// Payload stored as base64 (bincode Message bytes).
    pub payload: String,
}

/// ClickHouse-backed persistent store for profile batches.
pub struct ClickHouseStore {
    client: Client,
    table: String,
}

impl ClickHouseStore {
    /// Connect and ensure the table exists.
    pub async fn new(endpoint: &str, database: &str) -> Result<Self> {
        let client = Client::default()
            .with_url(endpoint)
            .with_database(database)
            .with_option("connect_timeout", "10")
            .with_option("request_timeout", "30");

        let store = Self {
            client: client.clone(),
            table: TABLE_NAME.to_string(),
        };

        store.ensure_table().await?;
        Ok(store)
    }

    async fn ensure_table(&self) -> Result<()> {
        let ddl = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                agent_id String,
                sequence UInt64,
                received_at_ms Int64,
                event_count UInt32,
                payload String
            ) ENGINE = {}",
            self.table, DEFAULT_TABLE_ENGINE
        );
        self.client
            .query(&ddl)
            .execute()
            .await
            .context("Create ClickHouse table")?;
        Ok(())
    }

    /// Insert a single batch (caller can batch multiple rows for efficiency).
    pub async fn insert_batch(
        &self,
        agent_id: &str,
        sequence: u64,
        received_at_ns: i64,
        event_count: u32,
        payload: &[u8],
    ) -> Result<()> {
        let received_at_ms = received_at_ns / 1_000_000;
        let payload_b64 = BASE64.encode(payload);
        let row = BatchRow {
            agent_id: agent_id.to_string(),
            sequence,
            received_at_ms,
            event_count,
            payload: payload_b64,
        };

        let mut insert = self.client.insert(&self.table).context("ClickHouse insert")?;
        insert.write(&row).await.context("Write batch row")?;
        insert.end().await.context("Flush insert")?;
        Ok(())
    }

    /// Query batches by optional agent and time range (nanoseconds since epoch).
    /// Returns (agent_id, sequence, event_count, received_at_ns).
    pub async fn fetch_batches(
        &self,
        agent_id_filter: Option<&str>,
        time_start_ns: Option<i64>,
        time_end_ns: Option<i64>,
        limit: u32,
    ) -> Result<Vec<(String, u64, u32, i64)>> {
        let limit = limit.min(10_000);
        let mut sql = format!(
            "SELECT agent_id, sequence, event_count, received_at_ms FROM {} WHERE 1=1",
            self.table
        );
        if agent_id_filter.is_some() {
            sql += " AND agent_id = ?";
        }
        if time_start_ns.is_some() {
            sql += " AND received_at_ms >= ?";
        }
        if time_end_ns.is_some() {
            sql += " AND received_at_ms <= ?";
        }
        sql += " ORDER BY received_at_ms DESC LIMIT ?";

        let mut q = self.client.query(&sql);
        if let Some(id) = agent_id_filter {
            q = q.bind(id);
        }
        if let Some(ts) = time_start_ns {
            q = q.bind(ts / 1_000_000);
        }
        if let Some(te) = time_end_ns {
            q = q.bind(te / 1_000_000);
        }
        q = q.bind(limit);

        #[derive(Debug, Row, Serialize, Deserialize)]
        struct QueryRow {
            agent_id: String,
            sequence: u64,
            event_count: u32,
            received_at_ms: i64,
        }

        let mut cursor = q.fetch::<QueryRow>().context("Query batches")?;
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            out.push((
                row.agent_id,
                row.sequence,
                row.event_count,
                row.received_at_ms * 1_000_000,
            ));
        }
        Ok(out)
    }
}

#[async_trait::async_trait]
impl crate::storage::BatchStore for ClickHouseStore {
    async fn write_batch(
        &self,
        agent_id: &str,
        sequence: u64,
        received_at_ns: i64,
        event_count: u32,
        payload: &[u8],
    ) -> Result<(), String> {
        self.insert_batch(agent_id, sequence, received_at_ns, event_count, payload)
            .await
            .map_err(|e| e.to_string())
    }

    async fn query_batches(
        &self,
        agent_id: Option<&str>,
        time_start_ns: Option<i64>,
        time_end_ns: Option<i64>,
        limit: u32,
    ) -> Result<Vec<(String, u64, u32, i64)>, String> {
        self.fetch_batches(agent_id, time_start_ns, time_end_ns, limit)
            .await
            .map_err(|e| e.to_string())
    }
}
