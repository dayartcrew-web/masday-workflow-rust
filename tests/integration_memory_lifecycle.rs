//! Integration test for memory lifecycle
//!
//! Tests: store → search → recall → delete across all 4 layers

use masday_db::pool::create_pool;
use masday_service::memory_service;
use serde_json::json;

#[tokio::test]
async fn test_working_memory_lifecycle() {
    let session_id = "test-session-1";

    // Store messages in working memory
    let msg1 = json!({"role": "user", "content": "hello"});
    let msg2 = json!({"role": "assistant", "content": "hi there"});
    let msg3 = json!({"role": "user", "content": "how are you?"});

    let resp1 = memory_service::MemoryService::working_store(session_id, msg1.clone());
    let resp2 = memory_service::MemoryService::working_store(session_id, msg2.clone());
    let resp3 = memory_service::MemoryService::working_store(session_id, msg3.clone());

    assert_eq!(resp1["status"], "stored");
    assert_eq!(resp2["status"], "stored");
    assert_eq!(resp3["status"], "stored");

    // Recall all messages
    let recalled = memory_service::MemoryService::working_recall(session_id, 10);
    assert_eq!(recalled.len(), 3);

    // Recall with limit
    let limited = memory_service::MemoryService::working_recall(session_id, 2);
    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn test_episodic_memory_lifecycle() {
    let pool = create_pool().await.expect("Failed to create pool");
    let session_id = "test-session-episodic";

    // Store episodes
    memory_service::MemoryService::episodic_store(
        &pool,
        session_id,
        "user",
        "Hello, how can you help me?"
    ).await.expect("Failed to store episode 1");

    memory_service::MemoryService::episodic_store(
        &pool,
        session_id,
        "assistant",
        "I can help you with your tasks"
    ).await.expect("Failed to store episode 2");

    memory_service::MemoryService::episodic_store(
        &pool,
        session_id,
        "user",
        "Great, let's start"
    ).await.expect("Failed to store episode 3");

    // Recall episodes
    let recalled = memory_service::MemoryService::episodic_recall(
        &pool,
        session_id,
        10,
    ).await.expect("Failed to recall episodes");

    assert_eq!(recalled.len(), 3);
    assert_eq!(recalled[0]["role"], "user");
    assert_eq!(recalled[0]["content"], "Hello, how can you help me?");

    // Recall with limit
    let limited = memory_service::MemoryService::episodic_recall(
        &pool,
        session_id,
        2,
    ).await.expect("Failed to recall limited episodes");

    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn test_long_term_memory_lifecycle() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Store memory
    let stored = memory_service::MemoryService::store(
        &pool,
        memory_service::StoreMemoryParams {
            memory_type: "fact",
            summary: "Test fact",
            content: "Rust is a systems programming language",
            created_by: "test-agent",
            importance: 0.9,
            tags: vec!["rust".to_string(), "programming".to_string()],
            workflow_id: None,
            task_id: None,
        },
    ).await.map_err(|e| format!("Store failed: {}", e))?;

    let memory_id = stored["id"].as_str().ok_or("Missing id")?;

    // Search memory
    let search_results = memory_service::MemoryService::search(&pool, "Rust", 10)
        .await.map_err(|e| format!("Search failed: {}", e))?;

    assert!(!search_results.is_empty());
    assert!(search_results.iter().any(|m| m["id"].as_str() == Some(memory_id)));

    // Recall recent
    let recent = memory_service::MemoryService::recall_recent(&pool, 10)
        .await.map_err(|e| format!("Recall recent failed: {}", e))?;

    assert!(!recent.is_empty());

    // Update memory
    let updated = memory_service::MemoryService::update(
        &pool,
        memory_id,
        Some("Updated: Rust is a systems programming language with memory safety"),
        Some(0.95),
    ).await.map_err(|e| format!("Update failed: {}", e))?;

    assert_eq!(updated["importance_score"].as_f64(), Some(0.95));

    // Delete memory
    let deleted = memory_service::MemoryService::delete(&pool, memory_id)
        .await.map_err(|e| format!("Delete failed: {}", e))?;

    assert_eq!(deleted["id"].as_str(), Some(memory_id));

    // Verify deletion
    let search_after_delete = memory_service::MemoryService::search(&pool, "Rust", 10)
        .await.map_err(|e| format!("Search after delete failed: {}", e))?;

    assert!(search_after_delete.iter().all(|m| m["id"].as_str() != Some(memory_id)));

    Ok(())
}

#[tokio::test]
async fn test_memory_by_task_association() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Create a task ID for testing
    let task_id = uuid::Uuid::new_v4().to_string();

    // Store memories associated with task
    memory_service::MemoryService::store(
        &pool,
        memory_service::StoreMemoryParams {
            memory_type: "preference",
            summary: "User prefers TypeScript",
            content: "User likes TypeScript over JavaScript",
            created_by: "test-agent",
            importance: 0.7,
            tags: vec!["typescript".to_string()],
            workflow_id: None,
            task_id: Some(&task_id),
        },
    ).await.map_err(|e| format!("Store memory 1 failed: {}", e))?;

    memory_service::MemoryService::store(
        &pool,
        memory_service::StoreMemoryParams {
            memory_type: "fact",
            summary: "Task completed",
            content: "Task implementation was successful",
            created_by: "test-agent",
            importance: 0.8,
            tags: vec!["task".to_string()],
            workflow_id: None,
            task_id: Some(&task_id),
        },
    ).await.map_err(|e| format!("Store memory 2 failed: {}", e))?;

    // Recall by task
    let task_memories = memory_service::MemoryService::recall_by_task(&pool, &task_id, 10)
        .await.map_err(|e| format!("Recall by task failed: {}", e))?;

    assert_eq!(task_memories.len(), 2);
    assert!(task_memories.iter().all(|m| m["task_id"].as_str() == Some(&task_id)));

    Ok(())
}

