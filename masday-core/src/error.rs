//! Error types for the masday application

use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Internal service error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Conversion error
    #[error("Conversion error: {0}")]
    Conversion(String),
}

impl AppError {
    /// Get HTTP status code for the error
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::NotFound(_) => 404,
            AppError::Validation(_) => 400,
            AppError::Auth(_) => 401,
            AppError::Conversion(_) => 422,
            AppError::Database(_) | AppError::Internal(_) => 500,
        }
    }

    /// Create a not found error with resource type and id
    pub fn not_found(resource_type: &str, id: &str) -> Self {
        AppError::NotFound(format!("{} '{}' not found", resource_type, id))
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        AppError::Validation(message.into())
    }

    /// Create a database error
    pub fn database(message: impl Into<String>) -> Self {
        AppError::Database(message.into())
    }

    /// Create an auth error
    pub fn auth(message: impl Into<String>) -> Self {
        AppError::Auth(message.into())
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        AppError::Internal(message.into())
    }

    /// Create a conversion error
    pub fn conversion(message: impl Into<String>) -> Self {
        AppError::Conversion(message.into())
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, AppError>;


