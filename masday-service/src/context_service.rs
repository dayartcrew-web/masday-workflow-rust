//! Context packs and fingerprinting
//!
//! Builds context packs for tasks and computes fingerprints for context validation.

use masday_core::{AppError, Result};
use masday_db::repos::{ContextDocumentRepo, MemoryRepo, PlanRepo, TaskRepo};
use masday_db::DbPool;
use tracing::{debug, info};

/// Context service
pub struct ContextService {
    task_repo: TaskRepo,
    plan_repo: PlanRepo,
    context_document_repo: ContextDocumentRepo,
    memory_repo: MemoryRepo,
}

impl ContextService {
    /// Create a new context service
    pub fn new(pool: DbPool) -> Self {
        Self {
            task_repo: TaskRepo::new(pool.clone()),
            plan_repo: PlanRepo::new(pool.clone()),
            context_document_repo: ContextDocumentRepo::new(pool.clone()),
            memory_repo: MemoryRepo::new(pool),
        }
    }

    /// Build a context pack for a task
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workflow_id` - Workflow ID
    /// * `plan_id` - Plan ID
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `Result<serde_json::Value>` - Context pack as JSON
    pub async fn build_context_pack(
        pool: &DbPool,
        workflow_id: &str,
        plan_id: &str,
        task_id: &str,
    ) -> Result<serde_json::Value> {
        debug!(
            "Building context pack for task {} in workflow {}",
            task_id, workflow_id
        );

        let service = Self::new(pool.clone());

        // Get task details
        let task = service.task_repo.get_by_id(task_id).await?;

        // Get plan details
        let plan = service
            .plan_repo
            .get_by_workflow(workflow_id)
            .await?
            .ok_or_else(|| AppError::not_found("Plan", workflow_id))?;

        // Get related memories (workflow context, bounded to 100)
        let memories = service
            .memory_repo
            .recall_by_workflow(workflow_id, 100)
            .await
            .unwrap_or_default();

        // Get related context documents (round-1 M6: previously ignored — the
        // ContextDocument table was write-only from execution). Bounded to 100
        // (consistent with memories); non-fatal on error.
        let context_documents = service
            .context_document_repo
            .list_by_workflow(workflow_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .take(100)
            .collect::<Vec<_>>();

        // Build context pack
        let context_pack = serde_json::json!({
            "workflow_id": workflow_id,
            "plan_id": plan_id,
            "task_id": task_id,
            "task": {
                "id": task.id,
                "title": task.title,
                "status": task.status,
                "acceptance_criteria": task.acceptance_criteria,
                "verification_steps": task.verification_steps,
                "required_context": task.required_context,
                "context_fingerprint": task.context_fingerprint
            },
            "plan": {
                "id": plan.id,
                "version": plan.version,
                "status": plan.status,
                "summary": plan.summary,
                "content": plan.content
            },
            "memories": memories,
            "context_documents": context_documents,
            "fingerprint": Self::compute_fingerprint(workflow_id, plan_id, task_id),
            "built_at": chrono::Utc::now().to_rfc3339()
        });

        info!("Context pack built for task {}", task_id);
        Ok(context_pack)
    }

    /// Compute a fingerprint for a task context
    ///
    /// # Arguments
    /// * `workflow_id` - Workflow ID
    /// * `plan_id` - Plan ID
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// * `String` - Fingerprint hash
    pub fn compute_fingerprint(workflow_id: &str, plan_id: &str, task_id: &str) -> String {
        debug!(
            "Computing fingerprint for workflow={}, plan={}, task={}",
            workflow_id, plan_id, task_id
        );

        // Create a simple fingerprint from the IDs
        // In production, this would use a proper hash function
        let fingerprint_data = format!("{}:{}:{}", workflow_id, plan_id, task_id);

        // Simple hash using std::collections::hash_map::DefaultHasher
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        fingerprint_data.hash(&mut hasher);
        let hash = hasher.finish();

        format!("{:x}", hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fingerprint() {
        let fp1 = ContextService::compute_fingerprint("wf1", "plan1", "task1");
        let fp2 = ContextService::compute_fingerprint("wf1", "plan1", "task1");
        let fp3 = ContextService::compute_fingerprint("wf1", "plan1", "task2");

        // Same inputs should produce same fingerprint
        assert_eq!(fp1, fp2);

        // Different inputs should produce different fingerprints
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_validate() {
        // Placeholder test
    }

    #[test]
    fn test_context_document_serializes_without_embedding() {
        // round-1 M6: build_context_pack now embeds Vec<ContextDocument> directly.
        // Pin the serde contract so the pack never bloats with the embedding vector
        // (#[serde(skip)] on the field) and the document fields are present.
        use chrono::Utc;
        use masday_db::schema::ContextDocument;

        let doc = ContextDocument {
            id: "doc-1".into(),
            workflow_id: Some("wf-1".into()),
            source_type: "analysis".into(),
            source_ref: None,
            title: Some("Analysis Summary".into()),
            content: "summary text".into(),
            metadata: None,
            fingerprint: Some("ctx-deadbeef".into()),
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let map = serde_json::to_value(&doc).unwrap();
        let obj = map.as_object().unwrap();
        assert_eq!(obj["id"], "doc-1");
        assert_eq!(obj["source_type"], "analysis");
        assert_eq!(obj["title"], "Analysis Summary");
        assert_eq!(obj["content"], "summary text");
        assert_eq!(obj["fingerprint"], "ctx-deadbeef");
        assert!(
            !obj.contains_key("embedding"),
            "embedding must be skipped so it never enters the context pack"
        );
    }
}
