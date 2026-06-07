//! Serve command — starts the API server with embedded dashboard frontend.

use anyhow::Result;

use crate::config::MasdayConfig;

/// Start the API server + embedded dashboard
pub async fn run(port: Option<u16>) -> Result<()> {
    let config = MasdayConfig::load_or_err()?;

    // Set env vars from config
    config.set_env_vars();

    let listen_port = port.unwrap_or(config.api_port);

    println!("{}", console::style("Starting Masday...").cyan());

    // Create DB pool
    let pool = masday_db::pool::init_pool_with_retry(3)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Database connection failed: {}. Run 'masday db start' first.",
                e
            )
        })?;

    // Initialize MCP client for HTTP proxy mode (tools call back to our own API)
    // Use empty API key since auth is disabled in dev mode (MASDAY_API_KEY not set)
    let api_url = format!("http://localhost:{}", listen_port);
    masday_mcp::client::init(api_url, String::new())
        .map_err(|e| anyhow::anyhow!("Failed to initialize MCP client: {}", e))?;

    // Build MCP handler with HTTP proxy registry (API server uses its own routes)
    let registry = masday_mcp::build_registry();
    let mcp_handler = masday_mcp::handler::McpHandler::new(registry);

    // Build API router with frontend fallback
    let state = masday_api::AppState::new(pool, mcp_handler);
    let api_routes = masday_api::build_router(state);

    let app = axum::Router::new()
        .merge(api_routes)
        .fallback(crate::frontend::handle_frontend);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], listen_port));
    println!("  Dashboard: http://localhost:{}", listen_port);
    println!("  API:       http://localhost:{}/api", listen_port);
    println!("  MCP (SSE): http://localhost:{}/mcp/sse", listen_port);
    println!("  MCP (HTTP): http://localhost:{}/mcp", listen_port);
    println!("  Press Ctrl+C to stop");
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
