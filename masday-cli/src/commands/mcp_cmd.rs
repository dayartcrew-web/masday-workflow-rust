//! MCP command — starts the MCP stdio server.
//!
//! Mode selection:
//! - "standalone" → SQLite only (no external deps)
//! - "local" → PostgreSQL (primary) + SQLite (cache) + Ollama (embed)
//! - "remote" → HTTP proxy to API server

use anyhow::Result;

/// Start the MCP stdio server
pub async fn run() -> Result<()> {
    let config = crate::config::MasdayConfig::load();
    if let Some(ref cfg) = config {
        cfg.set_env_vars();
    }

    // Show embedding provider/model + health (reads ~/.masday/config.toml directly).
    masday_mcp::print_embedding_diagnostics().await;

    let mode = config
        .as_ref()
        .map(|c| c.mode.as_str())
        .unwrap_or("standalone");

    match mode {
        "remote" => {
            let cfg = config.as_ref().unwrap();
            let api_url = cfg.api_url.clone();
            let api_key = cfg.api_key.clone();

            eprintln!("[masday] MCP server starting in remote mode → {}", api_url);

            masday_mcp::run_http(api_url, api_key)
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
        }
        "local" => {
            eprintln!("[masday] MCP server starting in local mode (PostgreSQL + SQLite + Ollama)");

            masday_mcp::run_local()
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
        }
        _ => {
            eprintln!("[masday] MCP server starting in standalone mode (SQLite only)");

            masday_mcp::run_stdio()
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
        }
    }
}
