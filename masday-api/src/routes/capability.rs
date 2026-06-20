//! Capability routes — wired to CapabilityService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use masday_core::AppError;
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn capability_routes() -> Router<AppState> {
    Router::new()
        // Agent management
        .route("/capabilities/agent", post(create_agent))
        .route("/capabilities/agents", get(list_agents))
        // Skill management
        .route("/capabilities/skill", post(create_skill))
        .route("/capabilities/skills", get(list_skills))
        // Matching & discovery
        .route("/capabilities/match", get(match_agent))
        .route("/capabilities/templates", get(list_templates))
        // Scaffolding
        .route("/capabilities/scaffold", post(scaffold_feature))
        .route("/capabilities/scaffold-feature", post(scaffold_feature))
        .route("/capabilities/mcp-server", post(scaffold_mcp_server))
        // System
        .route("/capabilities/system-readiness", get(system_readiness))
        .route("/capabilities/readiness", get(system_readiness))
        // Audit
        .route("/capabilities/audit/{id}", get(workflow_audit))
}

#[derive(Deserialize)]
struct ProjectRootQuery {
    #[serde(default)]
    project_root: Option<String>,
    #[serde(default)]
    task_description: Option<String>,
    /// Alias used by MCP tools: ?task=<description>
    #[serde(default)]
    task: Option<String>,
}

async fn create_agent(Json(_payload): Json<Value>) -> Result<Json<Value>, ApiError> {
    // HTTP/remote mode cannot create agents: the stdio path writes
    // `.claude/agents/{name}.md` (and updates the registry) inside the *client's*
    // project, but the API server does not share that filesystem. The former
    // `{"created": true}` response lied about success — callers believed an agent
    // was registered when nothing was written. Fail honestly instead; creating
    // agents/skills requires local/stdio mode (`masday` CLI + stdio MCP server).
    Err(ApiError(AppError::Validation(
        "capability_create_agent is not supported in remote/HTTP mode: the API \
         server cannot write to your project's .claude/ directory. Use \
         local/stdio mode (the `masday` CLI's stdio MCP server) to create agents."
            .into(),
    )))
}

async fn create_skill(Json(_payload): Json<Value>) -> Result<Json<Value>, ApiError> {
    // See `create_agent`: HTTP/remote mode cannot write `.claude/skills/{name}/`
    // into the client's project. Fail honestly instead of the former
    // `{"created": true}` lie.
    Err(ApiError(AppError::Validation(
        "capability_create_skill is not supported in remote/HTTP mode: the API \
         server cannot write to your project's .claude/ directory. Use \
         local/stdio mode (the `masday` CLI's stdio MCP server) to create skills."
            .into(),
    )))
}

async fn list_agents(Query(query): Query<ProjectRootQuery>) -> Result<Json<Value>, ApiError> {
    let root = query.project_root.as_deref().unwrap_or(".");
    let agents = masday_service::CapabilityService::list_agents(root).await?;
    Ok(Json(serde_json::json!(agents)))
}

async fn list_skills(_query: Query<ProjectRootQuery>) -> Json<Value> {
    Json(serde_json::json!([]))
}

async fn match_agent(Query(query): Query<ProjectRootQuery>) -> Result<Json<Value>, ApiError> {
    // MCP tools use ?task=<description>, accept both param names
    let desc = query
        .task
        .as_deref()
        .or(query.task_description.as_deref())
        .unwrap_or("");
    let agent = masday_service::CapabilityService::match_agent(desc).await?;
    Ok(Json(serde_json::json!({"agent": agent})))
}

async fn list_templates() -> Json<Value> {
    Json(serde_json::json!([]))
}

