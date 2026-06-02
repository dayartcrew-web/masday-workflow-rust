//! MCP command — starts the MCP stdio server.

use anyhow::Result;

use crate::config::MasdayConfig;

/// Start the MCP stdio server
pub async fn run() -> Result<()> {
    let config = MasdayConfig::load_or_err()?;

    // MCP runs on stdio — delegates to masday_mcp library
    masday_mcp::run(config.api_url, config.api_key)
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
}
