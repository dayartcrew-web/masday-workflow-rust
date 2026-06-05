//! End-to-end test for local_sync → local_push → verify roundtrip
//!
//! This test validates the full roundtrip of:
//! 1. Create workflow and tasks via API
//! 2. Sync to local state via local_sync
//! 3. Modify local state
//! 4. Push back via local_push
//! 5. Verify roundtrip consistency

use masday_mcp::tools::local;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

/// Helper: Create a test workflow via API
async fn create_test_workflow(api_url: &str, api_key: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/workflows", api_url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "name": "e2e-sync-test",
            "project_path": "/home/vibe-dev/masday-workflow-rust"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to create workflow: {}", response.status()).into());
    }

    let result: Value = response.json().await?;
    let workflow_id = result
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing workflow ID in response")?;

    Ok((workflow_id.to_string(), result.to_string()))
}

/// Helper: Add a task to workflow via API
async fn add_test_task(
    api_url: &str,
    api_key: &str,
    workflow_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/workflows/{}/tasks", api_url, workflow_id);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "name": "test-task",
            "agent": "masday-backend",
            "skill": "masday-e2e"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to add task: {}", response.status()).into());
    }

    let result: Value = response.json().await?;
    let task_id = result
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing task ID in response")?;

    Ok(task_id.to_string())
}

/// Helper: Start task (set to RUNNING) via API
async fn start_task(
    api_url: &str,
    api_key: &str,
    workflow_id: &str,
    task_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/tasks/{}/start", api_url, task_id);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({ "workflow_id": workflow_id }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to start task: {} - {}", status_code, error_text).into());
    }

    Ok(())
}

/// Helper: Get workflow from API
async fn get_workflow_from_api(
    api_url: &str,
    api_key: &str,
    workflow_id: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/workflows/{}", api_url, workflow_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get workflow: {}", response.status()).into());
    }

    let result: Value = response.json().await?;
    Ok(result)
}

/// Helper: Get tasks from API
async fn get_tasks_from_api(
    api_url: &str,
    api_key: &str,
    workflow_id: &str,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/workflows/{}/tasks", api_url, workflow_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get tasks: {}", response.status()).into());
    }

    let result: Value = response.json().await?;

    // API returns tasks as array directly or wrapped in "tasks" field
    let tasks = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or("Missing tasks array in response")?
            .to_vec()
    };

    Ok(tasks)
}

/// Helper: Delete workflow via API
async fn delete_workflow(
    api_url: &str,
    api_key: &str,
    workflow_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("{}/api/workflows/{}", api_url, workflow_id);

    let response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to delete workflow: {}", response.status()).into());
    }

    Ok(())
}

/// Helper: Read local state file
async fn read_local_state_file(cwd: &str, workflow_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let state_file = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows")
        .join(format!("{}.json", workflow_id));

    if !state_file.exists() {
        return Err(format!("State file does not exist: {:?}", state_file).into());
    }

    let content = tokio::fs::read_to_string(&state_file).await?;
    let parsed: Value = serde_json::from_str(&content)?;
    Ok(parsed)
}

