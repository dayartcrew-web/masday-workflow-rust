//! 4-layer memory service implementation
//!
//! Layer 1: Working Memory - In-process RAM, per session
//! Layer 2: Episodic Memory - Last N messages per session, persisted
//! Layer 3: Long-Term Memory - PostgreSQL via MemoryRepo
//! Layer 4: Knowledge Graph - PostgreSQL via GraphRepo

use masday_core::{AppError, Result};
use masday_db::{
    repos::{GraphRepo, MemoryRepo},
    DbPool,
};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Episode in episodic memory
#[derive(Debug, Clone)]
struct Episode {
    role: String,
    content: String,
    timestamp: i64,
}

/// Working memory - Layer 1 (In-process RAM, per session)
struct WorkingMemory {
    sessions: Mutex<HashMap<String, Vec<serde_json::Value>>>,
}

impl WorkingMemory {
    fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    fn store(&self, session_id: &str, message: serde_json::Value) -> serde_json::Value {
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .entry(session_id.to_string())
            .or_default()
            .push(message.clone());

        let response = json!({
            "session_id": session_id,
            "message": message,
            "status": "stored",
        });

        debug!("Working memory stored message for session {}", session_id);
        response
    }

    fn recall(&self, session_id: &str, limit: usize) -> Vec<serde_json::Value> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(session_id)
            .map(|messages| {
                let start = if messages.len() > limit {
                    messages.len() - limit
                } else {
                    0
                };
                messages[start..].to_vec()
            })
            .unwrap_or_default()
    }
}

/// Episodic memory - Layer 2 (Last N messages per session)
struct EpisodicMemory {
    capacity: usize,
    store: Mutex<HashMap<String, VecDeque<Episode>>>,
}

impl EpisodicMemory {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            store: Mutex::new(HashMap::new()),
        }
    }

    fn store(&self, session_id: &str, role: &str, content: &str) {
        let mut store = self.store.lock().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let episode = Episode {
            role: role.to_string(),
            content: content.to_string(),
            timestamp,
        };

        store
            .entry(session_id.to_string())
            .or_default()
            .push_back(episode.clone());

        // LRU eviction
        if let Some(session) = store.get_mut(session_id) {
            while session.len() > self.capacity {
                session.pop_front();
            }
        }

        debug!("Episodic memory stored episode for session {}", session_id);
    }

    fn recall(&self, session_id: &str, limit: usize) -> Vec<Episode> {
        let store = self.store.lock().unwrap();
        store
            .get(session_id)
            .map(|episodes| {
                let start = if episodes.len() > limit {
                    episodes.len() - limit
                } else {
                    0
                };
                episodes.iter().skip(start).cloned().collect()
            })
            .unwrap_or_default()
    }
}

/// Parameters for storing a memory
#[derive(Debug, Clone)]
pub struct StoreMemoryParams<'a> {
    pub memory_type: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub created_by: &'a str,
    pub importance: f64,
    pub tags: Vec<String>,
    pub workflow_id: Option<&'a str>,
    pub task_id: Option<&'a str>,
}

/// Memory service - Public API for all 4 layers
pub struct MemoryService;

impl MemoryService {
    // ========== Layer 1: Working Memory ==========

    /// Store message in working memory (in-process RAM)
    pub fn working_store(session_id: &str, message: serde_json::Value) -> serde_json::Value {
        static WORKING_MEMORY: std::sync::OnceLock<WorkingMemory> = std::sync::OnceLock::new();
        let working = WORKING_MEMORY.get_or_init(WorkingMemory::new);
        working.store(session_id, message)
    }

    /// Recall messages from working memory
    pub fn working_recall(session_id: &str, limit: usize) -> Vec<serde_json::Value> {
        static WORKING_MEMORY: std::sync::OnceLock<WorkingMemory> = std::sync::OnceLock::new();
        let working = WORKING_MEMORY.get_or_init(WorkingMemory::new);
        working.recall(session_id, limit)
    }

    // ========== Layer 2: Episodic Memory ==========

