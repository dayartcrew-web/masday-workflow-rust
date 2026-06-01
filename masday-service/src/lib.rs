//! masday-service - Business logic layer

pub mod capability_service;
pub mod context_service;
pub mod embedding_service;
pub mod memory_service;
pub mod plan_service;
pub mod policy_service;
pub mod reminder_service;
pub mod review_service;
pub mod search_service;
pub mod task_service;
pub mod workflow_service;

// Re-export service types for convenient access
pub use capability_service::CapabilityService;
pub use context_service::ContextService;
pub use embedding_service::EmbeddingService;
pub use memory_service::MemoryService;
pub use plan_service::PlanService;
pub use policy_service::PolicyService;
pub use reminder_service::ReminderService;
pub use review_service::ReviewService;
pub use search_service::SearchService;
pub use task_service::TaskService;
pub use workflow_service::WorkflowService;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test
    }
}
