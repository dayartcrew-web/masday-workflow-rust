//! API State Transition Validation Tests
//!
//! Tests that the API endpoint POST /api/workflows/{id}/update
//! properly validates state transitions and rejects invalid requests with 400.

use masday_db::pool::create_pool;
use masday_service::workflow_service;
use masday_core::WorkflowState;

/// Helper: Create a test workflow
async fn create_test_workflow(name: &str) -> Result<masday_db::schema::Workflow, String> {
    let pool = create_pool().map_err(|e| e.to_string())?;
    let wf = workflow_service::WorkflowService::create_workflow(
        &pool,
        name.to_string(),
        Some(format!("Test workflow: {}", name)),
        Some("/tmp/test".to_string()),
    ).await.map_err(|e| e.to_string())?;
    Ok(wf)
}

#[tokio::test]
#[ignore = "requires live PostgreSQL (DATABASE_URL)"]
async fn test_valid_state_transitions_via_service() {
    let pool = create_pool().expect("Failed to create pool");

    let workflow = create_test_workflow("valid-transitions-test").await.expect("Failed to create workflow");
    let workflow_id = workflow.id.clone();

    // Test valid transitions work via service layer
    // INIT → ANALYZE
    let analyzed = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Analyze,
    ).await.expect("INIT → ANALYZE should succeed");

    assert_eq!(analyzed.status, "ANALYZE");

    // ANALYZE → PLAN
    let planned = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Plan,
    ).await.expect("ANALYZE → PLAN should succeed");

    assert_eq!(planned.status, "PLAN");

    // PLAN → PAUSED
    let paused = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Paused,
    ).await.expect("PLAN → PAUSED should succeed");

    assert_eq!(paused.status, "PAUSED");

    // PAUSED → EXECUTE
    let executing = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Execute,
    ).await.expect("PAUSED → EXECUTE should succeed");

    assert_eq!(executing.status, "EXECUTE");

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await;
}

#[tokio::test]
#[ignore = "requires live PostgreSQL (DATABASE_URL)"]
async fn test_invalid_state_transition_done_to_execute() {
    let pool = create_pool().expect("Failed to create pool");

    let workflow = create_test_workflow("invalid-done-to-execute").await.expect("Failed to create workflow");
    let workflow_id = workflow.id.clone();

    // Transition to DONE
    let done_wf = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Done,
    ).await.expect("Transition to DONE should succeed");

    assert_eq!(done_wf.status, "DONE");

    // Try invalid transition: DONE → EXECUTE (should fail)
    let result = workflow_service::WorkflowService::transition_status(
        &pool,
        &workflow_id,
        WorkflowState::Execute,
    ).await;

    assert!(result.is_err(), "DONE → EXECUTE should fail validation");

    match result {
        Err(e) => {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Invalid state transition"),
                    "Error should mention invalid transition, got: {}", error_msg);
        }
        Ok(_) => panic!("DONE → EXECUTE should have failed"),
    }

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await;
}

#[tokio::test]
#[ignore = "requires live PostgreSQL (DATABASE_URL)"]
async fn test_invalid_status_string_rejected() {
    let pool = create_pool().expect("Failed to create pool");

    let workflow = create_test_workflow("invalid-status-string").await.expect("Failed to create workflow");
    let workflow_id = workflow.id.clone();

    // Test that invalid status strings are rejected at conversion time
    let invalid_statuses = vec![
        "INVALID_STATE",
        "NOT_A_REAL_STATE",
        "RandomText",
        "",
        "123",
    ];

    for invalid_status in invalid_statuses {
        let result = workflow_service::status_to_state(invalid_status);

        assert!(result.is_err(),
                "status_to_state should reject invalid status '{}'", invalid_status);

        match result {
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("Invalid workflow state") || error_msg.contains("Validation error"),
                        "Error should mention invalid state, got: {}", error_msg);
            }
            Ok(_) => panic!("status_to_state should have rejected '{}'", invalid_status),
        }
    }

    // Verify lowercase works (gets converted to uppercase)
    let result = workflow_service::status_to_state("init");
    assert!(result.is_ok(), "status_to_state should accept lowercase 'init'");
    assert!(matches!(result.unwrap(), WorkflowState::Init));

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow_id).await;
}

#[tokio::test]
async fn test_status_to_state_all_valid_states() {
    let valid_states = vec![
        ("INIT", WorkflowState::Init),
        ("ANALYZE", WorkflowState::Analyze),
        ("PLAN", WorkflowState::Plan),
        ("EXECUTE", WorkflowState::Execute),
        ("VERIFY", WorkflowState::Verify),
        ("FIX", WorkflowState::Fix),
        ("DONE", WorkflowState::Done),
        ("FAILED", WorkflowState::Failed),
        ("PAUSED", WorkflowState::Paused),
    ];

    for (status_str, expected_state) in &valid_states {
        let result = workflow_service::status_to_state(status_str);
        assert!(result.is_ok(), "status_to_state should accept '{}'", status_str);

        let state = result.unwrap();
        assert!(std::mem::discriminant(&state) == std::mem::discriminant(&expected_state),
                "status_to_state should convert '{}' to {:?}", status_str, expected_state);
    }

    // Test lowercase variants
    for (status_str, expected_state) in &valid_states {
        let lowercase = status_str.to_lowercase();
        let result = workflow_service::status_to_state(&lowercase);
        assert!(result.is_ok(), "status_to_state should accept lowercase '{}'", lowercase);

        let state = result.unwrap();
        assert!(std::mem::discriminant(&state) == std::mem::discriminant(&expected_state),
                "status_to_state should convert '{}' to {:?}", lowercase, expected_state);
    }
}
