//! masday-mcp-stdio — Standalone MCP server with direct PostgreSQL access
//!
//! No masday-api needed. Connects directly to PostgreSQL via DATABASE_URL.
//! Uses masday-service + masday-db for all DB operations.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    masday_mcp::run_stdio().await
}