    /// Store episode in episodic memory
    pub async fn episodic_store(
        pool: &DbPool,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<()> {
        static EPISODIC_MEMORY: std::sync::OnceLock<EpisodicMemory> = std::sync::OnceLock::new();

        let episodic = EPISODIC_MEMORY.get_or_init(|| EpisodicMemory::with_capacity(100));
        episodic.store(session_id, role, content);

        // Persist to PostgreSQL
        let client = pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let query = "
            INSERT INTO episodic_memories (id, session_id, role, content, timestamp)
            VALUES ($1, $2, $3, $4, to_timestamp($5))
        ";

        client
            .execute(query, &[&id, &session_id, &role, &content, &timestamp])
            .await
            .map_err(|e| AppError::Database(format!("Failed to store episodic memory: {}", e)))?;

        Ok(())
    }

    /// Recall episodes from episodic memory
    pub async fn episodic_recall(
        _pool: &DbPool,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        static EPISODIC_MEMORY: std::sync::OnceLock<EpisodicMemory> = std::sync::OnceLock::new();

        let episodic = EPISODIC_MEMORY.get_or_init(|| EpisodicMemory::with_capacity(100));
        let episodes = episodic.recall(session_id, limit);

        Ok(episodes
            .into_iter()
            .map(|ep| {
                json!({
                    "role": ep.role,
                    "content": ep.content,
                    "timestamp": ep.timestamp,
                })
            })
            .collect())
    }

    // ========== Layer 3: Long-Term Memory (delegates to MemoryRepo) ==========

    /// Store memory in long-term memory
    pub async fn store(pool: &DbPool, params: StoreMemoryParams<'_>) -> Result<serde_json::Value> {
        use masday_db::schema::NewMemory;

        let repo = MemoryRepo::new(pool.clone());
        let new_memory = NewMemory {
            workflow_id: params.workflow_id.map(|s| s.to_string()),
            task_id: params.task_id.map(|s| s.to_string()),
            memory_type: params.memory_type.to_string(),
            summary: params.summary.to_string(),
            content: params.content.to_string(),
            importance_score: Some(params.importance),
            created_by_agent: params.created_by.to_string(),
            tags: Some(params.tags),
            source: None,
            embedding: None,
        };

        let memory = repo.store(&new_memory).await?;
        Ok(json!(memory))
    }

    /// Search memories (delegates to MemoryRepo)
    pub async fn search(
        pool: &DbPool,
        query: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let repo = MemoryRepo::new(pool.clone());
        let memories = repo.search(query, limit as i64).await?;
        Ok(memories.into_iter().map(|m| json!(m)).collect())
    }

    /// Recall recent memories (delegates to MemoryRepo)
    pub async fn recall_recent(pool: &DbPool, limit: usize) -> Result<Vec<serde_json::Value>> {
        let repo = MemoryRepo::new(pool.clone());
        let memories = repo.recall_recent(limit as i64).await?;
        Ok(memories.into_iter().map(|m| json!(m)).collect())
    }

    /// Recall memories by task ID (delegates to MemoryRepo)
    pub async fn recall_by_task(
        pool: &DbPool,
        task_id: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let repo = MemoryRepo::new(pool.clone());
        let memories = repo.recall_by_task(task_id).await?;
        let limited: Vec<_> = memories.into_iter().take(limit).map(|m| json!(m)).collect();
        Ok(limited)
    }

    /// Update memory (delegates to MemoryRepo)
    pub async fn update(
        pool: &DbPool,
        id: &str,
        content: Option<&str>,
        importance: Option<f64>,
    ) -> Result<serde_json::Value> {
        let repo = MemoryRepo::new(pool.clone());
        let mut updates = serde_json::Map::new();

        if let Some(cnt) = content {
            updates.insert("content".to_string(), json!(cnt));
        }
        if let Some(imp) = importance {
            updates.insert("importance_score".to_string(), json!(imp));
        }

        let memory = repo.update(id, json!(updates)).await?;
        Ok(json!(memory))
    }

    /// Delete memory (delegates to MemoryRepo)
    pub async fn delete(pool: &DbPool, id: &str) -> Result<serde_json::Value> {
        let repo = MemoryRepo::new(pool.clone());

        // First get the memory to return it
        let memory = repo.get_by_id(id).await?;

        // Then delete it
        repo.delete(id).await?;

        Ok(json!(memory))
    }

    /// Get memory statistics (delegates to MemoryRepo)
    pub async fn stats(pool: &DbPool) -> Result<serde_json::Value> {
        let repo = MemoryRepo::new(pool.clone());
        let stats = repo.stats().await?;
        Ok(json!({
            "total_count": stats.total_count,
            "by_type": stats.by_type,
        }))
    }

    // ========== Layer 4: Knowledge Graph (delegates to GraphRepo) ==========

    /// Add node to knowledge graph
    pub async fn add_node(
        pool: &DbPool,
        name: &str,
        entity_type: &str,
        observations: Vec<String>,
    ) -> Result<serde_json::Value> {
        use masday_db::schema::NewGraphNode;

        let repo = GraphRepo::new(pool.clone());
        let new_node = NewGraphNode {
            node_type: entity_type.to_string(),
            name: name.to_string(),
            properties: Some(json!({ "observations": observations })),
        };

        let node = repo.add_node(&new_node).await?;
        Ok(json!(node))
    }

    /// Search knowledge graph nodes (delegates to GraphRepo)
    pub async fn search_nodes(pool: &DbPool, query: &str) -> Result<Vec<serde_json::Value>> {
        let repo = GraphRepo::new(pool.clone());
        let nodes = repo.search_nodes("", query, 20).await?;
        Ok(nodes.into_iter().map(|n| json!(n)).collect())
    }

    /// Auto-link nodes based on Jaccard similarity
    pub async fn auto_link(pool: &DbPool, node_id: &str) -> Result<usize> {
        let repo = GraphRepo::new(pool.clone());
        let threshold = 0.3;
        let edges = repo.auto_link(node_id, threshold).await?;
        Ok(edges.len())
    }

    // ========== Utility Functions ==========

    /// Calculate BM25 score for text search
    pub fn bm25_score(
        query: &str,
        document: &str,
        avg_dl: f64,
        doc_len: f64,
        df: f64,
        n_docs: f64,
    ) -> f64 {
        let k1 = 1.2;
        let b = 0.75;

        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let doc_terms: Vec<&str> = document.split_whitespace().collect();

        let mut score = 0.0;

        for term in query_terms {
            let term_freq = doc_terms.iter().filter(|&&t| t == term).count() as f64;

            if term_freq > 0.0 {
                let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                let numerator = term_freq * (k1 + 1.0);
                let denominator = term_freq + k1 * (1.0 - b + b * (doc_len / avg_dl));
                score += idf * (numerator / denominator);
            }
        }

        score
    }

    /// Calculate importance score based on memory type and content length
    pub fn calculate_importance(memory_type: &str, content_len: usize) -> f64 {
        let type_score = match memory_type {
            "fact" => 0.9,
            "preference" => 0.7,
            "strategy" => 0.8,
            "skill" => 0.6,
            "experience" => 0.5,
            _ => 0.5,
        };

        let length_bonus = (content_len as f64 / 1000.0).min(0.1);

        type_score + length_bonus
    }

    /// Calculate Jaccard similarity between two strings
    pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
        let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if set_a.is_empty() && set_b.is_empty() {
            return 0.0;
        }

        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;

        if union == 0.0 {
            return 0.0;
        }

        intersection / union
    }
}

