use serde::{Deserialize, Serialize};

/// Filter input containing event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterInput {
    /// Event type (cpu, lock, syscall)
    pub event_type: String,
    
    /// Process ID
    pub pid: u32,
    
    /// Thread ID
    pub tid: u32,
    
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    
    /// Process name
    pub comm: String,
    
    /// Stack trace (instruction pointers)
    pub stack_trace: Vec<u64>,
    
    /// Event-specific data (JSON)
    pub event_data: String,
}

/// Filter output result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterResult {
    /// Keep the event as-is
    Keep,
    
    /// Drop the event
    Drop,
    
    /// Transform the event
    Transform(FilterInput),
}

/// Filter API version
pub const FILTER_API_VERSION: u32 = 1;
