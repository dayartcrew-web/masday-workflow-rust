//! Request/response logging middleware with tracing

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// Request/response logging middleware
///
/// Logs method, path, status code, and duration for every request.
/// Uses tracing spans for structured logging.
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let start = Instant::now();

    let response = next.run(request).await;

    let elapsed = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        elapsed_ms = elapsed.as_millis() as u64,
        "{} {} → {} in {}ms",
        method,
        path,
        status,
        elapsed.as_millis()
    );

    response
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_logging_placeholder() {
        // Logging middleware is integration-tested via the router
        // Placeholder test
    }
}
