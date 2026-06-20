//! Reminder MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

/// Build the `?stuck_task_minutes=` query string for the reminder endpoints
/// from the advertised `stuckTaskMinutes` MCP arg. Returns an empty string when
/// the caller omitted it (legacy behavior — the API route then uses the default
/// 60-minute threshold). Pure so the param-forwarding is unit-testable without
/// hitting the API.
fn stuck_query_param(args: &Value) -> String {
    args.get("stuckTaskMinutes")
        .and_then(|v| v.as_i64())
        .map(|m| format!("?stuck_task_minutes={}", m))
        .unwrap_or_default()
}

pub async fn reminder_check(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let query = stuck_query_param(&args);
    let stale = client::api_get(&format!("/api/reminders/stale{}", query)).await?;
    let stuck = client::api_get(&format!("/api/reminders/stuck{}", query)).await?;
    Ok(serde_json::json!({ "stale": stale, "stuck": stuck }))
}

pub async fn reminder_list(args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Support optional workflow_id filter
    if let Some(wid) = args.get("workflow_id").and_then(|v| v.as_str()) {
        client::api_get(&format!("/api/reminders?workflow_id={}", wid)).await
    } else {
        client::api_get("/api/reminders").await
    }
}

pub async fn reminder_acknowledge(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let reminder_id = args
        .get("id")
        .or_else(|| args.get("reminder_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing id or reminder_id".to_string())?;
    client::api_post(&format!("/api/reminders/{}/acknowledge", reminder_id), args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reminder_acknowledge_args() {
        let args = json!({ "id": "rem-123" });
        let reminder_id = args
            .get("id")
            .or_else(|| args.get("reminder_id"))
            .and_then(|v| v.as_str());
        assert!(reminder_id.is_some());
        assert_eq!(reminder_id.unwrap(), "rem-123");

        let args = json!({ "reminder_id": "rem-456" });
        let reminder_id = args
            .get("id")
            .or_else(|| args.get("reminder_id"))
            .and_then(|v| v.as_str());
        assert!(reminder_id.is_some());
        assert_eq!(reminder_id.unwrap(), "rem-456");

        let args = json!({});
        let reminder_id = args
            .get("id")
            .or_else(|| args.get("reminder_id"))
            .and_then(|v| v.as_str());
        assert!(reminder_id.is_none());
    }

    #[test]
    fn test_reminder_list_with_workflow_filter() {
        let args = json!({ "workflow_id": "wf-123" });
        let wid = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(wid.is_some());
        assert_eq!(wid.unwrap(), "wf-123");

        let args = json!({});
        let wid = args.get("workflow_id").and_then(|v| v.as_str());
        assert!(wid.is_none());
    }

    #[test]
    fn test_stuck_query_param_present() {
        // advertised stuckTaskMinutes is forwarded as a query param.
        assert_eq!(
            stuck_query_param(&json!({ "stuckTaskMinutes": 10 })),
            "?stuck_task_minutes=10"
        );
    }

    #[test]
    fn test_stuck_query_param_absent() {
        // no arg -> empty string -> API uses the default threshold (legacy).
        assert_eq!(stuck_query_param(&json!({})), "");
        assert_eq!(stuck_query_param(&Value::Null), "");
    }
}
