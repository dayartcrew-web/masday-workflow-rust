//! Authentication middleware — API key validation via Authorization header

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth_header = headers.get("Authorization")?.to_str().ok()?;
    auth_header.strip_prefix("Bearer ")
}

/// API key authentication middleware
///
/// Validates the `Authorization: Bearer <key>` header against `MASDAY_API_KEY` env var.
/// Skips auth for health check endpoints (`/api/health`).
/// If `MASDAY_API_KEY` is not set, auth is disabled (dev mode).
pub async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Response {
    let path = request.uri().path();

    // Skip auth for health endpoints
    if path.starts_with("/api/health") {
        return next.run(request).await;
    }

    // If no API key configured, skip auth (dev mode)
    let Ok(expected_key) = std::env::var("MASDAY_API_KEY") else {
        return next.run(request).await;
    };

    // Empty key means auth disabled
    if expected_key.is_empty() {
        return next.run(request).await;
    }

    // Validate bearer token
    match extract_bearer_token(&headers) {
        Some(token) if token == expected_key => next.run(request).await,
        Some(_) => {
            tracing::warn!(path = %path, "Invalid API key");
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "error": "Invalid API key",
                    "code": 401
                })),
            )
                .into_response()
        }
        None => {
            tracing::warn!(path = %path, "Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "error": "Missing Authorization header. Use: Bearer <api-key>",
                    "code": 401
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer my-secret-key".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("my-secret-key"));
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn test_extract_bearer_token_wrong_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Basic dXNlcjpwYXNz".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
