//! Shared HTTP client for API calls

use reqwest::Client;
use std::sync::OnceLock;
use tracing::{debug, error};

/// Global API URL (set once at startup)
static API_URL: OnceLock<String> = OnceLock::new();

/// Global API key (set once at startup)
static API_KEY: OnceLock<String> = OnceLock::new();

/// Global shared HTTP client (connection pooling)
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Initialize the global HTTP client and API configuration
/// Called once at startup from main.rs
pub fn init(api_url: String, api_key: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    HTTP_CLIENT
        .set(client)
        .expect("HTTP client already initialized");
    API_URL.set(api_url).expect("API URL already set");
    API_KEY.set(api_key).expect("API key already set");

    Ok(())
}

/// Get the shared HTTP client
pub fn client() -> &'static Client {
    HTTP_CLIENT.get().expect("HTTP client not initialized")
}

/// Get the base API URL
pub fn api_url() -> &'static String {
    API_URL.get().expect("API URL not set")
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
        .map_err(|e| format!("GET request failed: {}", e))?;

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
        .map_err(|e| format!("POST request failed: {}", e))?;

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
        .map_err(|e| format!("PATCH request failed: {}", e))?;

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
        .map_err(|e| format!("DELETE request failed: {}", e))?;

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
        let result = init("http://localhost:3001".to_string(), "test-key".to_string());
        assert!(result.is_ok());
        assert_eq!(api_url(), "http://localhost:3001");
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
