//! SQLite schema for standalone stdio mode.
//!
//! Adapted from masday-db/migrations/001_initial_schema.sql.
//! Differences from PostgreSQL:
//! - JSONB → TEXT (store JSON strings, parse on read)
//! - TEXT[] → TEXT (JSON array strings)
//! - TIMESTAMPTZ → TEXT (ISO 8601 / RFC 3339)
//! - vector(768) → omitted (no vector search in standalone mode)
//! - Table names use snake_case (SQLite convention)

pub const SCHEMA: &str = r#"
-- ============================================================================
-- Workflow & Planning
-- ============================================================================

CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'INIT',
    project_path TEXT,
    trace_id TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plans (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    summary TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '{}',
    created_by_agent TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Tasks
-- ============================================================================

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    priority TEXT,
    owner_agent TEXT,
    skill TEXT,
    description TEXT,
    dependencies TEXT,
    acceptance_criteria TEXT,
    required_context TEXT,
    verification_steps TEXT,
    context_fingerprint TEXT,
    progress_percent INTEGER,
    requires_tdd INTEGER DEFAULT 0,
    input TEXT,
    result TEXT,
    test_evidence TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS task_progress_logs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    agent_name TEXT NOT NULL,
    status_before TEXT,
    status_after TEXT,
    progress_note TEXT NOT NULL,
    evidence TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Review & Session
-- ============================================================================

CREATE TABLE IF NOT EXISTS review_decisions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    reviewer_agent TEXT NOT NULL,
    decision TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    gaps TEXT,
    tests_verified INTEGER DEFAULT 0,
    test_summary TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_states (
    id TEXT PRIMARY KEY,
    session_key TEXT UNIQUE NOT NULL,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    plan_id TEXT REFERENCES plans(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    workflow_loaded INTEGER DEFAULT 0,
    plan_loaded INTEGER DEFAULT 0,
    task_loaded INTEGER DEFAULT 0,
    context_loaded INTEGER DEFAULT 0,
    review_approved INTEGER DEFAULT 0,
    context_fingerprint TEXT,
    execution_mode TEXT,
    active_branch_ids TEXT,
    synthesis_ready INTEGER DEFAULT 0,
    verification_ready INTEGER DEFAULT 0,
    last_command TEXT,
    metadata TEXT DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Parallel Execution
-- ============================================================================

CREATE TABLE IF NOT EXISTS parallel_branches (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    branch_key TEXT NOT NULL DEFAULT 'default',
    role TEXT NOT NULL DEFAULT 'executor',
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    input TEXT NOT NULL DEFAULT '{}',
    output TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Memory & Context
-- ============================================================================

CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    memory_type TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    importance_score REAL DEFAULT 0.5,
    created_by_agent TEXT NOT NULL,
    tags TEXT DEFAULT '[]',
    source TEXT,
    embedding BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    accessed_at TEXT,
    access_count INTEGER DEFAULT 0,
    version INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS context_documents (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_ref TEXT,
    title TEXT,
    content TEXT NOT NULL,
    metadata TEXT DEFAULT '{}',
    fingerprint TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Knowledge Graph
-- ============================================================================

CREATE TABLE IF NOT EXISTS graph_nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL,
    name TEXT UNIQUE NOT NULL,
    properties TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS graph_edges (
    id TEXT PRIMARY KEY,
    source_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    target_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    weight REAL DEFAULT 1.0,
    bidirectional INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Episodic Memory
-- ============================================================================

CREATE TABLE IF NOT EXISTS episodic_memories (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    sequence_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- LLM & Token Tracking
-- ============================================================================

CREATE TABLE IF NOT EXISTS llm_provider_configs (
    id TEXT PRIMARY KEY,
    provider_name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,
    api_key_env_var TEXT NOT NULL,
    models TEXT NOT NULL DEFAULT '[]',
    is_default INTEGER DEFAULT 0,
    priority INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS token_usage (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    route TEXT NOT NULL,
    model TEXT,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    latency_ms INTEGER DEFAULT 0,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Logging & Reminders
-- ============================================================================

CREATE TABLE IF NOT EXISTS retrieval_logs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    agent_name TEXT NOT NULL,
    query TEXT NOT NULL,
    source TEXT NOT NULL,
    results TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workflow_reminders (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    reminder_type TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'MEDIUM',
    message TEXT NOT NULL DEFAULT '',
    acknowledged INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;
