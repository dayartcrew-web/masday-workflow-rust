//! masday-core - Shared types, errors, and constants

pub mod types;
pub mod error;
pub mod constants;

// Re-export commonly used types
pub use error::{AppError, Result};
pub use types::{
    WorkflowState, TaskState, PlanState, ReviewStatus,
    MemoryType, BranchState, SessionState, LlmProvider,
};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test - will be replaced with real tests
        assert!(true);
    }
}
