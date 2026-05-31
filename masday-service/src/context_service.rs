//! Context packs and fingerprinting
//!
//! Builds context packs for tasks and computes fingerprints for context validation.

use masday_core::{AppError, Result};
use masday_db::repos::{MemoryRepo, PlanRepo, TaskRepo};
use masday_db::DbPool;
use tracing::{debug, info};

/// Context service
pub struct ContextService {
    task_repo: TaskRepo,
    plan_repo: PlanRepo,
    memory_repo: MemoryRepo,
}

impl ContextService {
    /// Create a new context service
    pub fn new(pool: DbPool) -> Self {
        Self {
            task_repo: TaskRepo::new(pool.clone()),
            plan_repo: PlanRepo::new(pool.clone()),
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
}
