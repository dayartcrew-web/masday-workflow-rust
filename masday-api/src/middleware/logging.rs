//! Request/response logging middleware

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Logging middleware
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Placeholder implementation - would log request/response
    next.run(request).await
}
