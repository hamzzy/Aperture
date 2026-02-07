//! Storage backends (Phase 5+)

#[cfg(feature = "clickhouse-storage")]
pub mod clickhouse;
pub mod scylla;

use anyhow::Result;
use aperture_shared::types::profile::Profile;
use async_trait::async_trait;

/// Persistent batch store for Phase 6 (e.g. ClickHouse). Optional in the push path.
#[async_trait]
pub trait BatchStore: Send + Sync {
    /// Persist one batch. Called after in-memory buffer is updated.
    async fn write_batch(
        &self,
        agent_id: &str,
        sequence: u64,
        received_at_ns: i64,
        event_count: u32,
        payload: &[u8],
    ) -> Result<(), String>;

    /// Query persisted batches (time range, agent filter). Default returns empty.
    async fn query_batches(
        &self,
        _agent_id: Option<&str>,
        _time_start_ns: Option<i64>,
        _time_end_ns: Option<i64>,
        _limit: u32,
    ) -> Result<Vec<(String, u64, u32, i64)>, String> {
        Ok(Vec::new())
    }
}

/// Storage backend trait (profile-level; used for future aggregation)
pub trait Storage: Send + Sync {
    /// Store a profile
    fn store_profile(&self, profile: &Profile) -> Result<()>;

    /// Query profiles by criteria
    fn query_profiles(&self, query: &str) -> Result<Vec<Profile>>;
}
