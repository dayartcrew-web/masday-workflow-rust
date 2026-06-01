//! Memory MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

pub async fn memory_store(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories", args).await
}

pub async fn memory_store_research(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories/research", args).await
}

pub async fn memory_search(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_post("/api/memories/search", args).await
}

pub async fn memory_recall_documents(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories?workflow_id=X&limit=N
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
    client::api_get(&format!(
        "/api/memories?workflow_id={}&limit={}",
        workflow_id, limit
    ))
    .await
}

pub async fn memory_recall_document_by_type(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories/by-type?memory_type=X&limit=N
    let source_type = args
        .get("source_type")
        .or_else(|| args.get("memory_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("fact");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
    client::api_get(&format!(
        "/api/memories/by-type?memory_type={}&limit={}",
        source_type, limit
    ))
    .await
}

pub async fn memory_recall_by_task(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // API: GET /api/memories/by-task/{task_id}
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing task_id".to_string())?;
    client::api_get(&format!("/api/memories/by-task/{}", task_id)).await
}

pub async fn memory_recall_recent(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    let memory_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if memory_type.is_empty() {
        client::api_get(&format!("/api/memories/recent?limit={}", limit)).await
    } else {
        client::api_get(&format!(
            "/api/memories/recent?limit={}&memory_type={}",
            limit, memory_type
        ))
        .await
    }
}

pub async fn memory_update(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("id")
        .or_else(|| args.get("memory_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or memory_id".to_string())?;
    client::api_patch(&format!("/api/memories/{}", memory_id), args).await
}

pub async fn memory_delete(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let memory_id = args
        .get("id")
        .or_else(|| args.get("memory_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or memory_id".to_string())?;
    client::api_delete(&format!("/api/memories/{}", memory_id)).await
}

pub async fn memory_delete_by_workflow(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let workflow_id = args
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing workflow_id".to_string())?;
    client::api_delete(&format!("/api/memories/workflow/{}", workflow_id)).await
}

pub async fn memory_stats(_args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    client::api_get("/api/memories/stats").await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_memory_recall_documents_args_parsing() {
        let args = json!({
            "workflow_id": "wf-123",
            "limit": 50
        });

        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);

        assert_eq!(workflow_id, "wf-123");
        assert_eq!(limit, 50);
    }

    #[test]
    fn test_memory_recall_documents_defaults() {
        let args = json!({});
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str()).unwrap_or("");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);

        assert_eq!(workflow_id, "");
        assert_eq!(limit, 20);
    }

    #[test]
    fn test_memory_recall_document_by_type_args() {
        let args = json!({
            "memory_type": "fact",
            "limit": 30
        });

        let source_type = args.get("source_type")
            .or_else(|| args.get("memory_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("fact");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);

        assert_eq!(source_type, "fact");
        assert_eq!(limit, 30);
    }

    #[test]
    fn test_memory_recall_document_by_type_fallback() {
        let args = json!({
            "source_type": "preference",
            "limit": 15
        });

        let source_type = args.get("source_type")
            .or_else(|| args.get("memory_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("fact");

        assert_eq!(source_type, "preference");
    }

    #[test]
    fn test_memory_recall_by_task_validation() {
        let args = json!({ "task_id": "task-456" });
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        assert!(task_id.is_some());
        assert_eq!(task_id.unwrap(), "task-456");

        let args = json!({});
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        assert!(task_id.is_none());
    }

    #[test]
    fn test_memory_recall_recent_with_type() {
        let args = json!({
            "limit": 25,
            "type": "experience"
        });

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
        let memory_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("");

        assert_eq!(limit, 25);
        assert_eq!(memory_type, "experience");
    }

    #[test]
    fn test_memory_update_args_fallback() {
        let args = json!({ "id": "mem-789" });
        let memory_id = args.get("id")
            .or_else(|| args.get("memory_id"))
            .and_then(|v| v.as_str());
        assert_eq!(memory_id.unwrap(), "mem-789");

        let args = json!({ "memory_id": "mem-999" });
        let memory_id = args.get("id")
            .or_else(|| args.get("memory_id"))
            .and_then(|v| v.as_str());
        assert_eq!(memory_id.unwrap(), "mem-999");
    }

    #[test]
    fn test_memory_update_missing_id() {
        let args = json!({});
        let memory_id = args.get("id")
            .or_else(|| args.get("memory_id"))
            .and_then(|v| v.as_str());
        assert!(memory_id.is_none());
    }

    #[test]
    fn test_memory_delete_workflow_validation() {
        let args = json!({ "workflow_id": "wf-123" });
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_some());
        assert_eq!(workflow_id.unwrap(), "wf-123");

        let args = json!({});
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(workflow_id.is_none());
    }
}
