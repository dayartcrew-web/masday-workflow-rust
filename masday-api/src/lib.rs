//! masday-api - HTTP API server layer

pub mod routes;
pub mod middleware;
pub mod extractors;

// Re-export error handler
pub use middleware::error_handler::app_error_into_response;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
