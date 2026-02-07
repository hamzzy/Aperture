//! In-memory buffer for ingested profile data (Phase 5)

use aperture_shared::protocol::wire::Message;
use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single batch as stored in the buffer
#[derive(Debug, Clone)]
pub struct StoredBatch {
    pub agent_id: String,
    pub sequence: u64,
    pub event_count: u64,
    pub received_at_ns: i64,
    pub payload: Vec<u8>,
}

/// In-memory ring buffer for agent pushes. Thread-safe.
#[derive(Debug)]
pub struct InMemoryBuffer {
    max_batches: usize,
    batches: RwLock<VecDeque<StoredBatch>>,
}

impl InMemoryBuffer {
    /// Create a buffer that keeps at most `max_batches` batches.
    pub fn new(max_batches: usize) -> Self {
        Self {
            max_batches,
            batches: RwLock::new(VecDeque::new()),
        }
    }

    /// Append a batch. Drops oldest if at capacity.
    pub fn push(&self, agent_id: String, sequence: u64, payload: Vec<u8>) -> Result<(), String> {
        let message = Message::from_bytes(&payload).map_err(|e| e.to_string())?;
        let event_count = message.events.len() as u64;
        let received_at_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let batch = StoredBatch {
            agent_id,
            sequence,
            event_count,
            received_at_ns,
            payload,
        };

        let mut batches = self.batches.write().map_err(|e| e.to_string())?;
        batches.push_back(batch);
        while batches.len() > self.max_batches {
            batches.pop_front();
        }
        Ok(())
    }

    /// Query recent batches, optionally filtered by agent_id. Returns at most `limit` batch infos.
    pub fn query(
        &self,
        agent_id_filter: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, u64, u64, i64)>, String> {
        let batches = self.batches.read().map_err(|e| e.to_string())?;
        let limit = limit.min(1000) as usize;
        let mut out = Vec::with_capacity(limit);

        for b in batches.iter().rev().take(limit * 2) {
            if out.len() >= limit {
                break;
            }
            if let Some(id) = agent_id_filter {
                if b.agent_id != id {
                    continue;
                }
            }
            out.push((b.agent_id.clone(), b.sequence, b.event_count, b.received_at_ns));
        }
        out.reverse();
        Ok(out)
    }
}
