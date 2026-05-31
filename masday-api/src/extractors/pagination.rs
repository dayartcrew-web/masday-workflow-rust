//! Pagination extractor with validation

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

/// Maximum allowed items per page
const MAX_PER_PAGE: usize = 100;
/// Default items per page
const DEFAULT_PER_PAGE: usize = 20;
/// Default page number (1-indexed)
const DEFAULT_PAGE: usize = 1;

/// Pagination parameters extracted from query string
///
/// Supports `page` and `per_page` query parameters with sensible defaults.
/// Clamps `per_page` to MAX_PER_PAGE (100) to prevent unbounded queries.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    /// Page number (1-indexed, default: 1)
    #[serde(default = "default_page")]
    pub page: usize,

    /// Items per page (default: 20, max: 100)
    #[serde(default = "default_per_page")]
    pub per_page: usize,
}

fn default_page() -> usize {
    DEFAULT_PAGE
}

fn default_per_page() -> usize {
    DEFAULT_PER_PAGE
}

impl Pagination {
    /// Calculate the SQL OFFSET value
    pub fn offset(&self) -> usize {
        if self.page == 0 {
            0
        } else {
            (self.page - 1) * self.per_page
        }
    }

    /// Get the effective LIMIT value (clamped to MAX_PER_PAGE)
    pub fn limit(&self) -> usize {
        self.per_page.clamp(1, MAX_PER_PAGE)
    }

    /// Clamp values to valid ranges
    pub fn clamp(&mut self) {
        if self.page == 0 {
            self.page = DEFAULT_PAGE;
        }
        self.per_page = self.per_page.clamp(1, MAX_PER_PAGE);
    }
}

/// Extract Pagination from query params, with validation
/// Generic over any state type since pagination doesn't depend on state
impl<S: Send + Sync> FromRequestParts<S> for Pagination {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = {
            let query = parts.uri.query().unwrap_or("");
            let mut pagination: Pagination =
                serde_html_form::from_str(query).unwrap_or(Pagination {
                    page: DEFAULT_PAGE,
                    per_page: DEFAULT_PER_PAGE,
                });
            pagination.clamp();
            Ok(pagination)
        };
        std::future::ready(result)
    }
}

/// Error response for pagination failures
pub struct PaginationError(pub String);

impl IntoResponse for PaginationError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": self.0,
                "code": 400
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let mut p = Pagination {
            page: 0,
            per_page: 0,
        };
        p.clamp();
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, 1);
    }

    #[test]
    fn test_clamp_per_page_to_max() {
        let mut p = Pagination {
            page: 1,
            per_page: 500,
        };
        p.clamp();
        assert_eq!(p.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn test_offset_calculation() {
        let p = Pagination {
            page: 3,
            per_page: 20,
        };
        assert_eq!(p.offset(), 40);
    }

    #[test]
    fn test_offset_first_page() {
        let p = Pagination {
            page: 1,
            per_page: 20,
        };
        assert_eq!(p.offset(), 0);
    }

    #[test]
    fn test_limit_clamped() {
        let p = Pagination {
            page: 1,
            per_page: 500,
        };
        assert_eq!(p.limit(), MAX_PER_PAGE);
    }

    #[test]
    fn test_limit_minimum() {
        let p = Pagination {
            page: 1,
            per_page: 0,
        };
        assert_eq!(p.limit(), 1);
    }
}
