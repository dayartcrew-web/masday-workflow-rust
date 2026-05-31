//! Database schema definitions
//!
//! This file defines all 16 table schemas as FromRow structs.
//! Field names match the TypeScript Drizzle schema in packages/db/src/schema.ts
//! (snake_case in Rust, camelCase in TypeScript).

use chrono::{DateTime, NaiveDateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Workflow & Planning Tables
// ============================================================================

/// Workflow table model
///
/// Represents a workflow execution with status tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project_path: Option<String>,
    pub current_plan_id: Option<String>,
    pub current_task_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workflow {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        Workflow {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            project_path: row.get("projectPath"),
            current_plan_id: row.get("currentPlanId"),
            current_task_id: row.get("currentTaskId"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
        }
    }
}

/// NewWorkflow for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflow {
    pub name: String,
    pub status: String,
    pub project_path: Option<String>,
    pub current_plan_id: Option<String>,
    pub current_task_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Plan table model
///
/// Represents a plan with phases and version tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub workflow_id: String,
    pub version: i32,
    pub status: String,
    pub summary: String,
    pub content: serde_json::Value,
    pub created_by_agent: String,
    pub created_at: DateTime<Utc>,
}

impl Plan {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        Plan {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            version: row.get("version"),
            status: row.get("status"),
            summary: row.get("summary"),
            content: row.get("content"),
            created_by_agent: row.get("createdByAgent"),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewPlan for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlan {
    pub workflow_id: String,
    pub version: i32,
    pub status: String,
    pub summary: String,
    pub content: serde_json::Value,
    pub created_by_agent: String,
}

// ============================================================================
// Task Tables
// ============================================================================

/// Task table model
///
/// Represents a task within a workflow with TDD support and progress tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub workflow_id: String,
    pub plan_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub owner_agent: Option<String>,
    pub acceptance_criteria: Option<serde_json::Value>,
    pub required_context: Option<serde_json::Value>,
    pub verification_steps: Option<serde_json::Value>,
    pub context_fingerprint: Option<String>,
    pub progress_percent: Option<i32>,
    pub requires_tdd: Option<bool>,
    pub test_evidence: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        Task {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            plan_id: row.get("planId"),
            title: row.get("title"),
            status: row.get("status"),
            priority: row.get("priority"),
            owner_agent: row.get("ownerAgent"),
            acceptance_criteria: row.try_get("acceptanceCriteria").unwrap_or(None),
            required_context: row.try_get("requiredContext").unwrap_or(None),
            verification_steps: row.try_get("verificationSteps").unwrap_or(None),
            context_fingerprint: row.get("contextFingerprint"),
            progress_percent: row.get("progressPercent"),
            requires_tdd: row.get("requiresTdd"),
            test_evidence: row.try_get("testEvidence").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
        }
    }
}

/// NewTask for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub workflow_id: String,
    pub plan_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub owner_agent: Option<String>,
    pub acceptance_criteria: Option<serde_json::Value>,
    pub required_context: Option<serde_json::Value>,
    pub verification_steps: Option<serde_json::Value>,
    pub context_fingerprint: Option<String>,
    pub progress_percent: Option<i32>,
    pub requires_tdd: Option<bool>,
    pub test_evidence: Option<serde_json::Value>,
}

