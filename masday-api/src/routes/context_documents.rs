//! Context documents routes — CRUD for ContextDocument

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::extractors::pagination::Pagination;
use crate::middleware::error_handler::ApiError;
use crate::AppState;
use masday_db::repos::ContextDocumentRepo;

#[derive(Deserialize)]
struct ListContextDocumentsQuery {
    #[serde(default)]
    workflow_id: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
}

pub fn context_document_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/context-documents",
            post(create_context_document).get(list_context_documents),
        )
        .route(
            "/context-documents/{id}",
            get(get_context_document).delete(delete_context_document),
        )
        .route(
            "/context-documents/workflow/{workflow_id}",
            get(list_by_workflow),
        )
        .route(
            "/context-documents/source/{source_type}",
            get(list_by_source_type),
        )
        .route(
            "/context-documents/fingerprint/{fingerprint}",
            get(get_by_fingerprint),
        )
}

/// POST /context-documents — Create a new context document
async fn create_context_document(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let new_doc = masday_db::schema::NewContextDocument {
        workflow_id: payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        source_type: payload
            .get("source_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        source_ref: payload
            .get("source_ref")
            .and_then(|v| v.as_str())
            .map(String::from),
        title: payload
            .get("title")
            .and_then(|v| v.as_str())
            .map(String::from),
        content: payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        metadata: payload.get("metadata").cloned(),
        fingerprint: payload
            .get("fingerprint")
            .and_then(|v| v.as_str())
            .map(String::from),
        embedding: None,
    };
    let doc = repo.create(&new_doc).await?;
    Ok(Json(serde_json::json!(doc)))
}

/// GET /context-documents — List context documents (with optional filters)
async fn list_context_documents(
    State(state): State<AppState>,
    Query(params): Query<ListContextDocumentsQuery>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let docs = if let Some(wid) = &params.workflow_id {
        repo.list_by_workflow(wid).await?
    } else if let Some(source_type) = &params.source_type {
        repo.list_by_source_type(source_type, Some(pagination.limit() as i64))
            .await?
    } else {
        repo.list_all(Some(pagination.limit() as i64)).await?
    };
    Ok(Json(serde_json::json!(docs)))
}

/// GET /context-documents/{id} — Get a context document by ID
async fn get_context_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let doc = repo.get_by_id(&id).await?;
    Ok(Json(serde_json::json!(doc)))
}

/// GET /context-documents/workflow/{workflow_id} — List context documents for a workflow
async fn list_by_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let docs = repo.list_by_workflow(&workflow_id).await?;
    Ok(Json(serde_json::json!(docs)))
}

/// GET /context-documents/source/{source_type} — List context documents by source type
async fn list_by_source_type(
    State(state): State<AppState>,
    Path(source_type): Path<String>,
    pagination: Pagination,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let docs = repo
        .list_by_source_type(&source_type, Some(pagination.limit() as i64))
        .await?;
    Ok(Json(serde_json::json!(docs)))
}

/// GET /context-documents/fingerprint/{fingerprint} — Get a context document by fingerprint
async fn get_by_fingerprint(
    State(state): State<AppState>,
    Path(fingerprint): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    if let Some(doc) = repo.get_by_fingerprint(&fingerprint).await? {
        Ok(Json(serde_json::json!(doc)))
    } else {
        Ok(Json(serde_json::json!(null)))
    }
}

/// DELETE /context-documents/{id} — Delete a context document
async fn delete_context_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = ContextDocumentRepo::new(state.pool.clone());
    let deleted = repo.delete(&id).await?;
    Ok(Json(serde_json::json!({"deleted": deleted, "id": id})))
}
