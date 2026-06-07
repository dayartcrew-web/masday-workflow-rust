//! masday-api - HTTP API server layer

pub mod extractors;
pub mod middleware;
pub mod routes;
pub mod state;

pub use middleware::error_handler::app_error_into_response;
pub use state::AppState;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the complete API router with all routes and middleware
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .merge(routes::workflow_routes())
        .merge(routes::task_routes())
        .merge(routes::memory_routes())
        .merge(routes::review_routes())
        .merge(routes::session_routes())
        .merge(routes::policy_routes())
        .merge(routes::capability_routes())
        .merge(routes::context_routes())
        .merge(routes::reminder_routes())
        .merge(routes::graph_routes())
        .merge(routes::health_routes())
        .merge(routes::plan_routes())
        .merge(routes::progress_log_routes())
        .merge(routes::retrieval_log_routes())
        .merge(routes::token_usage_routes())
        .merge(routes::episodic_memory_routes())
        .merge(routes::context_document_routes())
        .merge(routes::llm_provider_config_routes())
        .with_state(state.clone());

    let mcp_routes = Router::new()
        .route("/mcp/sse", axum::routing::get(routes::mcp::sse_handler))
        .route(
            "/mcp/messages",
            axum::routing::post(routes::mcp::messages_handler),
        )
        .route("/mcp", axum::routing::post(routes::mcp::streamable_handler))
        .with_state(state);

    // Permissive CORS for local development (Next.js dashboard on :3000)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api", api_routes)
        .merge(mcp_routes)
        // Middleware order (outermost first):
        // 1. CORS — handle preflight before anything else
        // 2. Tracing — structured HTTP logging via tower-http
        // 3. Custom logging — method/path/status/duration logging
        // 4. Auth — API key validation (skips /api/health)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(
            middleware::logging::logging_middleware,
        ))
        .layer(axum::middleware::from_fn(middleware::auth::auth_middleware))
}

/// Run the API server with the given pool and port.
/// Blocks until Ctrl+C shutdown signal received.
pub async fn run(
    pool: masday_db::pool::DbPool,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a minimal registry - the serve command handles proper tool initialization
    let registry = masday_mcp::registry::ToolRegistry::new();
    let mcp_handler = masday_mcp::handler::McpHandler::new(registry);
    let state = AppState::new(pool, mcp_handler);
    let app = build_router(state);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("masday-api listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test
    }
}
