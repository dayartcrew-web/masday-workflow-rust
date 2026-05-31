//! Graph routes — wired to GraphRepo via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::schema::{NewGraphEdge, NewGraphNode};

pub fn graph_routes() -> Router<AppState> {
    Router::new()
        .route("/graph/nodes", post(add_node))
        .route("/graph/nodes/{id}", get(get_node).delete(delete_node))
        .route("/graph/edges", post(add_edge))
        .route("/graph/search", post(search_nodes))
}

async fn add_node(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::GraphRepo::new(state.pool.clone());
    let node = NewGraphNode {
        node_type: payload
            .get("node_type")
            .and_then(|v| v.as_str())
            .unwrap_or("entity")
            .to_string(),
        name: payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        properties: payload.get("properties").cloned(),
    };
    let node = repo.add_node(&node).await?;
    Ok(Json(serde_json::json!(node)))
}

async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::GraphRepo::new(state.pool.clone());
    let node = repo.get_node(&id).await?;
    Ok(Json(serde_json::json!(node)))
}

async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::GraphRepo::new(state.pool.clone());
    repo.delete_node(&id).await?;
    Ok(Json(serde_json::json!({"deleted": id})))
}

async fn add_edge(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::GraphRepo::new(state.pool.clone());
    let edge = NewGraphEdge {
        source_node_id: payload
            .get("source_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        target_node_id: payload
            .get("target_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        relation_type: payload
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("related_to")
            .to_string(),
        weight: payload.get("weight").and_then(|v| v.as_f64()),
        bidirectional: payload.get("bidirectional").and_then(|v| v.as_bool()),
    };
    let edge = repo.add_edge(&edge).await?;
    Ok(Json(serde_json::json!(edge)))
}

async fn search_nodes(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::GraphRepo::new(state.pool.clone());
    let node_type = payload
        .get("node_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name_pattern = payload.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let limit = payload.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
    let nodes = repo.search_nodes(node_type, name_pattern, limit).await?;
    Ok(Json(serde_json::json!(nodes)))
}
