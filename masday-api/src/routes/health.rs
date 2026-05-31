//! Health check routes — with real DB connectivity check

use axum::routing::get;
use axum::{extract::State, Json, Router};
use serde_json::Value;

use crate::AppState;

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/db", get(db_health))
}

async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "masday-api",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn db_health(State(state): State<AppState>) -> Json<Value> {
    match masday_db::pool::health_check(&state.pool).await {
        Ok(_) => {
            let status = masday_db::pool::get_pool_status(&state.pool);
            Json(serde_json::json!({
                "status": "healthy",
                "database": "connected",
                "pool": {
                    "size": status.size,
                    "available": status.available,
                    "max_size": status.max_size
                }
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "unhealthy",
            "database": "disconnected",
            "error": e
        })),
    }
}
