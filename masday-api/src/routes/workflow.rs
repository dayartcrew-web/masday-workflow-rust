//! Workflow routes — wired to WorkflowService via ApiError

use axum::routing::{get, post};
use axum::{
    extract::{Path, Query, State},
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
        .route("/workflows/resume-suggestion", post(resume_suggestion))
        .route(
            "/workflows/parallel-branches",
            post(create_parallel_branches),
        )
        .route(
            "/workflows/parallel-branches/complete",
            post(complete_parallel_branch),
        )
        .route("/workflows/{id}", get(get_workflow).delete(delete_workflow))
        .route("/workflows/{id}/update", post(update_workflow_status))
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
        .route(
            "/workflows/{id}/parallel-branches",
            get(list_parallel_branches),
        )
        .route(
            "/workflows/{id}/synthesis-ready",
            post(mark_synthesis_ready),
        )
        .route(
            "/workflows/{id}/verification-ready",
            post(mark_verification_ready),
        )
        .route("/workflows/{id}/execution-mode", post(set_execution_mode))
        .route("/workflows/{id}/current-task", get(get_current_task))
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

#[derive(Deserialize)]
struct ListWorkflowsQuery {
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    per_page: Option<usize>,
    #[serde(default)]
    project_path: Option<String>,
}

async fn list_workflows(
    State(state): State<AppState>,
    Query(query): Query<ListWorkflowsQuery>,
) -> Result<Json<Value>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let pp = query.project_path.as_deref();

    let workflows = masday_service::WorkflowService::list_workflows(
        &state.pool,
        per_page as i64,
        offset as i64,
        pp,
    )
    .await?;
    Ok(Json(serde_json::json!(workflows)))
}

async fn get_active_workflows(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let workflows =
        masday_service::WorkflowService::get_active_workflows(&state.pool, None).await?;
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

#[derive(Deserialize)]
struct UpdateWorkflowInput {
    status: Option<String>,
}

async fn update_workflow_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateWorkflowInput>,
) -> Result<Json<Value>, ApiError> {
    if let Some(status) = input.status {
        // Convert status string to WorkflowState (validates the string)
        let target_state = masday_service::workflow_service::status_to_state(&status)?;

        // Use transition_status which validates:
        // 1. State transition is allowed by state machine
        // 2. Transition prerequisites are met
        let wf = masday_service::WorkflowService::transition_status(&state.pool, &id, target_state).await?;
        Ok(Json(serde_json::json!(wf)))
    } else {
        Err(ApiError(masday_core::AppError::validation(
            "status field required",
        )))
    }
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
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let task = masday_service::TaskService::start_task(&state.pool, &id, &task_id).await?;
    Ok(Json(serde_json::json!(task)))
}

async fn complete_current_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let result = payload.get("result").cloned();
    let task =
        masday_service::TaskService::complete_task(&state.pool, &id, &task_id, result).await?;
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
    // Use workflow_id for workflow and plan, find the current running task
    let tasks = masday_service::TaskService::list_tasks(&state.pool, &id).await?;
    let task_id = tasks
        .iter()
        .find(|t| t.status == "RUNNING")
        .map(|t| t.id.clone())
        .unwrap_or_default();
    let pack =
        match masday_service::ContextService::build_context_pack(&state.pool, &id, &id, &task_id)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                if matches!(e, masday_core::AppError::NotFound(_)) {
                    serde_json::json!({"tasks": tasks, "plan": null})
                } else {
                    return Err(ApiError::from(e));
                }
            }
        };
    Ok(Json(pack))
}

// ── Stub handlers for routes needed by MCP tools ──
// These validate that the referenced workflow exists before returning success.

async fn resume_suggestion(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, workflow_id).await?;
    Ok(Json(serde_json::json!({
        "workflow_id": workflow_id,
        "suggestion": "resume_from_last_completed_task",
        "next_step": "execute"
    })))
}