/// TaskProgressLog table model
///
/// Tracks progress updates for tasks with evidence capture.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskProgressLog {
    pub id: String,
    pub workflow_id: String,
    pub task_id: String,
    pub agent_name: String,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub progress_note: String,
    pub evidence: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl TaskProgressLog {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        TaskProgressLog {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            agent_name: row.get("agentName"),
            status_before: row.get("statusBefore"),
            status_after: row.get("statusAfter"),
            progress_note: row.get("progressNote"),
            evidence: row.try_get("evidence").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewTaskProgressLog for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskProgressLog {
    pub workflow_id: String,
    pub task_id: String,
    pub agent_name: String,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub progress_note: String,
    pub evidence: Option<serde_json::Value>,
}

// ============================================================================
// Review & Session Tables
// ============================================================================

/// ReviewDecision table model
///
/// Represents a code review decision with gap analysis.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewDecision {
    pub id: String,
    pub workflow_id: String,
    pub task_id: String,
    pub reviewer_agent: String,
    pub decision: String,
    pub notes: String,
    pub gaps: Option<serde_json::Value>,
    pub tests_verified: Option<bool>,
    pub test_summary: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl ReviewDecision {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        ReviewDecision {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            reviewer_agent: row.get("reviewerAgent"),
            decision: row.get("decision"),
            notes: row.get("notes"),
            gaps: row.try_get("gaps").unwrap_or(None),
            tests_verified: row.get("testsVerified"),
            test_summary: row.try_get("testSummary").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewReviewDecision for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReviewDecision {
    pub workflow_id: String,
    pub task_id: String,
    pub reviewer_agent: String,
    pub decision: String,
    pub notes: String,
    pub gaps: Option<serde_json::Value>,
    pub tests_verified: Option<bool>,
    pub test_summary: Option<serde_json::Value>,
}

/// SessionState table model
///
/// Tracks session state with workflow, plan, task, and context loading flags.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub session_key: String,
    pub workflow_id: Option<String>,
    pub plan_id: Option<String>,
    pub task_id: Option<String>,
    pub workflow_loaded: Option<bool>,
    pub plan_loaded: Option<bool>,
    pub task_loaded: Option<bool>,
    pub context_loaded: Option<bool>,
    pub review_approved: Option<bool>,
    pub context_fingerprint: Option<String>,
    pub execution_mode: Option<String>,
    pub active_branch_ids: Option<serde_json::Value>,
    pub synthesis_ready: Option<bool>,
    pub verification_ready: Option<bool>,
    pub last_command: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl SessionState {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        SessionState {
            id: row.get("id"),
            session_key: row.get("sessionKey"),
            workflow_id: row.get("workflowId"),
            plan_id: row.get("planId"),
            task_id: row.get("taskId"),
            workflow_loaded: row.get("workflowLoaded"),
            plan_loaded: row.get("planLoaded"),
            task_loaded: row.get("taskLoaded"),
            context_loaded: row.get("contextLoaded"),
            review_approved: row.get("reviewApproved"),
            context_fingerprint: row.get("contextFingerprint"),
            execution_mode: row.get("executionMode"),
            active_branch_ids: row.try_get("activeBranchIds").unwrap_or(None),
            synthesis_ready: row.get("synthesisReady"),
            verification_ready: row.get("verificationReady"),
            last_command: row.get("lastCommand"),
            metadata: row.try_get("metadata").unwrap_or(None),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewSessionState for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionState {
    pub session_key: String,
    pub workflow_id: Option<String>,
    pub plan_id: Option<String>,
    pub task_id: Option<String>,
    pub workflow_loaded: Option<bool>,
    pub plan_loaded: Option<bool>,
    pub task_loaded: Option<bool>,
    pub context_loaded: Option<bool>,
    pub review_approved: Option<bool>,
    pub context_fingerprint: Option<String>,
    pub execution_mode: Option<String>,
    pub active_branch_ids: Option<serde_json::Value>,
    pub synthesis_ready: Option<bool>,
    pub verification_ready: Option<bool>,
    pub last_command: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Parallel Execution Tables
// ============================================================================

/// ParallelBranch table model
///
/// Represents a parallel execution branch within a workflow.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ParallelBranch {
    pub id: String,
    pub workflow_id: String,
    pub task_id: String,
    pub branch_key: String,
    pub role: String,
    pub status: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ParallelBranch {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        ParallelBranch {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            branch_key: row.get("branchKey"),
            role: row.get("role"),
            status: row.get("status"),
            input: row.get("input"),
            output: row.try_get("output").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
        }
    }
}

/// NewParallelBranch for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewParallelBranch {
    pub workflow_id: String,
    pub task_id: String,
    pub branch_key: String,
    pub role: String,
    pub status: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
}

// ============================================================================
// Memory & Context Tables
// ============================================================================

/// Memory table model
///
/// Represents long-term memory with importance scoring and tags.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub memory_type: String,
    pub summary: String,
    pub content: String,
    pub importance_score: Option<f64>,
    pub created_by_agent: String,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Vector>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub access_count: Option<i32>,
    pub version: Option<i32>,
}

impl Memory {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        Memory {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            memory_type: row.get("memoryType"),
            summary: row.get("summary"),
            content: row.get("content"),
            importance_score: row.get("importanceScore"),
            created_by_agent: row.get("createdByAgent"),
            tags: row.try_get("tags").unwrap_or(None),
            source: row.try_get("source").unwrap_or(None),
            embedding: None,
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
            accessed_at: row.get::<_, Option<NaiveDateTime>>("accessedAt").map(|t| t.and_utc()),
            access_count: row.get("accessCount"),
            version: row.get("version"),
        }
    }
}

/// NewMemory for INSERT operations (without id, timestamps, access fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMemory {
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub memory_type: String,
    pub summary: String,
    pub content: String,
    pub importance_score: Option<f64>,
    pub created_by_agent: String,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Vector>,
}

/// ContextDocument table model
///
/// Stores context documents with fingerprinting and embeddings.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ContextDocument {
    pub id: String,
    pub workflow_id: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub fingerprint: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Vector>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ContextDocument {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        ContextDocument {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            source_type: row.get("sourceType"),
            source_ref: row.get("sourceRef"),
            title: row.get("title"),
            content: row.get("content"),
            metadata: row.try_get("metadata").unwrap_or(None),
            fingerprint: row.get("fingerprint"),
            embedding: None,
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
        }
    }
}

/// NewContextDocument for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewContextDocument {
    pub workflow_id: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub fingerprint: Option<String>,
    #[serde(skip)]
    pub embedding: Option<Vector>,
}

