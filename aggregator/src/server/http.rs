//! Admin HTTP server for health checks and metrics (Phase 7)

use crate::audit;
use crate::buffer::InMemoryBuffer;
use crate::metrics;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;

/// Start the admin HTTP server serving /healthz, /readyz, and /metrics.
pub async fn serve_admin(
    addr: SocketAddr,
    buffer: Arc<InMemoryBuffer>,
) -> Result<(), hyper::Error> {
    let make_svc = make_service_fn(move |_| {
        let buffer = buffer.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let buffer = buffer.clone();
                async move { handle(req, &buffer) }
            }))
        }
    });

    tracing::info!("Admin HTTP server listening on {}", addr);
    Server::bind(&addr).serve(make_svc).await
}

fn handle(
    req: Request<Body>,
    buffer: &InMemoryBuffer,
) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let (response, status) = match path {
        "/healthz" => (Response::new(Body::from("ok\n")), 200),

        "/readyz" => match buffer.len() {
            Ok(_) => (Response::new(Body::from("ready\n")), 200),
            Err(_) => (
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from("not ready\n"))
                    .unwrap(),
                503,
            ),
        },

        "/metrics" => {
            let body = metrics::encode_metrics();
            (
                Response::builder()
                    .header("Content-Type", "text/plain; version=0.0.4")
                    .body(Body::from(body))
                    .unwrap(),
                200,
            )
        }

        _ => (
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("not found\n"))
                .unwrap(),
            404,
        ),
    };

    if path == "/metrics" || path == "/readyz" || path == "/healthz" {
        audit::admin_http_request(path, status);
    }
    Ok(response)
}
