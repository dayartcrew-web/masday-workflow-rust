-- Masday Workflow Database - Initial Schema
-- This migration creates all 16 tables for the masday-workflow system.
-- Field names match Rust schema structs (snake_case) and TypeScript Drizzle schema (camelCase).

-- Enable pgvector extension for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- ============================================================================
-- Workflow & Planning Tables
-- ============================================================================

-- Workflows table: Stores workflow execution state with status tracking
CREATE TABLE workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL, -- INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
    project_path TEXT,
    trace_id TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for workflows
CREATE INDEX idx_workflows_status ON workflows(status);
CREATE INDEX idx_workflows_project_path ON workflows(project_path);

-- Plans table: Stores plan versions with phases and content
CREATE TABLE plans (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    status TEXT NOT NULL, -- ACTIVE, PENDING, READY, DONE
    summary TEXT NOT NULL,
    content JSONB NOT NULL,
    created_by_agent TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- Task Tables
-- ============================================================================

-- Tasks table: Stores tasks with TDD support and progress tracking
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL, -- PENDING, RUNNING, DONE, FAILED
    priority TEXT, -- LOW, MEDIUM, HIGH, CRITICAL
    owner_agent TEXT,
    skill TEXT,
    description TEXT,
    dependencies JSONB, -- Array of task IDs
    acceptance_criteria JSONB, -- Array of criteria strings
    required_context JSONB, -- Array of context requirements
    verification_steps JSONB, -- Array of verification steps
    context_fingerprint TEXT,
    progress_percent INTEGER,
    requires_tdd BOOLEAN,
    input JSONB,
    result JSONB,
    test_evidence JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for tasks
CREATE INDEX idx_tasks_workflow_id ON tasks(workflow_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_plan_id ON tasks(plan_id);

-- Task progress logs table: Tracks progress updates with evidence capture
CREATE TABLE task_progress_logs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    agent_name TEXT NOT NULL,
    status_before TEXT,
    status_after TEXT,
    progress_note TEXT NOT NULL,
    evidence JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- Review & Session Tables
-- ============================================================================

-- Review decisions table: Stores code review decisions with gap analysis
CREATE TABLE review_decisions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    reviewer_agent TEXT NOT NULL,
    decision TEXT NOT NULL, -- APPROVED, REWORK_REQUIRED, BLOCKED
    notes TEXT NOT NULL,
    gaps JSONB, -- Array of identified gaps
    tests_verified BOOLEAN,
    test_summary JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Session states table: Tracks session state with loading flags and execution mode
CREATE TABLE session_states (
    id TEXT PRIMARY KEY,
    session_key TEXT UNIQUE NOT NULL,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    plan_id TEXT REFERENCES plans(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    workflow_loaded BOOLEAN,
    plan_loaded BOOLEAN,
    task_loaded BOOLEAN,
    context_loaded BOOLEAN,
    review_approved BOOLEAN,
    context_fingerprint TEXT,
    execution_mode TEXT, -- sequential, parallel, autopilot
    active_branch_ids JSONB, -- Array of active branch IDs
    synthesis_ready BOOLEAN,
    verification_ready BOOLEAN,
    last_command TEXT,
    metadata JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- Parallel Execution Tables
-- ============================================================================

-- Parallel branches table: Stores parallel execution branches within workflows
CREATE TABLE parallel_branches (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    branch_key TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL, -- PENDING, RUNNING, DONE, FAILED
    input JSONB NOT NULL,
    output JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- Memory & Context Tables
-- ============================================================================

-- Memories table: Stores long-term memories with importance scoring and tags
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    memory_type TEXT NOT NULL, -- fact, preference, skill, experience, strategy
    summary TEXT NOT NULL,
    content TEXT NOT NULL,
    importance_score DOUBLE PRECISION,
    created_by_agent TEXT NOT NULL,
    tags TEXT[], -- Array of tags for filtering
    source TEXT,
    embedding vector(768), -- pgvector for semantic search (768-dim, configurable)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    accessed_at TIMESTAMPTZ,
    access_count INTEGER DEFAULT 0,
    version INTEGER DEFAULT 1
);

-- Indexes for memories
CREATE INDEX idx_memories_workflow_id ON memories(workflow_id);
CREATE INDEX idx_memories_type ON memories(memory_type);
CREATE INDEX idx_memories_tags ON memories USING GIN(tags);

-- Context documents table: Stores context documents with fingerprinting and embeddings
CREATE TABLE context_documents (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL, -- file, url, code_search, memory_store
    source_ref TEXT,
    title TEXT,
    content TEXT NOT NULL,
    metadata JSONB,
    fingerprint TEXT,
    embedding vector(1536), -- pgvector for similarity search
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- Knowledge Graph Tables
-- ============================================================================

-- Graph nodes table: Stores nodes in the knowledge graph
CREATE TABLE graph_nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL, -- workflow, task, memory, concept, agent, skill
    name TEXT UNIQUE NOT NULL,
    properties JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Graph edges table: Stores edges between nodes in the knowledge graph
CREATE TABLE graph_edges (
    id TEXT PRIMARY KEY,
    source_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    target_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL, -- contains, depends_on, relates_to, similar_to
    weight DOUBLE PRECISION,
    bidirectional BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for graph edges
CREATE INDEX idx_graph_edges_source ON graph_edges(source_node_id);
CREATE INDEX idx_graph_edges_target ON graph_edges(target_node_id);
CREATE INDEX idx_graph_edges_relation ON graph_edges(relation_type);

-- ============================================================================
-- Episodic Memory Tables
-- ============================================================================

-- Episodic memories table: Stores episodic memories per session with sequence ordering
CREATE TABLE episodic_memories (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL, -- user, assistant, system, tool
    content TEXT NOT NULL,
    sequence_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for episodic memories
CREATE INDEX idx_episodic_session_id ON episodic_memories(session_id);

-- ============================================================================
-- LLM & Token Tracking Tables
-- ============================================================================

-- LLM provider configs table: Stores LLM provider configuration with models and priority
CREATE TABLE llm_provider_configs (
    id TEXT PRIMARY KEY,
    provider_name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,
    api_key_env_var TEXT NOT NULL,
    models JSONB NOT NULL, -- Array of model configurations
    is_default BOOLEAN DEFAULT FALSE,
    priority INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Token usage table: Tracks token usage per source, route, and model
CREATE TABLE token_usage (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL, -- anthropic, openai, custom
    route TEXT NOT NULL, -- API endpoint or skill name
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    latency_ms INTEGER,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for token usage
CREATE INDEX idx_token_usage_source ON token_usage(source);
CREATE INDEX idx_token_usage_created_at ON token_usage(created_at);

-- ============================================================================
-- Logging & Reminder Tables
-- ============================================================================

-- Retrieval logs table: Logs retrieval operations for context and semantic search
CREATE TABLE retrieval_logs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    agent_name TEXT NOT NULL,
    query TEXT NOT NULL,
    source TEXT NOT NULL, -- memory, semantic_search, code_search, hybrid
    results JSONB, -- Array of retrieval results
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for retrieval logs
CREATE INDEX idx_retrieval_workflow_id ON retrieval_logs(workflow_id);
CREATE INDEX idx_retrieval_created_at ON retrieval_logs(created_at);

-- Workflow reminders table: Stores reminders for workflows with severity levels
CREATE TABLE workflow_reminders (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    reminder_type TEXT NOT NULL, -- stale, stuck, failed, timeout
    severity TEXT NOT NULL, -- LOW, MEDIUM, HIGH, CRITICAL
    message TEXT NOT NULL,
    acknowledged BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for workflow reminders
CREATE INDEX idx_reminders_workflow_id ON workflow_reminders(workflow_id);
CREATE INDEX idx_reminders_acknowledged ON workflow_reminders(acknowledged);
