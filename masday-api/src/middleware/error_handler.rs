//! Error handling middleware

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    Json as JsonResponse,
};
use masday_core::AppError;

pub fn app_error_into_response(error: AppError) -> Response {
    let (status, message) = match &error {
        AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg)),
        AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
        AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {}", msg)),
        AppError::Conversion(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
    };

    let body = serde_json::json!({
        "error": message,
        "code": status.as_u16()
    });

    (status, JsonResponse(body)).into_response()
}

