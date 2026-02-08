//! In-memory buffer for ingested profile data

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
    /// Pre-allocates capacity to avoid reallocations as the buffer fills (memory efficiency).
    pub fn new(max_batches: usize) -> Self {
        Self {
            max_batches,
            batches: RwLock::new(VecDeque::with_capacity(max_batches.min(4096))),
        }
    }

    /// Append a batch. Drops oldest if at capacity.
    /// If the payload fails to decode (schema/version mismatch), we still store it with event_count 0
    /// so data is not lost and the agent can keep pushing; aggregation will skip invalid payloads.
    pub fn push(&self, agent_id: String, sequence: u64, payload: Vec<u8>) -> Result<(), String> {
        let event_count = Message::from_bytes(&payload)
            .map(|m| m.events.len() as u64)
            .unwrap_or_else(|e| {
                tracing::warn!("Push payload decode failed (schema/version mismatch?): {} — storing anyway with event_count 0", e);
                0
            });
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
        let mut drops = 0u64;
        while batches.len() > self.max_batches {
            batches.pop_front();
            drops += 1;
        }

        crate::metrics::BUFFER_SIZE.set(batches.len() as f64);
        if drops > 0 {
            crate::metrics::BUFFER_DROPS.inc_by(drops as f64);
        }

        Ok(())
    }

    pub fn query(
        &self,
        agent_id_filter: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, u64, u64, i64)>, String> {
        let batches = self.batches.read().map_err(|e| e.to_string())?;
        let limit = limit.min(1000) as usize;
        let mut out = Vec::with_capacity(limit);

        for b in batches.iter().rev() {
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

    /// Current number of batches in the buffer.
    pub fn len(&self) -> Result<usize, String> {
        let batches = self.batches.read().map_err(|e| e.to_string())?;
        Ok(batches.len())
    }

    /// Buffer utilization as a fraction (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        let len = self.len().unwrap_or(0);
        if self.max_batches == 0 {
            return 0.0;
        }
        len as f64 / self.max_batches as f64
    }
}
