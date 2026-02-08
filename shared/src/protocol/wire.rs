//! Wire protocol implementation for agent-aggregator communication.
//!
//! Uses bincode with an explicit config so agent and aggregator always use the same
//! encoding (fixint for lengths and enums), avoiding version/skew mismatches.
//!
//! # Schema evolution
//!
//! Bincode is positional (not self-describing), so adding fields to event structs
//! breaks decoding of old payloads. We handle this via `LegacyMessage` types that
//! mirror the original (pre-symbol) struct shapes. When `from_bytes` fails with the
//! current schema it tries `LegacyMessage`, then converts to the current types with
//! the new fields defaulted.

use crate::types::events::{
    CpuId, CpuSample, GpuKernelEvent, LockEvent, Pid, ProfileEvent, StackTrace, SyscallEvent,
    Tid, Timestamp,
};
use anyhow::Result;
use bincode::Options;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Single bincode config for wire format: fixint encoding so vec lengths and enum tags
/// have a fixed size and cannot be misinterpreted across builds or bincode versions.
fn wire_bincode() -> impl bincode::config::Options {
    bincode::config::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}

// ---------------------------------------------------------------------------
// Legacy types (v1 schema before symbol fields were added)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyCpuSample {
    pub timestamp: Timestamp,
    pub pid: Pid,
    pub tid: Tid,
    pub cpu_id: CpuId,
    pub user_stack: StackTrace,
    pub kernel_stack: StackTrace,
    pub comm: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyLockEvent {
    pub timestamp: Timestamp,
    pub pid: Pid,
    pub tid: Tid,
    pub lock_addr: u64,
    pub hold_time_ns: u64,
    pub wait_time_ns: u64,
    pub stack_trace: StackTrace,
    pub comm: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum LegacyProfileEvent {
    CpuSample(LegacyCpuSample),
    Lock(LegacyLockEvent),
    Syscall(SyscallEvent),
    GpuKernel(GpuKernelEvent),
}

impl LegacyProfileEvent {
    fn into_current(self) -> ProfileEvent {
        match self {
            LegacyProfileEvent::CpuSample(s) => ProfileEvent::CpuSample(CpuSample {
                timestamp: s.timestamp,
                pid: s.pid,
                tid: s.tid,
                cpu_id: s.cpu_id,
                user_stack: s.user_stack,
                kernel_stack: s.kernel_stack,
                comm: s.comm,
                user_stack_symbols: vec![],
                kernel_stack_symbols: vec![],
            }),
            LegacyProfileEvent::Lock(e) => ProfileEvent::Lock(LockEvent {
                timestamp: e.timestamp,
                pid: e.pid,
                tid: e.tid,
                lock_addr: e.lock_addr,
                hold_time_ns: e.hold_time_ns,
                wait_time_ns: e.wait_time_ns,
                stack_trace: e.stack_trace,
                comm: e.comm,
                stack_symbols: vec![],
            }),
            LegacyProfileEvent::Syscall(e) => ProfileEvent::Syscall(e),
            LegacyProfileEvent::GpuKernel(e) => ProfileEvent::GpuKernel(e),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyMessage {
    pub version: u32,
    pub sequence: u64,
    pub events: Vec<LegacyProfileEvent>,
}

impl LegacyMessage {
    fn into_current(self) -> Message {
        Message {
            version: self.version,
            sequence: self.sequence,
            events: self.events.into_iter().map(|e| e.into_current()).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Current message type
// ---------------------------------------------------------------------------

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

    /// Serialize message to bytes (bincode, fixint encoding).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        wire_bincode().serialize(self).map_err(Into::into)
    }

    /// Deserialize message from bytes (bincode), validating the protocol version.
    ///
    /// Attempts decoding in order:
    /// 1. Current schema, fixint encoding
    /// 2. Current schema, legacy varint encoding
    /// 3. Legacy schema (no symbol fields), fixint encoding
    /// 4. Legacy schema, legacy varint encoding
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // 1. Current schema, fixint
        if let Ok(msg) = wire_bincode().deserialize::<Self>(bytes) {
            if msg.version == PROTOCOL_VERSION {
                return Ok(msg);
            }
        }
        // 2. Current schema, legacy varint
        if let Ok(msg) = bincode::deserialize::<Self>(bytes) {
            if msg.version == PROTOCOL_VERSION {
                return Ok(msg);
            }
        }
        // 3. Legacy schema (pre-symbol fields), fixint
        if let Ok(msg) = wire_bincode().deserialize::<LegacyMessage>(bytes) {
            if msg.version == PROTOCOL_VERSION {
                return Ok(msg.into_current());
            }
        }
        // 4. Legacy schema, legacy varint
        if let Ok(msg) = bincode::deserialize::<LegacyMessage>(bytes) {
            if msg.version == PROTOCOL_VERSION {
                return Ok(msg.into_current());
            }
        }
        anyhow::bail!(
            "failed to decode message: neither current nor legacy schema succeeded"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_fixint() {
        let msg = Message::new(42, vec![]);
        let bytes = msg.to_bytes().unwrap();
        let decoded = Message::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.sequence, 42);
        assert!(decoded.events.is_empty());
    }

    #[test]
    fn test_legacy_encoding_fallback() {
        // Simulate a payload serialized with the old bincode::serialize (varint)
        let msg = Message::new(7, vec![]);
        let bytes = bincode::serialize(&msg).unwrap();
        let decoded = Message::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.sequence, 7);
    }

    #[test]
    fn test_garbage_bytes_fail() {
        let bytes = vec![0xFF; 20];
        assert!(Message::from_bytes(&bytes).is_err());
    }

    /// Simulate decoding a payload that was serialized with the OLD struct shapes
    /// (CpuSample without symbol fields, LockEvent without stack_symbols).
    #[test]
    fn test_legacy_schema_decode() {
        // Serialize with legacy structs via fixint
        let legacy_msg = LegacyMessage {
            version: PROTOCOL_VERSION,
            sequence: 99,
            events: vec![
                LegacyProfileEvent::CpuSample(LegacyCpuSample {
                    timestamp: 5000,
                    pid: 10,
                    tid: 11,
                    cpu_id: 0,
                    user_stack: vec![0x1000, 0x2000],
                    kernel_stack: vec![0xffff0000],
                    comm: "old-agent".to_string(),
                }),
                LegacyProfileEvent::Lock(LegacyLockEvent {
                    timestamp: 6000,
                    pid: 10,
                    tid: 11,
                    lock_addr: 0xabcd,
                    hold_time_ns: 0,
                    wait_time_ns: 300,
                    stack_trace: vec![0x3000],
                    comm: "old-agent".to_string(),
                }),
            ],
        };
        let bytes = wire_bincode().serialize(&legacy_msg).unwrap();

        // Current decoder must handle this
        let decoded = Message::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.sequence, 99);
        assert_eq!(decoded.events.len(), 2);
        match &decoded.events[0] {
            ProfileEvent::CpuSample(s) => {
                assert_eq!(s.pid, 10);
                assert_eq!(s.user_stack, vec![0x1000, 0x2000]);
                assert!(s.user_stack_symbols.is_empty());
                assert!(s.kernel_stack_symbols.is_empty());
            }
            _ => panic!("expected CpuSample"),
        }
        match &decoded.events[1] {
            ProfileEvent::Lock(e) => {
                assert_eq!(e.lock_addr, 0xabcd);
                assert!(e.stack_symbols.is_empty());
            }
            _ => panic!("expected Lock"),
        }
    }

    /// Verify new-format roundtrip still works with symbol fields populated.
    #[test]
    fn test_new_schema_with_symbols() {
        let msg = Message::new(
            50,
            vec![ProfileEvent::CpuSample(CpuSample {
                timestamp: 1000,
                pid: 1,
                tid: 1,
                cpu_id: 0,
                user_stack: vec![0x100],
                kernel_stack: vec![],
                comm: "sym".to_string(),
                user_stack_symbols: vec![Some("main".to_string())],
                kernel_stack_symbols: vec![],
            })],
        );
        let bytes = msg.to_bytes().unwrap();
        let decoded = Message::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.events.len(), 1);
        match &decoded.events[0] {
            ProfileEvent::CpuSample(s) => {
                assert_eq!(s.user_stack_symbols, vec![Some("main".to_string())]);
            }
            _ => panic!("expected CpuSample"),
        }
    }
}
