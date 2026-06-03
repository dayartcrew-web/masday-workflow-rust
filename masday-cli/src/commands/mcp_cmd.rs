//! MCP command — starts the MCP stdio server.
//!
//! Reads config to set environment variables before starting.
//! - Standalone/Local: runs stdio with SQLite (no external deps)
//! - Remote: sets MASDAY_API_URL + MASDAY_API_KEY env vars for HTTP proxy tools

use anyhow::Result;

/// Start the MCP stdio server
pub async fn run() -> Result<()> {
    // Load config if available and set env vars (non-fatal if missing)
    if let Some(config) = crate::config::MasdayConfig::load() {
        config.set_env_vars();
    }

    // MCP runs on stdio — delegates to masday_mcp library (SQLite-only mode)
    masday_mcp::run_stdio()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
}
