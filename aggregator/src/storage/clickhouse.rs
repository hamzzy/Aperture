//! ClickHouse storage backend
//!
//! Persists profile batches for time-range queries and aggregation.
//! Inserts are buffered in memory and flushed periodically for throughput.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const TABLE_NAME: &str = "aperture_batches";

/// Convert a timestamp that may be in nanoseconds or milliseconds to milliseconds.
/// Values >= 1e15 are assumed to be nanoseconds and are divided by 1_000_000.
/// Values < 1e15 are assumed to already be milliseconds.
fn to_millis(ts: i64) -> i64 {
    if ts >= 1_000_000_000_000_000 {
        ts / 1_000_000
    } else {
        ts
    }
}

const DEFAULT_TABLE_ENGINE: &str = "\
MergeTree() \
PARTITION BY toYYYYMM(fromUnixTimestamp64Milli(received_at_ms)) \
ORDER BY (agent_id, received_at_ms, sequence) \
TTL toDateTime(fromUnixTimestamp64Milli(received_at_ms)) + INTERVAL 90 DAY \
SETTINGS index_granularity = 8192";
/// Flush when this many rows are buffered.
const FLUSH_THRESHOLD: usize = 100;
/// Flush at least this often.
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

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
/// Inserts are buffered and flushed in bulk.
pub struct ClickHouseStore {
    client: Client,
    table: String,
    pending: Arc<AsyncMutex<Vec<BatchRow>>>,
    cancel: CancellationToken,
    flush_handle: Mutex<Option<JoinHandle<()>>>,
    /// Notifies the background flush task to wake early when threshold is reached.
    flush_notify: Arc<tokio::sync::Notify>,
}

impl ClickHouseStore {

