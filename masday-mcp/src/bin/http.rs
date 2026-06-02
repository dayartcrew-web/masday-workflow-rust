//! masday-mcp-http — Thin HTTP proxy to masday-api
//!
//! Requires masday-api running on MASYDAY_API_URL (default: http://localhost:30101).
//! All DB-dependent tools are forwarded as HTTP requests.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| masday_core::constants::ports::api_base_url());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());
    masday_mcp::run_http(api_url, api_key).await
}
