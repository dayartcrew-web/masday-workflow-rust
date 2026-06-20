//! Workflow MCP tools - HTTP client calls to API

use crate::client;
use crate::safe_path;
use masday_core::validate_uuid;
use percent_encoding::percent_encode;
use percent_encoding::AsciiSet;
use serde_json::Value;

/// Characters that need encoding in URL query values (RFC 3986)
const QUERY_ENCODE_SET: &AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Create workflow (auto-injects project_path from current directory if not provided)
pub async fn workflow_create(
    mut args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    if args.get("project_path").is_none() {
        if let Ok(dir) = std::env::current_dir() {
            args["project_path"] = serde_json::json!(dir.to_string_lossy().to_string());
        }
    }
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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

    client::api_get(&format!("/api/workflows/{}/status", workflow_id)).await
}

/// Get workflow by ID
pub async fn workflow_get(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

    client::api_get(&safe_path!("/api/workflows/{}", workflow_id)).await
}

/// List workflows, filtered by project_path (auto-set to current directory if not provided)
pub async fn workflow_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(50);
    let mut url = format!("/api/workflows?page={}&per_page={}", page, page_size);
    let pp = args
        .get("project_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });
    if let Some(pp) = pp {
        url.push_str(&format!(
            "&project_path={}",
            percent_encode(pp.as_bytes(), QUERY_ENCODE_SET)
        ));
    }
    client::api_get(&url).await
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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

    client::api_post(
        &format!("/api/workflows/{}/save-progress", workflow_id),
        args,
    )
    .await
}

/// Fail task — mark a task FAILED and auto-transition its workflow to FIX for
/// recovery (leverage #6). The service method + `POST /tasks/{id}/fail` route
/// shipped in #44; this exposes them to MCP clients. Targets the task-scoped
/// fail route: `task_id` selects the task (path), `workflow_id` + optional
/// `error` travel in the body (the service needs `workflow_id` for the FIX
/// auto-transition and the best-effort failure memory).
pub async fn workflow_fail_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task_id".to_string())?;

    if !validate_uuid(task_id) {
        return Err(format!("Invalid task_id format: '{}'", task_id).into());
    }

    client::api_post(&format!("/api/tasks/{}/fail", task_id), args).await
}

/// Transition a workflow to an explicit target state (general transition — the
/// FIX→EXECUTE resume path, since workflow_execute idempotency-returns once at
/// EXECUTE+). Thin wrapper over `POST /api/workflows/{id}/update`, which runs
/// the validated `transition_status` (incl. the FIX→EXECUTE FAILED-task reset).
pub async fn workflow_update_status(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let id = args
        .get("id")
        .or_else(|| args.get("workflow_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or workflow_id".to_string())?;

    if !validate_uuid(id) {
        return Err(format!("Invalid id format: '{}'", id).into());
    }

    client::api_post(&format!("/api/workflows/{}/update", id), args).await
}

/// Create plan
pub async fn workflow_create_plan(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

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

/// Ping workflow - returns actual system health check
#[cfg(feature = "sqlite")]
pub async fn workflow_ping(
    _args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Check if we can access SQLite
    let sqlite_status = match crate::sqlite::try_connection() {
        Ok(_) => "connected",
        Err(e) => {
            return Ok(serde_json::json!({
                "status": "unhealthy",
                "service": "masday-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "database": "disconnected",
                "error": format!("SQLite connection failed: {}", e)
            }));
        }
    };

    Ok(serde_json::json!({
        "status": "healthy",
        "service": "masday-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "database": sqlite_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "mode": std::env::var("MASDAY_MODE").unwrap_or_else(|_| "stdio".to_string())
    }))
}

/// Get current task
pub async fn workflow_get_current_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;

    if !validate_uuid(workflow_id) {
        return Err(format!("Invalid workflow_id format: '{}'", workflow_id).into());
    }

    client::api_get(&format!("/api/workflows/{}/current-task", workflow_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_workflow_ping() {
        // Initialize SQLite for this test (ignore if already initialized)
        let _ = crate::sqlite::init_sqlite();

        let result = workflow_ping(json!({})).await;
        assert!(result.is_ok());
        let result_json = result.unwrap();
        // Check that we get a healthy response
        assert_eq!(result_json["status"], "healthy");
        assert_eq!(result_json["service"], "masday-mcp");
        assert!(result_json.get("version").is_some());
        assert!(result_json.get("timestamp").is_some());
        assert!(result_json.get("database").is_some());
    }

    #[test]
    fn test_workflow_execute_args_fallback() {
        let args = json!({ "id": "wf-123" });
        let workflow_id = args
            .get("id")
            .or_else(|| args.get("workflow_id"))
            .and_then(|v| v.as_str());
        assert_eq!(workflow_id.unwrap(), "wf-123");

        let args = json!({ "workflow_id": "wf-456" });
        let workflow_id = args
            .get("id")
            .or_else(|| args.get("workflow_id"))
            .and_then(|v| v.as_str());
        assert_eq!(workflow_id.unwrap(), "wf-456");
    }

    #[test]
    fn test_workflow_list_default_pagination() {
        let args = json!({});
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
        let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(50);
        assert_eq!(page, 1);
        assert_eq!(page_size, 50);
    }

    #[test]
    fn test_workflow_list_custom_pagination() {
        let args = json!({ "page": 2, "page_size": 100 });
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
        let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(50);
        assert_eq!(page, 2);
        assert_eq!(page_size, 100);
    }

    #[test]
    fn test_url_building() {
        let workflow_id = "wf-123";
        let url = format!("/api/workflows/{}/execute", workflow_id);
        assert_eq!(url, "/api/workflows/wf-123/execute");

        let workflow_id = "wf-456";
        let url = format!("/api/workflows/{}/status", workflow_id);
        assert_eq!(url, "/api/workflows/wf-456/status");

        let workflow_id = "wf-789";
        let url = format!("/api/workflows/{}/plan", workflow_id);
        assert_eq!(url, "/api/workflows/wf-789/plan");
    }
}
