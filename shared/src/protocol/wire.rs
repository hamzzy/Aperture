//! Wire protocol implementation
//!
//! TODO Phase 5: Implement Cap'n Proto-based wire protocol for agent-aggregator communication

use crate::types::events::ProfileEvent;
use anyhow::Result;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Wire message envelope
#[derive(Debug)]
pub struct Message {
    pub version: u32,
    pub sequence: u64,
    pub events: Vec<ProfileEvent>,
}

impl Message {
    /// Create a new message
    pub fn new(sequence: u64, events: Vec<ProfileEvent>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            sequence,
            events,
        }
    }

    /// Serialize message to bytes (placeholder)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        // TODO: Implement Cap'n Proto serialization
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize message from bytes (placeholder)
    pub fn from_bytes(_bytes: &[u8]) -> Result<Self> {
        // TODO: Implement Cap'n Proto deserialization
        todo!("Cap'n Proto deserialization not yet implemented")
    }
}
