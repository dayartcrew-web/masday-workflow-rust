//! Review MCP tools - HTTP client calls to API

use crate::client;
use masday_core::validate_uuid;
use serde_json::Value;

pub async fn review_submit(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/reviews", args).await
}

pub async fn review_get_latest(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task_id".to_string())?;

    if !validate_uuid(task_id) {
        return Err(format!("Invalid task_id format: '{}'", task_id).into());
    }

    // workflow_id is required by the API to scope the review lookup
    // (get_latest filters by workflow_id AND task_id). Pass it as a query param.
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;

    client::api_get(&format!(
        "/api/reviews/task/{}?workflow_id={}",
        task_id, workflow_id
    ))
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_review_get_latest_args() {
        let args = json!({ "task_id": "task-123" });
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        assert!(task_id.is_some());
        assert_eq!(task_id.unwrap(), "task-123");

        let args = json!({});
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        assert!(task_id.is_none());

        // Verify the error message
        let args = json!({});
        let result = std::panic::catch_unwind(|| {
            let task_id = args
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing task_id".to_string());
            task_id
        });
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }
}
