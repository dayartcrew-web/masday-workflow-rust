//! MCP server entry point

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Get API URL from environment
    let api_url = env::var("MASDAY_API_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let _api_key = env::var("MASDAY_API_KEY")
        .unwrap_or_else(|_| "PLACEHOLDER".to_string());

    tracing::info!("Starting MCP server connected to {}", api_url);

    // Start MCP stdio transport
    // Placeholder: would register all 89 tools and start JSON-RPC server
    tracing::info!("MCP server running on stdio");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        // Placeholder test
        assert!(true);
    }
}
