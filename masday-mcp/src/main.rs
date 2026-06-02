//! MCP server entry point — JSON-RPC 2.0 over stdio

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = std::env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3010".to_string());
    let api_key = std::env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "dev-key".to_string());
    masday_mcp::run(api_url, api_key).await
}
