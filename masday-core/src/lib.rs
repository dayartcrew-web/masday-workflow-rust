//! masday-core - Shared types, errors, and constants

pub mod constants;
pub mod error;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use error::{AppError, Result};
pub use types::{
    BranchState, LlmProvider, MemoryType, PlanState, ReviewStatus, SessionState, TaskState,
    WorkflowState,
};
pub use utils::validate_uuid;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test - will be replaced with real tests
    }
}