// ============================================================================
// Knowledge Graph Tables
// ============================================================================

/// GraphNode table model
///
/// Represents a node in the knowledge graph.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub properties: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl GraphNode {
    /// Map from DB row with PascalCase/camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        GraphNode {
            id: row.get("id"),
            node_type: row.get("nodeType"),
            name: row.get("name"),
            properties: row.try_get("properties").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewGraphNode for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGraphNode {
    pub node_type: String,
    pub name: String,
    pub properties: Option<serde_json::Value>,
}

/// GraphEdge table model
///
/// Represents an edge between nodes in the knowledge graph.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation_type: String,
    pub weight: Option<f64>,
    pub bidirectional: Option<bool>,
    pub created_at: DateTime<Utc>,
}

impl GraphEdge {
    /// Map from DB row with PascalCase/camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        GraphEdge {
            id: row.get("id"),
            source_node_id: row.get("sourceNodeId"),
            target_node_id: row.get("targetNodeId"),
            relation_type: row.get("relationType"),
            weight: row.get("weight"),
            bidirectional: row.get("bidirectional"),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewGraphEdge for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGraphEdge {
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation_type: String,
    pub weight: Option<f64>,
    pub bidirectional: Option<bool>,
}

// ============================================================================
// Episodic Memory Tables
// ============================================================================

/// EpisodicMemory table model
///
/// Stores episodic memories per session with sequence ordering.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EpisodicMemory {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub sequence_order: i32,
    pub created_at: DateTime<Utc>,
}

impl EpisodicMemory {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        EpisodicMemory {
            id: row.get("id"),
            session_id: row.get("sessionId"),
            role: row.get("role"),
            content: row.get("content"),
            sequence_order: row.get("sequenceOrder"),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewEpisodicMemory for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEpisodicMemory {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub sequence_order: i32,
}

// ============================================================================
// LLM & Token Tracking Tables
// ============================================================================

/// LlmProviderConfig table model
///
/// Stores LLM provider configuration with models and priority.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: String,
    pub provider_name: String,
    pub base_url: String,
    pub api_key_env_var: String,
    pub models: serde_json::Value,
    pub is_default: Option<bool>,
    pub priority: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LlmProviderConfig {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        LlmProviderConfig {
            id: row.get("id"),
            provider_name: row.get("providerName"),
            base_url: row.get("baseUrl"),
            api_key_env_var: row.get("apiKeyEnvVar"),
            models: row.get("models"),
            is_default: row.get("isDefault"),
            priority: row.get("priority"),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
            updated_at: row.get::<_, NaiveDateTime>("updatedAt").and_utc(),
        }
    }
}

/// NewLlmProviderConfig for INSERT operations (without id, created_at, updated_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLlmProviderConfig {
    pub provider_name: String,
    pub base_url: String,
    pub api_key_env_var: String,
    pub models: serde_json::Value,
    pub is_default: Option<bool>,
    pub priority: Option<i32>,
}