    pub async fn new(endpoint: &str, database: &str) -> Result<Self> {
        let mut client = Client::default()
            .with_url(endpoint)
            .with_database(database)
            .with_option("connect_timeout", "10")
            .with_option("receive_timeout", "30");
        if let Ok(password) = std::env::var("APERTURE_CLICKHOUSE_PASSWORD") {
            client = client.with_user("default").with_password(password);
        }

        let store = Self {
            client,
            table: TABLE_NAME.to_string(),
            pending: Arc::new(AsyncMutex::new(Vec::new())),
            cancel: CancellationToken::new(),
            flush_handle: Mutex::new(None),
            flush_notify: Arc::new(tokio::sync::Notify::new()),
        };

        store.ensure_table().await?;
        store.spawn_flush_task();
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

    /// Enqueue a row for batched insertion.
    pub async fn enqueue_batch(
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

        let mut pending = self.pending.lock().await;
        pending.push(row);
        let len = pending.len();
        crate::metrics::CH_PENDING_ROWS.set(len as f64);
        drop(pending);

        // Wake the background flush task early when threshold is reached.
        // This avoids blocking the push RPC on a synchronous ClickHouse insert.
        if len >= FLUSH_THRESHOLD {
            self.flush_notify.notify_one();
        }
        Ok(())
    }

    /// Flush all pending rows to ClickHouse in a single insert.
    pub async fn flush(&self) -> Result<()> {
        let rows = {
            let mut pending = self.pending.lock().await;
            if pending.is_empty() {
                return Ok(());
            }
            std::mem::take(&mut *pending)
        };
        crate::metrics::CH_PENDING_ROWS.set(0.0);

        let count = rows.len();
        let start = Instant::now();
        match Self::flush_rows(&self.client, &self.table, &rows).await {
            Ok(()) => {
                crate::metrics::CH_FLUSH_TOTAL.with_label_values(&["ok"]).inc();
                crate::metrics::CH_FLUSH_ROWS.inc_by(count as f64);
                crate::metrics::CH_FLUSH_DURATION.observe(start.elapsed().as_secs_f64());
                tracing::debug!("Flushed {} rows to ClickHouse", count);
                Ok(())
            }
            Err(e) => {
                crate::metrics::CH_FLUSH_TOTAL.with_label_values(&["error"]).inc();
                Err(e)
            }
        }
    }

    /// Spawn a background task that flushes pending rows on a timer or when
    /// notified (threshold reached). On cancel (shutdown), performs one final flush.
    fn spawn_flush_task(&self) {
        let pending = self.pending.clone();
        let client = self.client.clone();
        let table = self.table.clone();
        let cancel = self.cancel.clone();
        let notify = self.flush_notify.clone();

        /// Drain pending rows and flush to ClickHouse. Re-queues on error.
        async fn do_flush(
            pending: &AsyncMutex<Vec<BatchRow>>,
            client: &Client,
            table: &str,
            label: &str,
        ) {
            let rows = {
                let mut p = pending.lock().await;
                if p.is_empty() {
                    return;
                }
                std::mem::take(&mut *p)
            };
            crate::metrics::CH_PENDING_ROWS.set(0.0);
            let count = rows.len();
            let start = Instant::now();
            match ClickHouseStore::flush_rows(client, table, &rows).await {
                Ok(()) => {
                    crate::metrics::CH_FLUSH_TOTAL.with_label_values(&["ok"]).inc();
                    crate::metrics::CH_FLUSH_ROWS.inc_by(count as f64);
                    crate::metrics::CH_FLUSH_DURATION.observe(start.elapsed().as_secs_f64());
                    tracing::debug!("{}: {} rows to ClickHouse", label, count);
                }
                Err(e) => {
                    crate::metrics::CH_FLUSH_TOTAL.with_label_values(&["error"]).inc();
                    tracing::warn!("{} failed ({} rows), re-queuing: {:#}", label, count, e);
                    let mut p = pending.lock().await;
                    p.extend(rows);
                    crate::metrics::CH_PENDING_ROWS.set(p.len() as f64);
                }
            }
        }

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(FLUSH_INTERVAL);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        do_flush(&pending, &client, &table, "Timer flush").await;
                    }
                    _ = notify.notified() => {
                        do_flush(&pending, &client, &table, "Threshold flush").await;
                    }
                    _ = cancel.cancelled() => {
                        do_flush(&pending, &client, &table, "Shutdown flush").await;
                        break;
                    }
                }
            }
        });
        if let Ok(mut guard) = self.flush_handle.lock() {
            *guard = Some(handle);
        }
    }

    /// Gracefully shut down: cancel the flush task, await it, then do one final flush.
    pub async fn shutdown(&self) -> Result<(), String> {
        self.cancel.cancel();
        let handle = {
            let mut guard = self.flush_handle.lock().map_err(|e| e.to_string())?;
            guard.take()
        };
        if let Some(h) = handle {
            let _ = h.await;
        }
        self.flush().await.map_err(|e| e.to_string())
    }

    async fn flush_rows(client: &Client, table: &str, rows: &[BatchRow]) -> Result<()> {
        let mut insert = client.insert(table).context("ClickHouse insert")?;
        for row in rows {
            insert.write(row).await.context("Write batch row")?;
        }
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
        // Flush pending rows first so queries see recent data.
        let _ = self.flush().await;

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
            q = q.bind(to_millis(ts));
        }
        if let Some(te) = time_end_ns {
            q = q.bind(to_millis(te));
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

    /// Fetch raw base64 payloads for aggregation. Same filtering as fetch_batches.
    pub async fn fetch_payloads(
        &self,
        agent_id_filter: Option<&str>,
        time_start_ns: Option<i64>,
        time_end_ns: Option<i64>,
        limit: u32,
    ) -> Result<Vec<String>> {
        let _ = self.flush().await;

        let limit = limit.min(10_000);
        let mut sql = format!(
            "SELECT payload FROM {} WHERE 1=1",
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
        sql += " ORDER BY received_at_ms ASC LIMIT ?";

        let mut q = self.client.query(&sql);
        if let Some(id) = agent_id_filter {
            q = q.bind(id);
        }
        if let Some(ts) = time_start_ns {
            q = q.bind(to_millis(ts));
        }
        if let Some(te) = time_end_ns {
            q = q.bind(to_millis(te));
        }
        q = q.bind(limit);

        #[derive(Debug, Row, Serialize, Deserialize)]
        struct PayloadRow {
            payload: String,
        }

        let mut cursor = q.fetch::<PayloadRow>().context("Query payloads")?;
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            out.push(row.payload);
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
        self.enqueue_batch(agent_id, sequence, received_at_ns, event_count, payload)
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

    async fn fetch_payload_strings(
        &self,
        agent_id: Option<&str>,
        time_start_ns: Option<i64>,
        time_end_ns: Option<i64>,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        self.fetch_payloads(agent_id, time_start_ns, time_end_ns, limit)
            .await
            .map_err(|e| e.to_string())
    }

    async fn shutdown(&self) -> Result<(), String> {
        ClickHouseStore::shutdown(self).await
    }
}