async fn scaffold_feature(Json(_payload): Json<Value>) -> Result<Json<Value>, ApiError> {
    // See `create_agent`/`create_skill`: HTTP/remote mode cannot write
    // `.claude/agents/{name}.md` + `.claude/skills/{name}/SKILL.md` into the
    // client's project. Fail honestly instead of the former
    // `{"scaffolded": false, "message": "Not yet implemented"}` non-error stub.
    Err(ApiError(AppError::Validation(
        "capability_scaffold_feature is not supported in remote/HTTP mode: the API \
         server cannot write to your project's .claude/ directory. Use \
         local/stdio mode (the `masday` CLI's stdio MCP server) to scaffold features."
            .into(),
    )))
}

async fn scaffold_mcp_server(Json(_payload): Json<Value>) -> Result<Json<Value>, ApiError> {
    // See `create_agent`: HTTP/remote mode cannot write a new project directory
    // into the client's filesystem. The stdio path creates `{projectRoot}/{name}/`
    // locally; the API server has no access to that path. Fail honestly instead
    // of the former non-error stub.
    Err(ApiError(AppError::Validation(
        "capability_scaffold_mcp_server is not supported in remote/HTTP mode: the \
         API server cannot write to your project's filesystem. Use local/stdio \
         mode (the `masday` CLI's stdio MCP server) to scaffold an MCP server."
            .into(),
    )))
}

async fn system_readiness(State(state): State<AppState>) -> Json<Value> {
    let pool_status = masday_db::pool::get_pool_status(&state.pool);
    Json(serde_json::json!({
        "ready": true,
        "pool": {
            "size": pool_status.size,
            "available": pool_status.available,
            "max_size": pool_status.max_size,
        }
    }))
}

async fn workflow_audit(State(_state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    Json(serde_json::json!({
        "workflow_id": id,
        "audited": false,
        "message": "Not yet implemented"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    // Regression: create_agent / create_skill used to return `{"created": true}`
    // while writing nothing (theater-that-lies). They must now fail honestly in
    // remote/HTTP mode with a Validation error (400).

    #[tokio::test]
    async fn create_agent_fails_honestly_in_remote_mode() {
        let res = create_agent(Json(serde_json::json!({"name": "my-agent"}))).await;
        assert!(
            res.is_err(),
            "create_agent must fail honestly, not lie success"
        );
        match res.unwrap_err() {
            ApiError(AppError::Validation(msg)) => assert!(
                msg.contains("not supported in remote/HTTP mode"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected AppError::Validation, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_skill_fails_honestly_in_remote_mode() {
        let res = create_skill(Json(serde_json::json!({"name": "my-skill"}))).await;
        assert!(
            res.is_err(),
            "create_skill must fail honestly, not lie success"
        );
        match res.unwrap_err() {
            ApiError(AppError::Validation(msg)) => assert!(
                msg.contains("not supported in remote/HTTP mode"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected AppError::Validation, got: {other:?}"),
        }
    }

    // Regression: scaffold_feature / scaffold_mcp_server used to return
    // `{"scaffolded": false, ...}` non-error stubs. They must now fail honestly
    // in remote/HTTP mode (same constraint as create_agent/create_skill).

    #[tokio::test]
    async fn scaffold_feature_fails_honestly_in_remote_mode() {
        let res = scaffold_feature(Json(serde_json::json!({"name": "feat"}))).await;
        assert!(
            res.is_err(),
            "scaffold_feature must fail honestly, not lie success"
        );
        match res.unwrap_err() {
            ApiError(AppError::Validation(msg)) => assert!(
                msg.contains("not supported in remote/HTTP mode"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected AppError::Validation, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn scaffold_mcp_server_fails_honestly_in_remote_mode() {
        let res = scaffold_mcp_server(Json(serde_json::json!({"name": "srv"}))).await;
        assert!(
            res.is_err(),
            "scaffold_mcp_server must fail honestly, not lie success"
        );
        match res.unwrap_err() {
            ApiError(AppError::Validation(msg)) => assert!(
                msg.contains("not supported in remote/HTTP mode"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected AppError::Validation, got: {other:?}"),
        }
    }
}
