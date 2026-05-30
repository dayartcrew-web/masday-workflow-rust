//! masday-service - Business logic layer

pub mod workflow_service;
pub mod task_service;
pub mod plan_service;
pub mod memory_service;
pub mod review_service;
pub mod policy_service;
pub mod capability_service;
pub mod context_service;
pub mod reminder_service;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
