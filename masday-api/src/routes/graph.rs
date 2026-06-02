//! Graph routes — wired to GraphRepo via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_core::AppError;

/// Helper to create validation errors as ApiError
fn validation_err(msg: &str) -> ApiError {
    ApiError(AppError::Validation(msg.to_string()))
}
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

    // Support batch format: {"entities": [{name, entityType, observations}, ...]}
    if let Some(entities) = payload.get("entities").and_then(|v| v.as_array()) {
        // Cap batch size to prevent DoS
        if entities.len() > 100 {
            return Err(validation_err("entities array exceeds maximum of 100"));
        }
        let mut created = Vec::new();
        for ent in entities {
            let node_type = ent
                .get("entityType")
                .or_else(|| ent.get("node_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("entity");
            let name = ent.get("name").and_then(|v| v.as_str()).unwrap_or("");

            // Validate fields — fail closed on missing/empty name
            if name.is_empty() {
                return Err(validation_err("name is required for each entity"));
            }
            if node_type.len() > 100 || name.len() > 500 {
                return Err(validation_err("field exceeds maximum length"));
            }

            let node = NewGraphNode {
                node_type: node_type.to_string(),
                name: name.to_string(),
                properties: ent
                    .get("observations")
                    .map(|obs| {
                        serde_json::json!({
                            "observations": obs,
                        })
                    })
                    .or_else(|| ent.get("properties").cloned()),
            };
            let node = repo.add_node(&node).await?;
            created.push(serde_json::json!(node));
        }
        return Ok(Json(serde_json::json!({"created": created})));
    }

    // Single node format: {"name": "...", "node_type": "...", "properties": {...}}
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() {
        return Err(validation_err("name is required"));
    }
    let node_type = payload
        .get("node_type")
        .and_then(|v| v.as_str())
        .unwrap_or("entity");
    if node_type.len() > 100 || name.len() > 500 {
        return Err(validation_err("field exceeds maximum length"));
    }

    let node = NewGraphNode {
        node_type: node_type.to_string(),
        name: name.to_string(),
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