impl EpisodicMemory {
    #[allow(dead_code)]
    fn new_with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_similarity() {
        // Identical
        assert!(
            (MemoryService::jaccard_similarity("test workflow", "test workflow") - 1.0).abs()
                < 0.01
        );

        // Partial overlap
        let sim = MemoryService::jaccard_similarity("test workflow one", "test workflow two");
        assert!((sim - 0.5).abs() < 0.01);

        // No overlap
        assert_eq!(MemoryService::jaccard_similarity("foo bar", "baz qux"), 0.0);

        // Empty
        assert_eq!(MemoryService::jaccard_similarity("", ""), 0.0);
    }

    #[test]
    fn test_calculate_importance() {
        // Fact type, 200 chars: 0.9 + (200/1000).min(0.1) = 0.9 + 0.1 = 1.0
        let imp = MemoryService::calculate_importance("fact", 200);
        assert!((imp - 1.0).abs() < 0.01);

        // Preference type, 500 chars: 0.7 + (500/1000).min(0.1) = 0.7 + 0.1 = 0.8
        let imp = MemoryService::calculate_importance("preference", 500);
        assert!((imp - 0.8).abs() < 0.01);

        // Unknown type, 100 chars: 0.5 + (100/1000).min(0.1) = 0.5 + 0.1 = 0.6
        let imp = MemoryService::calculate_importance("unknown", 100);
        assert!((imp - 0.6).abs() < 0.01);

        // Skill type, 50 chars: 0.6 + (50/1000).min(0.1) = 0.6 + 0.05 = 0.65
        let imp = MemoryService::calculate_importance("skill", 50);
        assert!((imp - 0.65).abs() < 0.01);

        // Long content max bonus: 0.6 + 0.1 (capped) = 0.7
        let imp = MemoryService::calculate_importance("skill", 5000);
        assert!((imp - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_bm25_score() {
        // Basic BM25 calculation
        let score =
            MemoryService::bm25_score("test query", "test document query", 10.0, 3.0, 2.0, 100.0);
        assert!(score > 0.0);

        // No matches
        let score = MemoryService::bm25_score("foo bar", "baz qux", 10.0, 3.0, 2.0, 100.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_working_memory() {
        let session_id = "test-session";

        // Create a fresh working memory for testing
        let working = WorkingMemory::new();

        // Store messages
        let msg1 = json!({"role": "user", "content": "hello"});
        let msg2 = json!({"role": "assistant", "content": "hi there"});

        working.store(session_id, msg1.clone());
        working.store(session_id, msg2.clone());

        // Recall
        let recalled = working.recall(session_id, 10);
        assert_eq!(recalled.len(), 2);

        // Limit
        let limited = working.recall(session_id, 1);
        assert_eq!(limited.len(), 1);
    }
}
