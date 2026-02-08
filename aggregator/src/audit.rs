//! Audit logging for security and operational events (Phase 7).
//!
//! All events are emitted via `tracing` with a dedicated target so they can be
//! filtered and formatted (e.g. JSON) for audit pipelines.

use tracing::{info, warn};

const AUDIT_TARGET: &str = "aperture::audit";

/// Log gRPC authentication success (valid Bearer token or auth disabled).
pub fn grpc_auth_success() {
    info!(
        target: AUDIT_TARGET,
        event = "grpc_auth_success",
        result = "ok",
    );
}

/// Log gRPC authentication failure.
pub fn grpc_auth_failure(reason: &str) {
    warn!(
        target: AUDIT_TARGET,
        event = "grpc_auth_failure",
        result = "denied",
        reason = %reason,
    );
}

/// Log admin HTTP request (sensitive endpoints: metrics, readiness).
pub fn admin_http_request(path: &str, status: u16) {
    info!(
        target: AUDIT_TARGET,
        event = "admin_http_request",
        path = %path,
        status = %status,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_target_is_static() {
        assert_eq!(AUDIT_TARGET, "aperture::audit");
    }
}