async fn create_parallel_branches(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, workflow_id).await?;

    let branches_raw = payload
        .get("branches")
        .cloned()
        .unwrap_or(serde_json::json!([]));

    let branch_keys: Vec<String> = if let Some(arr) = branches_raw.as_array() {
        // Cap branch count to prevent resource exhaustion
        const MAX_BRANCHES: usize = 100;
        const MAX_KEY_LEN: usize = 255;
        if arr.len() > MAX_BRANCHES {
            return Err(ApiError(masday_core::AppError::Validation(format!(
                "Too many branches: {} (max {})",
                arr.len(),
                MAX_BRANCHES
            ))));
        }
        arr.iter()
            .map(|b| match b {
                Value::String(s) => s.clone(),
                Value::Object(obj) => obj
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("branch")
                    .to_string(),
                _ => "branch".to_string(),
            })
            .filter(|s| {
                if s.len() > MAX_KEY_LEN {
                    tracing::warn!(
                        "Skipping branch key exceeding max length: {} chars",
                        s.len()
                    );
                    false
                } else {
                    true
                }
            })
            .collect()
    } else {
        vec![]
    };

    // Persist branches to DB via BranchRepo
    let repo = masday_db::repos::branch_repo::BranchRepo::new(state.pool.clone());
    let new_branches: Vec<masday_db::schema::NewParallelBranch> = branch_keys
        .iter()
        .map(|key| masday_db::schema::NewParallelBranch {
            workflow_id: workflow_id.to_string(),
            task_id: None,
            branch_key: key.clone(),
            role: "worker".to_string(),
            status: "PENDING".to_string(),
            input: serde_json::json!({}),
            output: None,
        })
        .collect();

    let created = repo.create_branches(&new_branches).await.map_err(|e| {
        ApiError(masday_core::AppError::Internal(format!(
            "Failed to create parallel branches: {}",
            e
        )))
    })?;

    Ok(Json(serde_json::json!(created)))
}

async fn complete_parallel_branch(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let workflow_id = payload
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let branch_key = payload
        .get("branch_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, workflow_id).await?;

    let repo = masday_db::repos::branch_repo::BranchRepo::new(state.pool.clone());
    let branches = repo.list_branches(workflow_id).await.map_err(|e| {
        ApiError(masday_core::AppError::Internal(format!(
            "Failed to list branches: {}",
            e
        )))
    })?;
    let branch = branches
        .iter()
        .find(|b| b.branch_key == branch_key)
        .ok_or_else(|| {
            ApiError(masday_core::AppError::NotFound(format!(
                "Branch '{}' not found",
                branch_key
            )))
        })?;
    let output = payload
        .get("output")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let completed = repo
        .complete_branch(&branch.id, output)
        .await
        .map_err(|e| {
            ApiError(masday_core::AppError::Internal(format!(
                "Failed to complete branch: {}",
                e
            )))
        })?;

    Ok(Json(serde_json::json!(completed)))
}

async fn list_parallel_branches(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let repo = masday_db::repos::branch_repo::BranchRepo::new(state.pool.clone());
    let branches = repo.list_branches(&workflow_id).await.map_err(|e| {
        ApiError(masday_core::AppError::Internal(format!(
            "Failed to list branches: {}",
            e
        )))
    })?;
    Ok(Json(serde_json::json!(branches)))
}

async fn mark_synthesis_ready(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, &id).await?;
    let session_key = payload
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(Json(
        serde_json::json!({"session_key": session_key, "synthesis_ready": true}),
    ))
}

async fn mark_verification_ready(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, &id).await?;
    let session_key = payload
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(Json(
        serde_json::json!({"session_key": session_key, "verification_ready": true}),
    ))
}

async fn set_execution_mode(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Validate workflow exists
    let _ = masday_service::WorkflowService::get_workflow(&state.pool, &id).await?;
    let session_key = payload
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("sequential");
    Ok(Json(
        serde_json::json!({"session_key": session_key, "execution_mode": mode}),
    ))
}

async fn get_current_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let tasks = masday_service::TaskService::list_tasks(&state.pool, &id).await?;
    let current = tasks.iter().find(|t| t.status == "RUNNING");
    match current {
        Some(t) => Ok(Json(serde_json::json!(t))),
        None => Ok(Json(serde_json::json!(null))),
    }
}
