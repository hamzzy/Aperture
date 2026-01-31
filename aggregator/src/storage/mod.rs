//! Storage backends (Phase 5+)

pub mod clickhouse;
pub mod scylla;

use anyhow::Result;
use aperture_shared::types::profile::Profile;

/// Storage backend trait
pub trait Storage: Send + Sync {
    /// Store a profile
    fn store_profile(&self, profile: &Profile) -> Result<()>;

    /// Query profiles by criteria
    fn query_profiles(&self, query: &str) -> Result<Vec<Profile>>;
}
