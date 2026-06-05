//! MCP command — starts the MCP stdio server.
//!
//! Reads config to determine mode:
//! - "standalone" or no config → SQLite-only (no external deps)
//! - "local" or "remote" → HTTP proxy to API server (reads api_url + api_key from config)

use anyhow::Result;

/// Start the MCP stdio server
pub async fn run() -> Result<()> {
    // Load config if available and set env vars (non-fatal if missing)
    let config = crate::config::MasdayConfig::load();
    if let Some(ref cfg) = config {
        cfg.set_env_vars();
    }

    // Determine mode from config
    let mode = config
        .as_ref()
        .map(|c| c.mode.as_str())
        .unwrap_or("standalone");

    match mode {
        "local" | "remote" => {
            // HTTP proxy mode — connect to API server
            let cfg = config.as_ref().unwrap();
            let api_url = cfg.api_url.clone();
            let api_key = cfg.api_key.clone();

            eprintln!(
                "[masday] MCP server starting in {} mode → {}",
                mode, api_url
            );

            masday_mcp::run_http(api_url, api_key)
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
        }
        _ => {
            // Standalone mode — SQLite, no external deps
            eprintln!("[masday] MCP server starting in standalone mode (SQLite)");

            masday_mcp::run_stdio()
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
        }
    }
}