/// Helper: Write local state file
async fn write_local_state_file(
    cwd: &str,
    workflow_id: &str,
    state: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let state_file = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows")
        .join(format!("{}.json", workflow_id));

    let state_json = serde_json::to_string_pretty(state)?;
    tokio::fs::write(&state_file, state_json).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Run with: cargo test -p masday-mcp --test e2e_local_sync_push -- --ignored
async fn test_e2e_local_sync_push_roundtrip() {
    // Setup
    let api_url = masday_core::constants::ports::api_base_url();
    let api_key = std::env::var("MASDAY_API_KEY").unwrap_or_default().to_string(); // From .env
    let cwd = "/home/vibe-dev/masday-workflow-rust";

    // Initialize client module
    masday_mcp::client::init(api_url.clone(), api_key.clone()).unwrap();

    // Ensure .masday directory structure exists
    let state_dir = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows");
    tokio::fs::create_dir_all(&state_dir)
        .await
        .expect("Failed to create state directory");

    // Step 1: Create test workflow
    println!("Step 1: Creating test workflow...");
    let (workflow_id, _workflow_json) = create_test_workflow(&api_url, &api_key)
        .await
        .expect("Failed to create workflow");
    println!("Created workflow: {}", workflow_id);

    // Step 2: Add task
    println!("Step 2: Adding test task...");
    let task_id = add_test_task(&api_url, &api_key, &workflow_id)
        .await
        .expect("Failed to add task");
    println!("Created task: {}", task_id);

    // Step 3: Start task (set to RUNNING status - required before completing)
    println!("Step 3: Starting task (set to RUNNING)...");
    start_task(&api_url, &api_key, &workflow_id, &task_id)
        .await
        .expect("Failed to start task");
    println!("Task started (status: RUNNING)");

    // Small delay to ensure DB consistency
    sleep(Duration::from_millis(100)).await;

    // Step 4: Test local_sync (PostgreSQL → local file)
    println!("Step 4: Testing local_sync...");
    let sync_args = json!({
        "cwd": cwd,
        "workflow_id": &workflow_id
    });

    let sync_result = local::local_sync(sync_args)
        .await
        .expect("local_sync failed");
    println!("local_sync result: {}", sync_result);

    // Verify state file exists
    let local_state = read_local_state_file(cwd, &workflow_id)
        .await
        .expect("Failed to read local state file");
    println!("Local state file read successfully");

    // Verify workflow data in local state
    assert_eq!(
        local_state
            .get("workflow")
            .and_then(|w| w.get("id"))
            .and_then(|v| v.as_str()),
        Some(workflow_id.as_str()),
        "Workflow ID mismatch in local state"
    );

    // Verify tasks data in local state
    let tasks = local_state
        .get("tasks")
        .and_then(|v| v.as_array())
        .expect("Missing tasks array in local state");
    assert_eq!(tasks.len(), 1, "Expected 1 task in local state");
    assert_eq!(
        tasks[0].get("id").and_then(|v| v.as_str()),
        Some(task_id.as_str()),
        "Task ID mismatch in local state"
    );

    println!("Step 4: local_sync verified ✓");

    // Step 5: Modify state file (simulating local changes)
    println!("Step 5: Modifying local state...");
    let mut modified_state = local_state.clone();

    // Change task status
    if let Some(tasks) = modified_state.get_mut("tasks").and_then(|v| v.as_array_mut()) {
        if let Some(task) = tasks.get_mut(0) {
            if let Some(status) = task.get_mut("status") {
                *status = json!("DONE");
                println!("Task status changed to DONE");
            }

            // Add output field
            task["output"] = json!("Test output from local modification");
            task["result"] = json!("Test result from local modification");
            println!("Added output and result fields");
        }
    }

    write_local_state_file(cwd, &workflow_id, &modified_state)
        .await
        .expect("Failed to write modified state file");
    println!("Step 5: Local state modified ✓");

    // Step 6: Test local_push (local file → PostgreSQL)
    println!("Step 6: Testing local_push...");
    let push_args = json!({
        "cwd": cwd,
        "workflow_id": &workflow_id
    });

    let push_result = local::local_push(push_args)
        .await
        .expect("local_push failed");
    println!("local_push result: {}", push_result);

    // Verify push was successful
    assert_eq!(
        push_result.get("pushed").and_then(|v| v.as_bool()),
        Some(true),
        "local_push should return pushed=true"
    );

    let pushed_workflows = push_result
        .get("workflows_pushed")
        .and_then(|v| v.as_array())
        .expect("Missing workflows_pushed in result");
    assert_eq!(pushed_workflows.len(), 1, "Expected 1 workflow pushed");
    assert_eq!(
        pushed_workflows[0].as_str(),
        Some(workflow_id.as_str()),
        "Pushed workflow ID mismatch"
    );

    println!("Step 6: local_push verified ✓");

    // Small delay to ensure DB consistency
    sleep(Duration::from_millis(100)).await;

    // Step 7: Verify roundtrip - compare API data with local modifications
    println!("Step 7: Verifying roundtrip...");

    let api_workflow = get_workflow_from_api(&api_url, &api_key, &workflow_id)
        .await
        .expect("Failed to get workflow from API");
    let api_tasks = get_tasks_from_api(&api_url, &api_key, &workflow_id)
        .await
        .expect("Failed to get tasks from API");

    println!("API workflow: {}", api_workflow);
    println!("API tasks: {}", api_tasks.len());

    // Verify task status was updated
    assert_eq!(api_tasks.len(), 1, "Expected 1 task from API");
    assert_eq!(
        api_tasks[0].get("status").and_then(|v| v.as_str()),
        Some("DONE"),
        "Task status should be DONE after push"
    );

    // Verify result was pushed (stored in test_evidence field by API)
    assert_eq!(
        api_tasks[0]
            .get("test_evidence")
            .and_then(|v| v.as_str()),
        Some("Test result from local modification"),
        "Task result mismatch after push (stored in test_evidence field)"
    );

    println!("Step 7: Roundtrip verified ✓");

    // Step 8: Cleanup
    println!("Step 8: Cleaning up...");

    // Delete workflow from API
    delete_workflow(&api_url, &api_key, &workflow_id)
        .await
        .expect("Failed to delete workflow");
    println!("Workflow deleted from API");

    // Delete local state file
    let state_file = std::path::Path::new(cwd)
        .join(".masday")
        .join("state")
        .join("workflows")
        .join(format!("{}.json", workflow_id));

    tokio::fs::remove_file(&state_file)
        .await
        .expect("Failed to delete state file");
    println!("Local state file deleted");

    println!("Step 8: Cleanup complete ✓");
    println!("\n✅ E2E test PASSED: local_sync → local_push → verify roundtrip");
}

#[tokio::test]
#[ignore] // Run with: cargo test -p masday-mcp --test e2e_local_sync_push -- --ignored
async fn test_local_sync_invalid_workflow_id() {
    // Setup
    let api_url = masday_core::constants::ports::api_base_url();
    let api_key = std::env::var("MASDAY_API_KEY").unwrap_or_default().to_string();
    let cwd = "/home/vibe-dev/masday-workflow-rust";

    // Initialize client module (may already be initialized from other tests)
    let _ = masday_mcp::client::init(api_url.clone(), api_key.clone());

    // Test with invalid workflow ID (path traversal attempt)
    let sync_args = json!({
        "cwd": cwd,
        "workflow_id": "../etc/passwd"
    });

    let result = local::local_sync(sync_args).await;

    assert!(result.is_err(), "local_sync should reject invalid workflow ID");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid workflow_id"),
        "Error should mention invalid workflow_id");
}

#[tokio::test]
#[ignore] // Run with: cargo test -p masday-mcp --test e2e_local_sync_push -- --ignored
async fn test_local_push_nonexistent_workflow() {
    // Setup
    let api_url = masday_core::constants::ports::api_base_url();
    let api_key = std::env::var("MASDAY_API_KEY").unwrap_or_default().to_string();
    let cwd = "/home/vibe-dev/masday-workflow-rust";

    // Initialize client module (may already be initialized from other tests)
    let _ = masday_mcp::client::init(api_url.clone(), api_key.clone());

    // Test with nonexistent workflow ID
    let push_args = json!({
        "cwd": cwd,
        "workflow_id": "nonexistent-workflow-id"
    });

    let result = local::local_push(push_args).await;

    // Should succeed but return error about file not found
    assert!(result.is_ok(), "local_push should not fail for missing file");
    let result_json = result.unwrap();
    assert_eq!(
        result_json.get("pushed").and_then(|v| v.as_bool()),
        Some(false),
        "local_push should return pushed=false for missing file"
    );
    assert!(
        result_json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("Workflow file not found"),
        "Error should mention file not found"
    );
}
