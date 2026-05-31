//! Workflow MCP tools - HTTP client calls to API

use crate::client;
use crate::safe_path;
use serde_json::Value;

/// Create workflow
pub async fn workflow_create(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/workflows", args).await
}

/// Execute workflow
pub async fn workflow_execute(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("id")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or workflow_id".to_string())?;
    client::api_post(&format!("/api/workflows/{}/execute", workflow_id), args).await
}

/// Get workflow status
pub async fn workflow_get_status(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("id")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or workflow_id".to_string())?;
    client::api_get(&format!("/api/workflows/{}/status", workflow_id)).await
}

/// Get workflow by ID
pub async fn workflow_get(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&safe_path!("/api/workflows/{}", workflow_id)).await
}

/// List workflows
pub async fn workflow_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(50);
    client::api_get(&format!(
        "/api/workflows?page={}&page_size={}",
        page, page_size
    ))
    .await
}

/// Get active workflow
pub async fn workflow_get_active(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/workflows/active").await
}

/// Delete workflow
pub async fn workflow_delete(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_delete(&safe_path!("/api/workflows/{}", workflow_id)).await
}

/// Add task to workflow
pub async fn workflow_add_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(&format!("/api/workflows/{}/tasks", workflow_id), args).await
}

/// Start task
pub async fn workflow_start_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(&format!("/api/workflows/{}/start-task", workflow_id), args).await
}

/// Complete task
pub async fn workflow_complete_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(
        &format!("/api/workflows/{}/complete-task", workflow_id),
        args,
    )
    .await
}

/// Save progress
pub async fn workflow_save_progress(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(
        &format!("/api/workflows/{}/save-progress", workflow_id),
        args,
    )
    .await
}

/// Create plan
pub async fn workflow_create_plan(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(&format!("/api/workflows/{}/plan", workflow_id), args).await
}

/// Get plan
pub async fn workflow_get_plan(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&format!("/api/workflows/{}/plan", workflow_id)).await
}

/// List tasks
pub async fn workflow_list_tasks(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&format!("/api/workflows/{}/tasks", workflow_id)).await
}

/// Create parallel branches
pub async fn workflow_create_parallel_branches(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/workflows/parallel-branches", args).await
}

/// Complete parallel branch
pub async fn workflow_complete_parallel_branch(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/workflows/parallel-branches/complete", args).await
}

/// List parallel branches
pub async fn workflow_list_parallel_branches(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&format!("/api/workflows/{}/parallel-branches", workflow_id)).await
}

/// Mark synthesis ready
pub async fn workflow_mark_synthesis_ready(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(
        &format!("/api/workflows/{}/synthesis-ready", workflow_id),
        args,
    )
    .await
}

/// Mark verification ready
pub async fn workflow_mark_verification_ready(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(
        &format!("/api/workflows/{}/verification-ready", workflow_id),
        args,
    )
    .await
}

/// Set execution mode
pub async fn workflow_set_execution_mode(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_post(
        &format!("/api/workflows/{}/execution-mode", workflow_id),
        args,
    )
    .await
}

/// Resume suggestion
pub async fn workflow_resume_suggestion(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/workflows/resume-suggestion", args).await
}

/// Ping workflow
pub async fn workflow_ping(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::json!({"status": "pong"}))
}

/// Get current task
pub async fn workflow_get_current_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_get(&format!("/api/workflows/{}/current-task", workflow_id)).await
}
