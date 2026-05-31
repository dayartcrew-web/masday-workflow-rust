//! Agent and skill registry
//!
//! Stub implementation - full functionality will be in the MCP layer.
//! This service provides placeholders for agent and skill discovery.

use masday_core::Result;
use tracing::debug;

/// Capability service (stub - full implementation in MCP layer)
pub struct CapabilityService;

impl Default for CapabilityService {
    fn default() -> Self {
        Self
    }
}

impl CapabilityService {
    /// List available agents (stub)
    ///
    /// # Arguments
    /// * `project_root` - Path to project root
    ///
    /// # Returns
    /// * `Result<Vec<serde_json::Value>>` - List of agents (placeholder)
    ///
    /// # Note
    /// This is a stub implementation. The full implementation will read
    /// the `.claude/agents/` directory in the MCP layer.
    pub async fn list_agents(project_root: &str) -> Result<Vec<serde_json::Value>> {
        debug!(
            "Listing agents for project root {} (stub implementation)",
            project_root
        );

        // Return placeholder data
        // In production, this would read .claude/agents/*.md files
        Ok(vec![
            serde_json::json!({
                "name": "masday-orchestrator",
                "description": "Workflow orchestration agent",
                "capabilities": ["workflow", "coordination"]
            }),
            serde_json::json!({
                "name": "masday-tdd-guide",
                "description": "Test-driven development guide",
                "capabilities": ["testing", "tdd"]
            }),
            serde_json::json!({
                "name": "masday-executor",
                "description": "Code execution agent",
                "capabilities": ["implementation", "coding"]
            }),
        ])
    }

    /// Match an agent to a task description (stub)
    ///
    /// # Arguments
    /// * `task_description` - Description of the task
    ///
    /// # Returns
    /// * `Result<String>` - Matched agent name
    ///
    /// # Note
    /// This is a stub implementation. The full implementation will use
    /// semantic search and capability matching in the MCP layer.
    pub async fn match_agent(task_description: &str) -> Result<String> {
        debug!(
            "Matching agent for task: {} (stub implementation)",
            task_description
        );

        // Simple keyword-based matching for stub
        let task_lower = task_description.to_lowercase();

        if task_lower.contains("workflow") || task_lower.contains("orchestrat") {
            Ok("masday-orchestrator".to_string())
        } else if task_lower.contains("test") || task_lower.contains("tdd") {
            Ok("masday-tdd-guide".to_string())
        } else if task_lower.contains("implement") || task_lower.contains("code") {
            Ok("masday-executor".to_string())
        } else {
            // Default to orchestrator
            Ok("masday-orchestrator".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_agents() {
        let agents = CapabilityService::list_agents("/tmp/project")
            .await
            .unwrap();
        assert!(!agents.is_empty());
        assert!(agents.len() >= 3);
    }

    #[tokio::test]
    async fn test_match_agent() {
        // Test workflow matching
        let agent = CapabilityService::match_agent("Create a new workflow")
            .await
            .unwrap();
        assert_eq!(agent, "masday-orchestrator");

        // Test TDD matching
        let agent = CapabilityService::match_agent("Write tests for this feature")
            .await
            .unwrap();
        assert_eq!(agent, "masday-tdd-guide");

        // Test implementation matching
        let agent = CapabilityService::match_agent("Implement the user service")
            .await
            .unwrap();
        assert_eq!(agent, "masday-executor");
    }
}