/// TokenUsage table model
///
/// Tracks token usage per source, route, and model.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TokenUsage {
    pub id: String,
    pub source: String,
    pub route: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub latency_ms: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl TokenUsage {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        TokenUsage {
            id: row.get("id"),
            source: row.get("source"),
            route: row.get("route"),
            model: row.get("model"),
            prompt_tokens: row.get("promptTokens"),
            completion_tokens: row.get("completionTokens"),
            total_tokens: row.get("totalTokens"),
            latency_ms: row.get("latencyMs"),
            metadata: row.try_get("metadata").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewTokenUsage for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTokenUsage {
    pub source: String,
    pub route: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub latency_ms: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Logging & Reminder Tables
// ============================================================================

/// RetrievalLog table model
///
/// Logs retrieval operations for context and semantic search.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RetrievalLog {
    pub id: String,
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_name: String,
    pub query: String,
    pub source: String,
    pub results: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl RetrievalLog {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        RetrievalLog {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            agent_name: row.get("agentName"),
            query: row.get("query"),
            source: row.get("source"),
            results: row.try_get("results").unwrap_or(None),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewRetrievalLog for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRetrievalLog {
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_name: String,
    pub query: String,
    pub source: String,
    pub results: Option<serde_json::Value>,
}

/// WorkflowReminder table model
///
/// Stores reminders for workflows with severity levels.
/// NOTE: The "type" column is lowercase in the actual DB (not camelCase "reminderType").
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkflowReminder {
    pub id: String,
    pub workflow_id: String,
    pub task_id: Option<String>,
    pub reminder_type: String,
    pub severity: String,
    pub message: String,
    pub acknowledged: Option<bool>,
    pub created_at: DateTime<Utc>,
}

impl WorkflowReminder {
    /// Map from DB row with PascalCase table / camelCase column names
    pub fn from_row(row: &tokio_postgres::Row) -> Self {
        WorkflowReminder {
            id: row.get("id"),
            workflow_id: row.get("workflowId"),
            task_id: row.get("taskId"),
            // DB column is "type" (lowercase), not "reminderType"
            reminder_type: row.get("type"),
            severity: row.get("severity"),
            message: row.get("message"),
            acknowledged: row.get("acknowledged"),
            created_at: row.get::<_, NaiveDateTime>("createdAt").and_utc(),
        }
    }
}

/// NewWorkflowReminder for INSERT operations (without id, created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowReminder {
    pub workflow_id: String,
    pub task_id: Option<String>,
    pub reminder_type: String,
    pub severity: String,
    pub message: String,
    pub acknowledged: Option<bool>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_serialization() {
        let workflow = Workflow {
            id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
            name: "Test Workflow".to_string(),
            status: "ACTIVE".to_string(),
            project_path: Some("/path/to/project".to_string()),
            current_plan_id: None,
            current_task_id: None,
            metadata: Some(serde_json::json!({"key": "value"})),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&workflow).unwrap();
        let deserialized: Workflow = serde_json::from_str(&serialized).unwrap();

        assert_eq!(workflow.id, deserialized.id);
        assert_eq!(workflow.name, deserialized.name);
    }

    #[test]
    fn test_task_with_all_fields() {
        let task = Task {
            id: "123e4567-e89b-12d3-a456-426614174001".to_string(),
            workflow_id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
            plan_id: "123e4567-e89b-12d3-a456-426614174002".to_string(),
            title: "Implement feature".to_string(),
            status: "PENDING".to_string(),
            priority: Some("HIGH".to_string()),
            owner_agent: Some("masday-executor".to_string()),
            acceptance_criteria: Some(serde_json::json!([" criterion1"])),
            required_context: Some(serde_json::json!(["context1"])),
            verification_steps: Some(serde_json::json!(["step1"])),
            context_fingerprint: Some("abc123".to_string()),
            progress_percent: Some(50),
            requires_tdd: Some(true),
            test_evidence: Some(serde_json::json!({"test": "pass"})),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(task.status, "PENDING");
        assert_eq!(task.requires_tdd, Some(true));
        assert_eq!(task.progress_percent, Some(50));
    }

    #[test]
    fn test_memory_with_tags() {
        let memory = Memory {
            id: "123e4567-e89b-12d3-a456-426614174003".to_string(),
            workflow_id: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
            task_id: None,
            memory_type: "preference".to_string(),
            summary: "User prefers dark mode".to_string(),
            content: "User explicitly chose dark mode in settings".to_string(),
            importance_score: Some(0.8),
            created_by_agent: "masday-orchestrator".to_string(),
            tags: Some(vec!["ui".to_string(), "preferences".to_string()]),
            source: Some("user_feedback".to_string()),
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: None,
            access_count: Some(0),
            version: Some(1),
        };

        assert_eq!(memory.memory_type, "preference");
        assert!(memory.tags.is_some());
        assert_eq!(memory.tags.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_new_workflow_insertable() {
        let new_workflow = NewWorkflow {
            name: "New Workflow".to_string(),
            status: "INIT".to_string(),
            project_path: Some("/new/path".to_string()),
            current_plan_id: None,
            current_task_id: None,
            metadata: None,
        };

        assert_eq!(new_workflow.status, "INIT");
        assert!(new_workflow.project_path.is_some());
    }

    #[test]
    fn test_graph_edge_relationship() {
        let edge = GraphEdge {
            id: "123e4567-e89b-12d3-a456-426614174004".to_string(),
            source_node_id: "node1".to_string(),
            target_node_id: "node2".to_string(),
            relation_type: "depends_on".to_string(),
            weight: Some(0.8),
            bidirectional: Some(false),
            created_at: Utc::now(),
        };

        assert_eq!(edge.relation_type, "depends_on");
        assert_eq!(edge.weight, Some(0.8));
        assert_eq!(edge.bidirectional, Some(false));
    }

    #[test]
    fn test_token_usage_aggregation() {
        let token_usage = TokenUsage {
            id: "123e4567-e89b-12d3-a456-426614174005".to_string(),
            source: "openai".to_string(),
            route: "/chat/completions".to_string(),
            model: Some("gpt-4".to_string()),
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            latency_ms: Some(1250),
            metadata: None,
            created_at: Utc::now(),
        };

        assert_eq!(token_usage.total_tokens, Some(150));
        assert_eq!(token_usage.source, "openai");
    }

    #[test]
    fn test_session_state_flags() {
        let session_state = SessionState {
            id: "123e4567-e89b-12d3-a456-426614174006".to_string(),
            session_key: "session-123".to_string(),
            workflow_id: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
            plan_id: None,
            task_id: None,
            workflow_loaded: Some(true),
            plan_loaded: Some(false),
            task_loaded: Some(false),
            context_loaded: Some(false),
            review_approved: Some(false),
            context_fingerprint: None,
            execution_mode: Some("sequential".to_string()),
            active_branch_ids: None,
            synthesis_ready: Some(false),
            verification_ready: Some(false),
            last_command: None,
            metadata: None,
            updated_at: Utc::now(),
            created_at: Utc::now(),
        };

        assert_eq!(session_state.workflow_loaded, Some(true));
        assert_eq!(session_state.plan_loaded, Some(false));
        assert_eq!(session_state.execution_mode, Some("sequential".to_string()));
    }

    #[test]
    fn test_all_16_tables_exist() {
        // This test verifies all 16 tables are present in the schema
        // At compile time, this ensures no table is accidentally removed

        // Workflow & Planning (2)
        let _ = Workflow {
            id: String::new(),
            name: String::new(),
            status: String::new(),
            project_path: None,
            current_plan_id: None,
            current_task_id: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let _ = Plan {
            id: String::new(),
            workflow_id: String::new(),
            version: 0,
            status: String::new(),
            summary: String::new(),
            content: serde_json::Value::Null,
            created_by_agent: String::new(),
            created_at: Utc::now(),
        };

        // Task (2)
        let _ = Task {
            id: String::new(),
            workflow_id: String::new(),
            plan_id: String::new(),
            title: String::new(),
            status: String::new(),
            priority: None,
            owner_agent: None,
            acceptance_criteria: None,
            required_context: None,
            verification_steps: None,
            context_fingerprint: None,
            progress_percent: None,
            requires_tdd: None,
            test_evidence: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let _ = TaskProgressLog {
            id: String::new(),
            workflow_id: String::new(),
            task_id: String::new(),
            agent_name: String::new(),
            status_before: None,
            status_after: None,
            progress_note: String::new(),
            evidence: None,
            created_at: Utc::now(),
        };

        // Review & Session (2)
        let _ = ReviewDecision {
            id: String::new(),
            workflow_id: String::new(),
            task_id: String::new(),
            reviewer_agent: String::new(),
            decision: String::new(),
            notes: String::new(),
            gaps: None,
            tests_verified: None,
            test_summary: None,
            created_at: Utc::now(),
        };

        let _ = SessionState {
            id: String::new(),
            session_key: String::new(),
            workflow_id: None,
            plan_id: None,
            task_id: None,
            workflow_loaded: None,
            plan_loaded: None,
            task_loaded: None,
            context_loaded: None,
            review_approved: None,
            context_fingerprint: None,
            execution_mode: None,
            active_branch_ids: None,
            synthesis_ready: None,
            verification_ready: None,
            last_command: None,
            metadata: None,
            updated_at: Utc::now(),
            created_at: Utc::now(),
        };

        // Parallel (1)
        let _ = ParallelBranch {
            id: String::new(),
            workflow_id: String::new(),
            task_id: String::new(),
            branch_key: String::new(),
            role: String::new(),
            status: String::new(),
            input: serde_json::Value::Null,
            output: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Memory & Context (2)
        let _ = Memory {
            id: String::new(),
            workflow_id: None,
            task_id: None,
            memory_type: String::new(),
            summary: String::new(),
            content: String::new(),
            importance_score: None,
            created_by_agent: String::new(),
            tags: None,
            source: None,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: None,
            access_count: None,
            version: None,
        };

        let _ = ContextDocument {
            id: String::new(),
            workflow_id: None,
            source_type: String::new(),
            source_ref: None,
            title: None,
            content: String::new(),
            metadata: None,
            fingerprint: None,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Knowledge Graph (2)
        let _ = GraphNode {
            id: String::new(),
            node_type: String::new(),
            name: String::new(),
            properties: None,
            created_at: Utc::now(),
        };

        let _ = GraphEdge {
            id: String::new(),
            source_node_id: String::new(),
            target_node_id: String::new(),
            relation_type: String::new(),
            weight: None,
            bidirectional: None,
            created_at: Utc::now(),
        };

        // Episodic Memory (1)
        let _ = EpisodicMemory {
            id: String::new(),
            session_id: String::new(),
            role: String::new(),
            content: String::new(),
            sequence_order: 0,
            created_at: Utc::now(),
        };

        // LLM & Token (2)
        let _ = LlmProviderConfig {
            id: String::new(),
            provider_name: String::new(),
            base_url: String::new(),
            api_key_env_var: String::new(),
            models: serde_json::Value::Null,
            is_default: None,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let _ = TokenUsage {
            id: String::new(),
            source: String::new(),
            route: String::new(),
            model: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            latency_ms: None,
            metadata: None,
            created_at: Utc::now(),
        };

        // Logging & Reminder (2)
        let _ = RetrievalLog {
            id: String::new(),
            workflow_id: None,
            task_id: None,
            agent_name: String::new(),
            query: String::new(),
            source: String::new(),
            results: None,
            created_at: Utc::now(),
        };

        let _ = WorkflowReminder {
            id: String::new(),
            workflow_id: String::new(),
            task_id: None,
            reminder_type: String::new(),
            severity: String::new(),
            message: String::new(),
            acknowledged: None,
            created_at: Utc::now(),
        };

        // Total: 16 tables verified
    }
}
