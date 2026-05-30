//! Shared type definitions

use serde::{Deserialize, Serialize};

/// Workflow state representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkflowState {
    Init,
    Analyze,
    Plan,
    Execute,
    Verify,
    Fix,
    Done,
    Failed,
    Paused,
}

impl WorkflowState {
    /// Check if state transition is valid
    pub fn can_transition_to(&self, target: &WorkflowState) -> bool {
        match (self, target) {
            (WorkflowState::Init, WorkflowState::Analyze | WorkflowState::Done | WorkflowState::Failed) => true,
            (WorkflowState::Analyze, WorkflowState::Plan | WorkflowState::Done | WorkflowState::Failed) => true,
            (WorkflowState::Plan, WorkflowState::Execute | WorkflowState::Paused | WorkflowState::Failed) => true,
            (WorkflowState::Execute, WorkflowState::Verify | WorkflowState::Fix | WorkflowState::Paused | WorkflowState::Failed) => true,
            (WorkflowState::Verify, WorkflowState::Done | WorkflowState::Fix) => true,
            (WorkflowState::Fix, WorkflowState::Done | WorkflowState::Execute | WorkflowState::Failed) => true,
            (WorkflowState::Paused, WorkflowState::Execute | WorkflowState::Failed) => true,
            _ => false,
        }
    }
}

/// Task state representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskState {
    Pending,
    Running,
    Done,
    Failed,
}

/// Plan state representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PlanState {
    Active,
    Pending,
    Ready,
    Done,
}

/// Review decision status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReviewStatus {
    Approved,
    ReworkRequired,
    Blocked,
}

/// Memory type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Fact,
    Preference,
    Skill,
    Experience,
    Strategy,
}

/// Parallel branch state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum BranchState {
    Active,
    Completed,
    Failed,
}

/// Session state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SessionState {
    Active,
    Idle,
    Closed,
}

/// LLM provider enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LlmProvider {
    Anthropic,
    OpenAi,
    Custom,
}
