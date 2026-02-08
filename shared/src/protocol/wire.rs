//! Wire protocol implementation for agent-aggregator communication.
//!
//! Uses bincode for efficient serialization. Cap'n Proto can be added later for zero-copy.

use crate::types::events::ProfileEvent;
use anyhow::Result;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Wire message envelope
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    /// Serialize message to bytes (bincode)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize message from bytes (bincode), validating the protocol version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let msg: Self = bincode::deserialize(bytes)?;
        if msg.version != PROTOCOL_VERSION {
            anyhow::bail!(
                "protocol version mismatch: expected {}, got {}",
                PROTOCOL_VERSION,
                msg.version
            );
        }
        Ok(msg)
    }
}
