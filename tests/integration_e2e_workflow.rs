//! End-to-end integration test for workflow lifecycle
//!
//! Tests: create workflow → add task → start task → complete task → verify status transitions

use masday_db::pool::create_pool;
use masday_service::workflow_service;
use masday_service::task_service;
use masday_core::{WorkflowState, TaskState};

/// Helper: Create a test workflow
async fn create_test_workflow(name: &str) -> Result<masday_db::schema::Workflow, String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;
    let wf = workflow_service::WorkflowService::create_workflow(
        &pool,
        name.to_string(),
        Some(format!("Test workflow: {}", name)),
        Some("/tmp/test".to_string()),
    ).await.map_err(|e| e.to_string())?;
    Ok(wf)
}

#[tokio::test]
async fn test_e2e_workflow_lifecycle() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Step 1: Create workflow (status should be INIT)
    let workflow = workflow_service::WorkflowService::create_workflow(
        &pool,
        "e2e-test-workflow".to_string(),
        Some("End-to-end test workflow".to_string()),
        Some("/tmp/test".to_string()),
    ).await.map_err(|e| format!("Create workflow failed: {}", e))?;

    assert_eq!(workflow.status, "INIT");
    let workflow_id = workflow.id.clone();

    // Step 2: Create a plan for the workflow
    let phases = serde_json::json!({
        "phases": [
            {"name": "setup", "tasks": ["task1"]},
            {"name": "execute", "tasks": ["task2"]}
        ]
    });

    let plan = masday_service::plan_service::PlanService::create_plan(
        &pool,
        workflow_id.clone(),
        phases,
    ).await.map_err(|e| format!("Create plan failed: {}", e))?;

    let plan_id = plan.id.clone();

    // Step 3: Add task to workflow
    let task = task_service::TaskService::add_task(
        &pool,
        workflow_id.clone(),
        plan_id.clone(),
        "Test task".to_string(),
        Some("test-agent".to_string()),
        None,
    ).await.map_err(|e| format!("Add task failed: {}", e))?;

    assert_eq!(task.status, "PENDING");
    assert_eq!(task.workflow_id, workflow_id);
    let task_id = task.id.clone();

    // Step 4: Start task (status should be RUNNING)
    let started_task = task_service::TaskService::start_task(&pool, &workflow_id, &task_id)
        .await.map_err(|e| format!("Start task failed: {}", e))?;

    assert_eq!(started_task.status, "RUNNING");

    // Step 5: Complete task (status should be DONE)
    let completed_task = task_service::TaskService::complete_task(&pool, &workflow_id, &task_id, Some("Task completed successfully".to_string()))
        .await.map_err(|e| format!("Complete task failed: {}", e))?;

    assert_eq!(completed_task.status, "DONE");

    // Step 5: Transition workflow from INIT to EXECUTE (requires proper transition)
    let executing_workflow = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Execute,
    ).await.map_err(|e| format!("Transition to EXECUTE failed: {}", e))?;

    assert_eq!(executing_workflow.status, "EXECUTE");

    // Step 6: Transition workflow to VERIFY
    let verifying_workflow = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Verify,
    ).await.map_err(|e| format!("Transition to VERIFY failed: {}", e))?;

    assert_eq!(verifying_workflow.status, "VERIFY");

    // Step 7: Transition workflow to DONE
    let done_workflow = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Done,
    ).await.map_err(|e| format!("Transition to DONE failed: {}", e))?;

    assert_eq!(done_workflow.status, "DONE");

    // Cleanup
    workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await
        .map_err(|e| format!("Delete workflow failed: {}", e))?;

    Ok(())
}

