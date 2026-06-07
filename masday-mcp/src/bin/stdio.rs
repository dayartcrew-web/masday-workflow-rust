//! masday-mcp-stdio — Standalone MCP server with SQLite
//!
//! No external services needed. Uses local SQLite at ~/.masday/data.db.
//! Binary can be placed directly in PATH.

#![cfg(feature = "sqlite")]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    masday_mcp::run_stdio().await
}
