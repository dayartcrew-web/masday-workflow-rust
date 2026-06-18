//! Cross-domain integration tests
//!
//! Tests require live PostgreSQL on port 54341.
//! All tests use #[ignore] — run with:
//! ```sh
//! DATABASE_URL="postgresql://masday:masday_dev_password@localhost:54341/masday_workflow" \
//!   cargo test -p masday-service --test integration_cross_domain -- --ignored --nocapture
//! ```

use masday_db::pool::{create_pool, DbPool};
use masday_service::memory_service::{MemoryService, StoreMemoryParams};
use masday_service::plan_service::PlanService;
use masday_service::review_service::ReviewService;
use masday_service::task_service::TaskService;
use masday_service::workflow_service::WorkflowService;

fn get_pool() -> DbPool {
    create_pool().expect("Failed to create pool — is PostgreSQL running on port 54341?")
}

async fn setup_workflow(pool: &DbPool) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let wf = WorkflowService::create_workflow(
        pool,
        "cross-domain-test".to_string(),
        Some("Cross-domain test".to_string()),
        Some("/tmp/test".to_string()),
    )
    .await?;
    Ok(wf.id)
}

/// Test 1: Cross-domain workflow → task → plan → complete lifecycle
#[tokio::test]
#[ignore]
async fn test_cross_domain_workflow_task_plan_lifecycle(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = get_pool();
    let workflow_id = setup_workflow(&pool).await?;

    // Create plan
    let plan = PlanService::create_plan(
        &pool,
        workflow_id.clone(),
        serde_json::json!({"phases": [{"name": "setup"}, {"name": "execute"}]}),
    )
    .await?;
    let plan_id = plan.id;

    // Add task
    let task = TaskService::add_task(
        &pool,
        workflow_id.clone(),
        plan_id,
        "Cross-domain test task".to_string(),
        Some("test-agent".to_string()),
        None,
        None,
    )
    .await?;
    let task_id = task.id;
    assert_eq!(task.status, "PENDING");

    // Start task
    let started = TaskService::start_task(&pool, &workflow_id, &task_id).await?;
    assert_eq!(started.status, "RUNNING");

    // Complete task
    let completed = TaskService::complete_task(
        &pool,
        &workflow_id,
        &task_id,
        Some(serde_json::json!("Task completed successfully")),
    )
    .await?;
    assert_eq!(completed.status, "DONE");

    // Verify workflow still accessible
    let wf = WorkflowService::get_workflow(&pool, &workflow_id).await?;
    assert_eq!(wf.id, workflow_id);

    // Cleanup
    let _ = WorkflowService::delete_workflow(&pool, &workflow_id).await;
    Ok(())
}

/// Test 2: Cross-domain memory store → search → recall → update → delete
#[tokio::test]
#[ignore]
async fn test_cross_domain_memory_store_search_delete(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = get_pool();

    // Store memory
    let stored = MemoryService::store(
        &pool,
        StoreMemoryParams {
            memory_type: "fact",
            summary: "Cross-domain test memory unique-ax7b",
            content: "Test content for cross-domain search",
            created_by: "test-agent",
            importance: 0.7,
            tags: vec!["test".to_string(), "cross-domain".to_string()],
            workflow_id: None,
            task_id: None,
        },
    )
    .await?;

    let memory_id = stored["id"].as_str().unwrap().to_string();
    assert_eq!(stored["memory_type"], "fact");

    // Search
    let results = MemoryService::search(&pool, "unique-ax7b", 10).await?;
    assert!(!results.is_empty());

    // Recall recent
    let recent = MemoryService::recall_recent(&pool, 10).await?;
    assert!(!recent.is_empty());

    // Update
    let updated =
        MemoryService::update(&pool, &memory_id, Some("Updated content"), Some(0.9)).await?;
    assert_eq!(updated["id"], memory_id);

    // Delete
    MemoryService::delete(&pool, &memory_id).await?;
    Ok(())
}

/// Test 3: Cross-domain review after task completion
#[tokio::test]
#[ignore]
async fn test_cross_domain_review_after_task(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = get_pool();
    let workflow_id = setup_workflow(&pool).await?;

    // Create plan
    let plan = PlanService::create_plan(&pool, workflow_id.clone(), serde_json::json!({})).await?;

    // Create and complete task
    let task = TaskService::add_task(
        &pool,
        workflow_id.clone(),
        plan.id,
        "Review test task".to_string(),
        Some("test-agent".to_string()),
        None,
        None,
    )
    .await?;
    let task_id = task.id;

    TaskService::start_task(&pool, &workflow_id, &task_id).await?;
    TaskService::complete_task(&pool, &workflow_id, &task_id, None).await?;

    // Submit review
    let review = ReviewService::submit_review(
        &pool,
        workflow_id.clone(),
        task_id.clone(),
        "reviewer-agent".to_string(),
        "APPROVED".to_string(),
        "All checks passed".to_string(),
        None,
    )
    .await?;
    assert_eq!(review.decision, "APPROVED");
    assert_eq!(review.task_id, task_id);

    // Get latest review
    let latest = ReviewService::get_latest_review(&pool, &workflow_id, &task_id).await?;
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().decision, "APPROVED");

    // Cleanup
    let _ = WorkflowService::delete_workflow(&pool, &workflow_id).await;
    Ok(())
}

/// Test 4: Cross-domain memory with workflow context
#[tokio::test]
#[ignore]
async fn test_cross_domain_memory_with_workflow(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = get_pool();
    let workflow_id = setup_workflow(&pool).await?;

    // Store memory linked to workflow
    let stored = MemoryService::store(
        &pool,
        StoreMemoryParams {
            memory_type: "experience",
            summary: "Workflow execution note",
            content: "Completed phase 1 successfully",
            created_by: "masday-executor",
            importance: 0.8,
            tags: vec!["workflow".to_string()],
            workflow_id: Some(&workflow_id),
            task_id: None,
        },
    )
    .await?;
    assert_eq!(stored["memory_type"], "experience");

    // Recall by task (should find nothing for random task)
    let by_task = MemoryService::recall_by_task(&pool, "nonexistent-task", 10).await?;
    assert!(by_task.is_empty());

    // Stats should reflect the stored memory
    let stats = MemoryService::stats(&pool).await?;
    assert!(stats["total_count"].as_i64().unwrap() > 0);

    // Cleanup
    let _ = WorkflowService::delete_workflow(&pool, &workflow_id).await;
    Ok(())
}