#[tokio::test]
async fn test_workflow_state_transitions() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    let workflow = create_test_workflow("transition-test").await?;
    let workflow_id = workflow.id.clone();

    // Test valid transition: INIT → ANALYZE
    let analyzed = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Analyze,
    ).await.map_err(|e| format!("INIT → ANALYZE failed: {}", e))?;

    assert_eq!(analyzed.status, "ANALYZE");

    // Test valid transition: ANALYZE → PLAN
    let planned = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Plan,
    ).await.map_err(|e| format!("ANALYZE → PLAN failed: {}", e))?;

    assert_eq!(planned.status, "PLAN");

    // Test valid transition: PLAN → PAUSED
    let paused = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Paused,
    ).await.map_err(|e| format!("PLAN → PAUSED failed: {}", e))?;

    assert_eq!(paused.status, "PAUSED");

    // Test valid transition: PAUSED → EXECUTE
    let executing = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Execute,
    ).await.map_err(|e| format!("PAUSED → EXECUTE failed: {}", e))?;

    assert_eq!(executing.status, "EXECUTE");

    // Test valid transition: EXECUTE → FIX
    let fixing = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Fix,
    ).await.map_err(|e| format!("EXECUTE → FIX failed: {}", e))?;

    assert_eq!(fixing.status, "FIX");

    // Test valid transition: FIX → DONE
    let done = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Done,
    ).await.map_err(|e| format!("FIX → DONE failed: {}", e))?;

    assert_eq!(done.status, "DONE");

    // Cleanup
    workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_invalid_workflow_transition() {
    let pool = create_pool().await.expect("Failed to create pool");

    let workflow = create_test_workflow("invalid-transition-test").await.expect("Failed to create workflow");
    let workflow_id = workflow.id.clone();

    // First transition to DONE
    let done_wf = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Done,
    ).await.expect("Transition to DONE should succeed");

    assert_eq!(done_wf.status, "DONE");

    // Now try invalid transition: DONE → EXECUTE (should fail)
    let result = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Execute,
    ).await;

    assert!(result.is_err(), "DONE → EXECUTE should be invalid");

    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid state transition"));
    }

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await;
}

#[tokio::test]
async fn test_task_lifecycle() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    let workflow = create_test_workflow("task-lifecycle-test").await?;
    let workflow_id = workflow.id.clone();

    // Create plan
    let phases = serde_json::json!({"phases": []});
    let plan = masday_service::plan_service::PlanService::create_plan(
        &pool,
        workflow_id.clone(),
        phases,
    ).await?;
    let plan_id = plan.id.clone();

    // Add task
    let task = task_service::TaskService::add_task(
        &pool,
        workflow_id.clone(),
        plan_id.clone(),
        "Lifecycle test task".to_string(),
        Some("test-agent".to_string()),
        None,
    ).await?;

    assert_eq!(task.status, "PENDING");
    let task_id = task.id.clone();

    // Start task
    let running = task_service::TaskService::start_task(&pool, &workflow_id, &task_id).await?;
    assert_eq!(running.status, "RUNNING");

    // Complete task
    let done = task_service::TaskService::complete_task(&pool, &workflow_id, &task_id, Some(serde_json::json!("All done"))).await?;
    assert_eq!(done.status, "DONE");

    // Cleanup
    workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_task_failure() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    let workflow = create_test_workflow("task-failure-test").await?;
    let workflow_id = workflow.id.clone();

    // Create plan
    let phases = serde_json::json!({"phases": []});
    let plan = masday_service::plan_service::PlanService::create_plan(
        &pool,
        workflow_id.clone(),
        phases,
    ).await?;
    let plan_id = plan.id.clone();

    let task = task_service::TaskService::add_task(
        &pool,
        workflow_id.clone(),
        plan_id,
        "Failing task".to_string(),
        Some("test-agent".to_string()),
        None,
    ).await?;

    let task_id = task.id.clone();

    // Start task
    task_service::TaskService::start_task(&pool, &workflow_id, &task_id).await?;

    // Note: TaskService doesn't have a fail_task method in the current implementation
    // We'll just verify the RUNNING state
    let running_task = task_service::TaskService::get_current_task(&pool, &workflow_id).await?;
    assert!(running_task.is_some());
    assert_eq!(running_task.unwrap().status, "RUNNING");

    // Cleanup
    workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await?;

    Ok(())
}