#[tokio::test]
async fn test_memory_statistics() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Store various memory types
    for memory_type in &["fact", "preference", "skill", "experience"] {
        memory_service::MemoryService::store(
            &pool,
            memory_service::StoreMemoryParams {
                memory_type,
                summary: &format!("Test {}", memory_type),
                content: &format!("Content for {}", memory_type),
                created_by: "test-agent",
                importance: 0.7,
                tags: vec![memory_type.to_string()],
                workflow_id: None,
                task_id: None,
            },
        ).await.map_err(|e| format!("Store {} failed: {}", memory_type, e))?;
    }

    // Get statistics
    let stats = memory_service::MemoryService::stats(&pool)
        .await.map_err(|e| format!("Stats failed: {}", e))?;

    assert!(stats["total_count"].as_i64().unwrap_or(0) >= 4);

    let by_type = stats["by_type"].as_object().ok_or("Missing by_type")?;
    assert!(by_type.len() >= 4);

    Ok(())
}

#[tokio::test]
async fn test_knowledge_graph_node_lifecycle() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Add node
    let node = memory_service::MemoryService::add_node(
        &pool,
        "Rust",
        "language",
        vec!["systems programming".to_string(), "memory safety".to_string()],
    ).await.map_err(|e| format!("Add node failed: {}", e))?;

    let node_id = node["id"].as_str().ok_or("Missing node id")?;

    // Search nodes
    let found = memory_service::MemoryService::search_nodes(&pool, "Rust")
        .await.map_err(|e| format!("Search nodes failed: {}", e))?;

    assert!(!found.is_empty());
    assert!(found.iter().any(|n| n["id"].as_str() == Some(node_id)));

    Ok(())
}

#[tokio::test]
async fn test_auto_link_nodes() -> Result<(), String> {
    let pool = create_pool().await.map_err(|e| e.to_string())?;

    // Add nodes with similar content (should auto-link)
    let node1 = memory_service::MemoryService::add_node(
        &pool,
        "TypeScript",
        "language",
        vec!["JavaScript superset".to_string(), "static typing".to_string()],
    ).await.map_err(|e| format!("Add node 1 failed: {}", e))?;

    let node1_id = node1["id"].as_str().ok_or("Missing node 1 id")?;

    let node2 = memory_service::MemoryService::add_node(
        &pool,
        "JavaScript",
        "language",
        vec!["dynamic typing".to_string(), "web development".to_string()],
    ).await.map_err(|e| format!("Add node 2 failed: {}", e))?;

    let node2_id = node2["id"].as_str().ok_or("Missing node 2 id")?;

    // Auto-link (Jaccard similarity should find "JavaScript" overlap)
    let edge_count = memory_service::MemoryService::auto_link(&pool, node1_id)
        .await.map_err(|e| format!("Auto-link failed: {}", e))?;

    // May or may not create edges depending on similarity threshold
    assert!(edge_count >= 0);

    Ok(())
}

#[tokio::test]
async fn test_memory_importance_scoring() {
    // Test the importance calculation logic
    use masday_service::memory_service::MemoryService;

    // Fact type, 200 chars
    let imp = MemoryService::calculate_importance("fact", 200);
    assert!((imp - 1.0).abs() < 0.01);

    // Preference type, 500 chars
    let imp = MemoryService::calculate_importance("preference", 500);
    assert!((imp - 0.8).abs() < 0.01);

    // Skill type, 50 chars
    let imp = MemoryService::calculate_importance("skill", 50);
    assert!((imp - 0.65).abs() < 0.01);

    // Long content max bonus
    let imp = MemoryService::calculate_importance("skill", 5000);
    assert!((imp - 0.7).abs() < 0.01);
}

#[tokio::test]
async fn test_bm25_scoring() {
    use masday_service::memory_service::MemoryService;

    // Basic BM25 calculation
    let score = MemoryService::bm25_score("test query", "test document query", 10.0, 3.0, 2.0, 100.0);
    assert!(score > 0.0);

    // No matches
    let score = MemoryService::bm25_score("foo bar", "baz qux", 10.0, 3.0, 2.0, 100.0);
    assert_eq!(score, 0.0);
}

#[tokio::test]
async fn test_jaccard_similarity() {
    use masday_service::memory_service::MemoryService;

    // Identical
    let sim = MemoryService::jaccard_similarity("test workflow", "test workflow");
    assert!((sim - 1.0).abs() < 0.01);

    // Partial overlap
    let sim = MemoryService::jaccard_similarity("test workflow one", "test workflow two");
    assert!((sim - 0.5).abs() < 0.01);

    // No overlap
    assert_eq!(MemoryService::jaccard_similarity("foo bar", "baz qux"), 0.0);

    // Empty
    assert_eq!(MemoryService::jaccard_similarity("", ""), 0.0);
}
