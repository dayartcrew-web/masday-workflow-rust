//! Workflow routes — wired to WorkflowService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::middleware::error_handler::ApiError;
use crate::AppState;

pub fn workflow_routes() -> Router<AppState> {
    Router::new()
        .route("/workflows", post(create_workflow).get(list_workflows))
        .route("/workflows/active", get(get_active_workflows))
        .route("/workflows/{id}", get(get_workflow).delete(delete_workflow))
        .route("/workflows/{id}/execute", post(execute_workflow))
        .route("/workflows/{id}/status", get(get_workflow_status))
        .route(
            "/workflows/{id}/tasks",
            post(add_task).get(list_workflow_tasks),
        )
        .route("/workflows/{id}/plan", get(get_plan).post(create_plan))
        .route("/workflows/{id}/start-task", post(start_current_task))
        .route("/workflows/{id}/complete-task", post(complete_current_task))
        .route("/workflows/{id}/save-progress", post(save_progress))
        .route("/workflows/{id}/context-pack", get(build_context_pack))
}

#[derive(Deserialize)]
struct CreateWorkflowInput {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    project_path: Option<String>,
}

async fn create_workflow(
    State(state): State<AppState>,
    Json(input): Json<CreateWorkflowInput>,
) -> Result<Json<Value>, ApiError> {
    let wf = masday_service::WorkflowService::create_workflow(
        &state.pool,
        input.name,
        input.description,
        input.project_path,
    )
    .await?;
    Ok(Json(
        serde_json::json!({"id": wf.id, "name": wf.name, "status": wf.status}),
    ))
}

async fn list_workflows(
    State(state): State<AppState>,
    pagination: crate::extractors::pagination::Pagination,
) -> Result<Json<Value>, ApiError> {
    let workflows = masday_service::WorkflowService::list_workflows(
        &state.pool,
        pagination.limit() as i64,
        pagination.offset() as i64,
    )
    .await?;
    Ok(Json(serde_json::json!(workflows)))
}

async fn get_active_workflows(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let workflows = masday_service::WorkflowService::get_active_workflows(&state.pool).await?;
    Ok(Json(serde_json::json!(workflows)))
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let wf = masday_service::WorkflowService::get_workflow(&state.pool, &id).await?;
    Ok(Json(serde_json::json!(wf)))
}

async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    masday_service::WorkflowService::delete_workflow(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({"deleted": id})))
}

async fn execute_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let wf = masday_service::WorkflowService::execute_workflow(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({"id": wf.id, "status": wf.status})))
}

async fn get_workflow_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let wf = masday_service::WorkflowService::get_workflow(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({"id": wf.id, "status": wf.status})))
}

async fn add_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();
    let plan_id = payload
        .get("plan_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let agent = payload
        .get("agent")
        .and_then(|v| v.as_str())
        .map(String::from);
    let deps: Option<Vec<String>> =
        payload
            .get("dependencies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

    let task =
        masday_service::TaskService::add_task(&state.pool, id, plan_id, name, agent, deps).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn list_workflow_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let tasks = masday_service::TaskService::list_tasks(&state.pool, &id).await?;
    Ok(Json(serde_json::json!(tasks)))
}

async fn get_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = masday_service::PlanService::get_plan(&state.pool, &id).await?;
    Ok(Json(serde_json::json!(plan)))
}

async fn create_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_data = payload.get("plan").cloned().unwrap_or(payload);
    let plan = masday_service::PlanService::create_plan(&state.pool, id, plan_data).await?;
    Ok(Json(serde_json::json!(plan)))
}

async fn start_current_task(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let task = masday_service::TaskService::start_task(&state.pool, &workflow_id, &task_id).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn complete_current_task(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let result = payload.get("result").cloned();
    let task =
        masday_service::TaskService::complete_task(&state.pool, &workflow_id, &task_id, result)
            .await?;
    Ok(Json(serde_json::json!(task)))
}

async fn save_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let agent_name = payload
        .get("agent_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let note = payload
        .get("progress_note")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    masday_service::TaskService::save_progress(&state.pool, &id, &task_id, agent_name, note, None)
        .await?;
    Ok(Json(serde_json::json!({"saved": true})))
}

async fn build_context_pack(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pack =
        masday_service::ContextService::build_context_pack(&state.pool, &id, &id, &id).await?;
    Ok(Json(pack))
}
