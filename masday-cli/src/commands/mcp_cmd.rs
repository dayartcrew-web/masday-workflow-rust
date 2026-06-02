//! MCP command — starts the MCP stdio server.

use anyhow::Result;

/// Start the MCP stdio server
pub async fn run() -> Result<()> {
    // MCP runs on stdio — delegates to masday_mcp library (SQLite-only mode)
    masday_mcp::run_stdio()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
}
