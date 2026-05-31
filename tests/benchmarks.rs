//! Performance benchmarks for the masday workflow system
//!
//! Targets:
//! - Cold start <500ms
//! - Tool call latency <20ms
//! - Memory <30MB

use masday_db::pool::create_pool;
use masday_service::{workflow_service, memory_service, task_service};
use std::time::Instant;

/// Benchmark: Cold start (creating first workflow)
#[tokio::test]
async fn benchmark_cold_start() {
    let start = Instant::now();

    let pool = create_pool().await.expect("Failed to create pool");

    let workflow = workflow_service::WorkflowService::create_workflow(
        &pool,
        "cold-start-test".to_string(),
        Some("Testing cold start".to_string()),
        Some("/tmp/test".to_string()),
    ).await.expect("Failed to create workflow");

    let elapsed = start.elapsed();

    println!("Cold start time: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 500,
        "Cold start should be <500ms, got {}ms",
        elapsed.as_millis()
    );

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow.id).await;
}

/// Benchmark: Tool call latency (workflow creation)
#[tokio::test]
async fn benchmark_workflow_creation_latency() {
    let pool = create_pool().await.expect("Failed to create pool");

    let iterations = 100;
    let mut total_time = 0;

    for i in 0..iterations {
        let start = Instant::now();

        let workflow = workflow_service::WorkflowService::create_workflow(
            &pool,
            format!("latency-test-{}", i),
            Some("Testing latency".to_string()),
            Some("/tmp/test".to_string()),
        ).await.expect("Failed to create workflow");

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();

        // Cleanup
        let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow.id).await;
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ms = avg_time_us as f64 / 1000.0;

    println!("Average workflow creation time: {:.2}ms", avg_time_ms);
    assert!(
        avg_time_ms < 20.0,
        "Average workflow creation should be <20ms, got {:.2}ms",
        avg_time_ms
    );
}

