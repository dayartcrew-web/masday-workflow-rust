//! Shared HTTP client for API calls

use reqwest::Client;
use std::sync::OnceLock;
use tracing::{debug, error};

/// Validate and sanitize an ID for URL path segments.
/// Only allows alphanumeric, hyphens, and underscores.
/// Returns None if the ID contains path traversal characters or is empty.
pub fn sanitize_id(id: &str) -> Option<&str> {
    if id.is_empty() {
        return None;
    }
    // Block path traversal and special characters
    if id.contains('/')
        || id.contains('\\')
        || id.contains('?')
        || id.contains('#')
        || id.contains('&')
        || id.contains(' ')
        || id.contains('\0')
    {
        return None;
    }
    // Only allow alphanumeric, hyphens, underscores, dots
    if id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        Some(id)
    } else {
        None
    }
}

/// Macro to safely build API paths with validated IDs
#[macro_export]
macro_rules! safe_path {
    ($fmt:expr, $id:expr) => {{
        let id = $crate::client::sanitize_id($id)
            .ok_or_else(|| format!("Invalid ID: contains disallowed characters"))?;
        format!($fmt, id)
    }};
}

/// Global API URL (set once at startup)
static API_URL: OnceLock<String> = OnceLock::new();

/// Global API key (set once at startup)
static API_KEY: OnceLock<String> = OnceLock::new();

/// Global shared HTTP client (connection pooling)
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Initialize the global HTTP client and API configuration
/// Called once at startup from main.rs. Safe to call multiple times
/// (subsequent calls are no-ops).
pub fn init(
    api_url: String,
    api_key: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let _ = HTTP_CLIENT.set(client);
    let _ = API_URL.set(api_url);
    let _ = API_KEY.set(api_key);

    Ok(())
}

/// Get the shared HTTP client (panics if not initialized - use try_get_client for safe access)
pub fn client() -> &'static Client {
    HTTP_CLIENT.get().expect("HTTP client not initialized")
}

/// Safely get the HTTP client if initialized (returns None if not set)
pub fn try_get_client() -> Option<&'static Client> {
    HTTP_CLIENT.get()
}

/// Get the base API URL (panics if not set - use try_get_api_url for safe access)
pub fn api_url() -> &'static String {
    API_URL.get().expect("API URL not set")
}

/// Safely get the API URL if initialized (returns None if not set)
/// Use this for stdio mode where API client may not be initialized
pub fn try_get_api_url() -> Option<&'static String> {
    API_URL.get()
}

/// Get the API key
pub fn api_key() -> &'static String {
    API_KEY.get().expect("API key not set")
}

/// Make a GET request to the API
pub async fn api_get(
    path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}{}", api_url(), path);
    debug!("API GET: {}", url);

    let response = client()
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key()))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("API request timed out after 30s. The API server may be down or unreachable: {}", api_url())
            } else if e.is_connect() {
                format!("Cannot connect to API server at {}. Check if the server is running.", api_url())
            } else {
                format!("GET request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("API GET failed: {} - {}", status, error_text);
        return Err(format!("API GET failed: {} - {}", status, error_text).into());
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(result)
}

/// Make a POST request to the API
pub async fn api_post(
    path: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}{}", api_url(), path);
    debug!("API POST: {}", url);

    let response = client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key()))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("API request timed out after 30s. The API server may be down or unreachable: {}", api_url())
            } else if e.is_connect() {
                format!("Cannot connect to API server at {}. Check if the server is running.", api_url())
            } else {
                format!("POST request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("API POST failed: {} - {}", status, error_text);
        return Err(format!("API POST failed: {} - {}", status, error_text).into());
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(result)
}

/// Make a PATCH request to the API
pub async fn api_patch(
    path: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}{}", api_url(), path);
    debug!("API PATCH: {}", url);

    let response = client()
        .patch(&url)
        .header("Authorization", format!("Bearer {}", api_key()))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("API request timed out after 30s. The API server may be down or unreachable: {}", api_url())
            } else if e.is_connect() {
                format!("Cannot connect to API server at {}. Check if the server is running.", api_url())
            } else {
                format!("PATCH request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("API PATCH failed: {} - {}", status, error_text);
        return Err(format!("API PATCH failed: {} - {}", status, error_text).into());
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(result)
}

/// Make a DELETE request to the API
pub async fn api_delete(
    path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}{}", api_url(), path);
    debug!("API DELETE: {}", url);

    let response = client()
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_key()))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("API request timed out after 30s. The API server may be down or unreachable: {}", api_url())
            } else if e.is_connect() {
                format!("Cannot connect to API server at {}. Check if the server is running.", api_url())
            } else {
                format!("DELETE request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("API DELETE failed: {} - {}", status, error_text);
        return Err(format!("API DELETE failed: {} - {}", status, error_text).into());
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let result = init(
            masday_core::constants::ports::api_base_url(),
            "test-key".to_string(),
        );
        assert!(result.is_ok());
        assert_eq!(api_url(), &masday_core::constants::ports::api_base_url());
        assert_eq!(api_key(), "test-key");
    }

    #[test]
    #[should_panic(expected = "HTTP client not initialized")]
    fn test_client_not_initialized() {
        // This test must run before any init() call
        // In a real test suite, we'd use a test fixture
        client();
    }
}
