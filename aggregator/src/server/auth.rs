//! Bearer token authentication interceptor

use crate::audit;
use tonic::{Request, Status};

/// Create a tonic interceptor that validates bearer tokens.
///
/// If `expected_token` is `None`, authentication is disabled and all requests pass.
/// If set, requests must include `authorization: Bearer <token>` metadata.
pub fn make_auth_interceptor(
    expected_token: Option<String>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |req: Request<()>| {
        let Some(ref expected) = expected_token else {
            audit::grpc_auth_success();
            return Ok(req);
        };

        match req.metadata().get("authorization") {
            Some(val) => {
                let val_str = val.to_str().map_err(|_| {
                    audit::grpc_auth_failure("invalid authorization header encoding");
                    Status::unauthenticated("Invalid authorization header encoding")
                })?;
                let token = val_str.strip_prefix("Bearer ").ok_or_else(|| {
                    audit::grpc_auth_failure("missing Bearer prefix");
                    Status::unauthenticated("Missing Bearer prefix")
                })?;
                if token == expected.as_str() {
                    audit::grpc_auth_success();
                    Ok(req)
                } else {
                    audit::grpc_auth_failure("invalid token");
                    Err(Status::unauthenticated("Invalid token"))
                }
            }
            None => {
                audit::grpc_auth_failure("missing authorization header");
                Err(Status::unauthenticated("Missing authorization header"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_disabled() {
        let interceptor = make_auth_interceptor(None);
        let req = Request::new(());
        assert!(interceptor(req).is_ok());
    }

    #[test]
    fn test_valid_token() {
        let interceptor = make_auth_interceptor(Some("secret123".to_string()));
        let mut req = Request::new(());
        req.metadata_mut()
            .insert("authorization", "Bearer secret123".parse().unwrap());
        assert!(interceptor(req).is_ok());
    }

    #[test]
    fn test_invalid_token() {
        let interceptor = make_auth_interceptor(Some("secret123".to_string()));
        let mut req = Request::new(());
        req.metadata_mut()
            .insert("authorization", "Bearer wrong".parse().unwrap());
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn test_missing_header() {
        let interceptor = make_auth_interceptor(Some("secret123".to_string()));
        let req = Request::new(());
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn test_missing_bearer_prefix() {
        let interceptor = make_auth_interceptor(Some("secret123".to_string()));
        let mut req = Request::new(());
        req.metadata_mut()
            .insert("authorization", "secret123".parse().unwrap());
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }
}
