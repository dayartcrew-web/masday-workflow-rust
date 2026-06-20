//! Reminder MCP tools - HTTP client calls to API

use crate::client;
use serde_json::Value;

/// Build the query string for the reminder endpoints from the advertised MCP
/// args. Forwards `staleExecutionMinutes` → `stale_execution_minutes`,
/// `stuckTaskMinutes` → `stuck_task_minutes`, and `includeFailed` →
/// `include_failed`. Returns an empty string when none is supplied (legacy
/// behavior — the API route then uses the defaults). Pure so the
/// param-forwarding is unit-testable without hitting the API.
fn reminder_query_string(args: &Value) -> String {
    let mut params: Vec<String> = Vec::new();
    if let Some(m) = args.get("staleExecutionMinutes").and_then(|v| v.as_i64()) {
        params.push(format!("stale_execution_minutes={}", m));
    }
    if let Some(m) = args.get("stuckTaskMinutes").and_then(|v| v.as_i64()) {
        params.push(format!("stuck_task_minutes={}", m));
    }
    if args.get("includeFailed").and_then(|v| v.as_bool()) == Some(true) {
        params.push("include_failed=true".to_string());
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

pub async fn reminder_check(
    args: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let query = reminder_query_string(&args);
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
    fn test_reminder_query_string_stuck_only() {
        // advertised stuckTaskMinutes is forwarded as a query param.
        assert_eq!(
            reminder_query_string(&json!({ "stuckTaskMinutes": 10 })),
            "?stuck_task_minutes=10"
        );
    }

    #[test]
    fn test_reminder_query_string_stale_only() {
        // advertised staleExecutionMinutes is forwarded as a query param.
        assert_eq!(
            reminder_query_string(&json!({ "staleExecutionMinutes": 180 })),
            "?stale_execution_minutes=180"
        );
    }

    #[test]
    fn test_reminder_query_string_include_failed() {
        // includeFailed=true forwarded; false/absent omitted.
        assert_eq!(
            reminder_query_string(&json!({ "includeFailed": true })),
            "?include_failed=true"
        );
        assert_eq!(
            reminder_query_string(&json!({ "includeFailed": false })),
            ""
        );
    }

    #[test]
    fn test_reminder_query_string_both_params() {
        // both params combine with stable ordering (stuck first).
        assert_eq!(
            reminder_query_string(&json!({ "stuckTaskMinutes": 10, "includeFailed": true })),
            "?stuck_task_minutes=10&include_failed=true"
        );
    }

    #[test]
    fn test_reminder_query_string_all_three_params() {
        // stale, stuck, include combine with stable ordering matching the
        // advertised schema (stale, stuck, include).
        assert_eq!(
            reminder_query_string(&json!({
                "staleExecutionMinutes": 180,
                "stuckTaskMinutes": 10,
                "includeFailed": true
            })),
            "?stale_execution_minutes=180&stuck_task_minutes=10&include_failed=true"
        );
    }

    #[test]
    fn test_reminder_query_string_absent() {
        // no args -> empty string -> API uses defaults (legacy).
        assert_eq!(reminder_query_string(&json!({})), "");
        assert_eq!(reminder_query_string(&Value::Null), "");
    }
}
