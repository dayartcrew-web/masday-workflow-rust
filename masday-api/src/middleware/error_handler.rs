//! Error handling — AppError → HTTP response mapping via ApiError wrapper

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use masday_core::AppError;

/// API error wrapper that implements IntoResponse
///
/// Wraps the core AppError to satisfy Rust's orphan rules
/// (can't impl foreign trait for foreign type).
#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        ApiError(err)
    }
}

/// Convert ApiError into an HTTP response
///
/// Maps domain errors to appropriate HTTP status codes:
/// - NotFound → 404
/// - Validation → 400
/// - Database → 500
/// - Auth → 401
/// - Internal → 500
/// - Conversion → 422
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", msg),
            ),
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {}", msg),
            ),
            AppError::Conversion(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
        };

        tracing::warn!(
            status = %status.as_u16(),
            error = %message,
            "Request error"
        );

        let body = serde_json::json!({
            "error": message,
            "code": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}

/// Standalone function for manual error conversion (backward compat)
pub fn app_error_into_response(error: AppError) -> Response {
    ApiError(error).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_not_found_maps_to_404() {
        let err = AppError::NotFound("Workflow not found".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_validation_maps_to_400() {
        let err = AppError::Validation("Invalid state transition".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_database_maps_to_500() {
        let err = AppError::Database("Connection failed".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_auth_maps_to_401() {
        let err = AppError::Auth("Invalid API key".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_internal_maps_to_500() {
        let err = AppError::Internal("Unexpected error".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_conversion_maps_to_422() {
        let err = AppError::Conversion("Cannot parse input".into());
        let response = ApiError(err).into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_from_app_error() {
        let err: AppError = AppError::NotFound("test".into());
        let api_err: ApiError = err.into();
        assert!(matches!(api_err.0, AppError::NotFound(_)));
    }
}
