//! Authentication middleware

use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

/// API key authentication middleware
pub async fn auth_middleware(
    _headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Placeholder implementation - would validate API key from Authorization header
    // For now, just pass through
    next.run(request).await
}