/// Benchmark: Memory store latency
#[tokio::test]
async fn benchmark_memory_store_latency() {
    let pool = create_pool().await.expect("Failed to create pool");

    let iterations = 100;
    let mut total_time = 0;

    for i in 0..iterations {
        let start = Instant::now();

        let _ = memory_service::MemoryService::store(
            &pool,
            memory_service::StoreMemoryParams {
                memory_type: "fact",
                summary: &format!("Test fact {}", i),
                content: &format!("Content for fact {}", i),
                created_by: "test-agent",
                importance: 0.7,
                tags: vec!["benchmark".to_string()],
                workflow_id: None,
                task_id: None,
            },
        ).await.expect("Failed to store memory");

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ms = avg_time_us as f64 / 1000.0;

    println!("Average memory store time: {:.2}ms", avg_time_ms);
    assert!(
        avg_time_ms < 20.0,
        "Average memory store should be <20ms, got {:.2}ms",
        avg_time_ms
    );
}

/// Benchmark: Memory search latency
#[tokio::test]
async fn benchmark_memory_search_latency() {
    let pool = create_pool().await.expect("Failed to create pool");

    // Seed some memories
    for i in 0..50 {
        memory_service::MemoryService::store(
            &pool,
            memory_service::StoreMemoryParams {
                memory_type: "fact",
                summary: &format!("Test fact {}", i),
                content: &format!("Content about topic {}", i % 5),
                created_by: "test-agent",
                importance: 0.7,
                tags: vec![format!("topic-{}", i % 5)],
                workflow_id: None,
                task_id: None,
            },
        ).await.expect("Failed to store memory");
    }

    let iterations = 100;
    let mut total_time = 0;

    for i in 0..iterations {
        let start = Instant::now();

        let _ = memory_service::MemoryService::search(&pool, &format!("topic-{}", i % 5), 10)
            .await.expect("Failed to search memory");

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ms = avg_time_us as f64 / 1000.0;

    println!("Average memory search time: {:.2}ms", avg_time_ms);
    assert!(
        avg_time_ms < 20.0,
        "Average memory search should be <20ms, got {:.2}ms",
        avg_time_ms
    );
}

/// Benchmark: Working memory latency
#[tokio::test]
async fn benchmark_working_memory_latency() {
    let session_id = "benchmark-session";

    let iterations = 1000;
    let mut total_time = 0;

    for i in 0..iterations {
        let start = Instant::now();

        let msg = serde_json::json!({"content": format!("Message {}", i)});
        memory_service::MemoryService::working_store(session_id, msg);

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ns = avg_time_us as f64 * 1000.0;

    println!("Average working memory store time: {:.2}ns", avg_time_ns);
    assert!(
        avg_time_ns < 20_000.0,
        "Average working memory store should be <20μs, got {:.2}ns",
        avg_time_ns
    );
}

/// Benchmark: Task creation latency
#[tokio::test]
async fn benchmark_task_creation_latency() {
    let pool = create_pool().await.expect("Failed to create pool");

    let workflow = workflow_service::WorkflowService::create_workflow(
        &pool,
        "task-benchmark-test".to_string(),
        Some("Testing task latency".to_string()),
        Some("/tmp/test".to_string()),
    ).await.expect("Failed to create workflow");

    // Create plan for tasks
    let phases = serde_json::json!({"phases": []});
    let plan = masday_service::plan_service::PlanService::create_plan(
        &pool,
        workflow.id.clone(),
        phases,
    ).await.expect("Failed to create plan");

    let iterations = 100;
    let mut total_time = 0;

    for i in 0..iterations {
        let start = Instant::now();

        let task = task_service::TaskService::add_task(
            &pool,
            workflow.id.clone(),
            plan.id.clone(),
            format!("Task {}", i),
            Some(format!("agent-{}", i % 3)),
            None,
        ).await.expect("Failed to add task");

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ms = avg_time_us as f64 / 1000.0;

    println!("Average task creation time: {:.2}ms", avg_time_ms);
    assert!(
        avg_time_ms < 20.0,
        "Average task creation should be <20ms, got {:.2}ms",
        avg_time_ms
    );

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow.id).await;
}

/// Benchmark: Workflow state transition latency
#[tokio::test]
async fn benchmark_state_transition_latency() {
    use masday_core::WorkflowState;

    let pool = create_pool().await.expect("Failed to create pool");

    let workflow = workflow_service::WorkflowService::create_workflow(
        &pool,
        "transition-benchmark-test".to_string(),
        Some("Testing transition latency".to_string()),
        Some("/tmp/test".to_string()),
    ).await.expect("Failed to create workflow");

    let iterations = 100;
    let mut total_time = 0;

    for _ in 0..iterations {
        // Reset to INIT
        workflow_service::WorkflowService::transition_status(
            &pool,
            &workflow.id,
            WorkflowState::Init,
        ).await.expect("Failed to reset to INIT");

        let start = Instant::now();

        workflow_service::WorkflowService::transition_status(
            &pool,
            &workflow.id,
            WorkflowState::Execute,
        ).await.expect("Failed to transition");

        let elapsed = start.elapsed();
        total_time += elapsed.as_micros();
    }

    let avg_time_us = total_time / iterations;
    let avg_time_ms = avg_time_us as f64 / 1000.0;

    println!("Average state transition time: {:.2}ms", avg_time_ms);
    assert!(
        avg_time_ms < 20.0,
        "Average state transition should be <20ms, got {:.2}ms",
        avg_time_ms
    );

    // Cleanup
    let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow.id).await;
}

/// Benchmark: Concurrent workflow operations
#[tokio::test]
async fn benchmark_concurrent_operations() {
    let pool = create_pool().await.expect("Failed to create pool");

    let start = Instant::now();

    // Create 10 workflows concurrently
    let mut handles = Vec::new();
    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            workflow_service::WorkflowService::create_workflow(
                &pool_clone,
                format!("concurrent-test-{}", i),
                Some("Testing concurrent operations".to_string()),
                Some("/tmp/test".to_string()),
            ).await
        });
        handles.push(handle);
    }

    let workflows = tokio::try_join_all(handles)
        .await
        .expect("Failed to join tasks")
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let elapsed = start.elapsed();

    println!("Concurrent workflow creation (10): {:?}", elapsed);
    assert_eq!(workflows.len(), 10);

    // Cleanup
    for workflow in workflows {
        let _ = workflow_service::WorkflowService::delete_workflow(&pool, &workflow.id).await;
    }
}
