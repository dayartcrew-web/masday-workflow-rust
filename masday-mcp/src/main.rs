//! MCP server entry point — JSON-RPC 2.0 over stdio

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| masday_core::constants::ports::api_base_url());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());
    masday_mcp::run(api_url, api_key).await
}
