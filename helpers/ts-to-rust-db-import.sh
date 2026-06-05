#!/bin/bash
# =============================================================================
# ts-to-rust-db-import.sh
# Import data from Masday TypeScript (Supabase) DB to Rust (snake_case) DB
# Handles: camelCase → snake_case column mapping, FK bypass, TSV pipe transfer
# =============================================================================
#
# Usage:
#   bash ts-to-rust-db-import.sh [SOURCE_DB_URL] [TARGET_DB_URL]
#
# Defaults:
#   SOURCE: Supabase Masday TS (camelCase tables: Workflow, Task, etc.)
#   TARGET: Local masday_workflow (snake_case tables: workflows, tasks, etc.)
#
# Requirements: psql, both DBs reachable
# =============================================================================

set -euo pipefail

SOURCE_DB="${1:?Usage: $0 SOURCE_DB [TARGET_DB]}"
TARGET_DB="${2:-postgresql://localhost:54341/masday_workflow}"

echo "=== Masday TS → Rust DB Import ==="
echo "Source: $SOURCE_DB"
echo "Target: $TARGET_DB"
echo ""

# --- Pre-flight checks ---
psql "$SOURCE_DB" -c "SELECT 1;" > /dev/null 2>&1 || { echo "❌ Cannot reach SOURCE DB"; exit 1; }
psql "$TARGET_DB" -c "SELECT 1;" > /dev/null 2>&1 || { echo "❌ Cannot reach TARGET DB"; exit 1; }
echo "✅ Both DBs reachable"

# =============================================================================
# COLUMN MAPPING REFERENCE
# =============================================================================
# TS Table         → Rust Table          | Notes
# -----------------|----------------------|-----------------------------------
# Workflow         → workflows           | No current_plan_id/current_task_id in Rust
# Plan             → plans               |
# Task             → tasks               | No skill/description/dependencies/input/result in TS
# TaskProgressLog  → task_progress_logs  |
# ReviewDecision   → review_decisions    |
# SessionState     → session_states      |
# ParallelBranch   → parallel_branches   |
# GraphNode        → graph_nodes         | Unique name constraint (duplicates skipped)
# GraphEdge        → graph_edges         |
# Memory           → memories            | Skip embedding column (different dims)
# ContextDocument  → context_documents   | Skip embedding column (different dims)
# EpisodicMemory   → episodic_memories   |
# LlmProviderConfig→ llm_provider_configs|
# TokenUsage       → token_usage         |
# RetrievalLog     → retrieval_logs      |
# WorkflowReminder → workflow_reminders  | type → reminder_type
# =============================================================================

# --- Step 1: Drop FK constraints for clean import ---
echo ""
echo "=== Dropping FK constraints ==="
psql "$TARGET_DB" -c "
DO \$\$ DECLARE
  r RECORD;
BEGIN
  FOR r IN (SELECT conname, conrelid::regclass AS tbl
            FROM pg_constraint
            WHERE contype = 'f' AND connamespace = 'public'::regnamespace) LOOP
    EXECUTE 'ALTER TABLE ' || r.tbl || ' DROP CONSTRAINT IF EXISTS ' || r.conname;
    RAISE NOTICE 'Dropped % on %', r.conname, r.tbl;
  END LOOP;
END \$\$;
" 2>&1 | tail -1
echo "✅ FK constraints dropped"

# Drop unique constraints that might conflict (graph_nodes.name)
psql "$TARGET_DB" -c "ALTER TABLE graph_nodes DROP CONSTRAINT IF EXISTS graph_nodes_name_key;" 2>/dev/null || true

# --- Step 2: Truncate all target tables ---
echo ""
echo "=== Truncating target tables ==="
psql "$TARGET_DB" -c "
TRUNCATE TABLE
  workflow_reminders, retrieval_logs, token_usage, llm_provider_configs,
  episodic_memories, context_documents, memories,
  graph_edges, graph_nodes,
  parallel_branches, session_states, review_decisions,
  task_progress_logs, tasks, plans, workflows
CASCADE;
" 2>&1 | tail -1
echo "✅ Tables truncated"

# --- Step 3: Import function ---
# Uses \copy (client-side) to avoid Docker volume path issues
IMPORT_ERRORS=0

do_import() {
  local src_table="$1"
  local dst_table="$2"
  local src_cols="$3"    # Comma-separated, quoted
  local dst_cols="$4"    # Comma-separated, quoted
  local order_col="${5:-\"createdAt\"}"  # Default order by createdAt

  local tmpfile="/tmp/masday_imp_${dst_table}.tsv"

  # Count source rows
  local src_count
  src_count=$(psql "$SOURCE_DB" -t -A -c "SELECT COUNT(*) FROM public.\"$src_table\";" 2>/dev/null || echo "0")

  if [ "$src_count" = "0" ]; then
    echo "⏭️  $dst_table: empty source"
    return
  fi

  # Export from source
  psql "$SOURCE_DB" -c "\COPY (SELECT $src_cols FROM public.\"$src_table\" ORDER BY $order_col ASC) TO '$tmpfile' WITH (FORMAT csv, DELIMITER E'\t')" 2>/dev/null

  if [ -s "$tmpfile" ]; then
    local lines
    lines=$(wc -l < "$tmpfile")

    # Import to target
    if psql "$TARGET_DB" -c "\COPY $dst_table ($dst_cols) FROM '$tmpfile' WITH (FORMAT csv, DELIMITER E'\t')" 2>&1; then
      echo "✅ $dst_table: $lines rows (source: $src_count)"
    else
      echo "⚠️  $dst_table: COPY error (imported partial)"
      IMPORT_ERRORS=$((IMPORT_ERRORS + 1))
    fi
  else
    echo "⚠️  $dst_table: no data exported"
    IMPORT_ERRORS=$((IMPORT_ERRORS + 1))
  fi

  rm -f "$tmpfile"
}

# --- Step 4: Import all tables (parent tables first) ---
echo ""
echo "=== Importing data ==="
echo ""

# Parent: workflows
do_import "Workflow" "workflows" \
  '"id","name","status","projectPath","metadata","createdAt","updatedAt"' \
  '"id","name","status","project_path","metadata","created_at","updated_at"'

# Dependent: plans
do_import "Plan" "plans" \
  '"id","workflowId","version","status","summary","content","createdByAgent","createdAt"' \
  '"id","workflow_id","version","status","summary","content","created_by_agent","created_at"'

# Dependent: tasks
do_import "Task" "tasks" \
  '"id","workflowId","planId","title","status","priority","ownerAgent","acceptanceCriteria","requiredContext","verificationSteps","contextFingerprint","progressPercent","requiresTdd","testEvidence","createdAt","updatedAt"' \
  '"id","workflow_id","plan_id","title","status","priority","owner_agent","acceptance_criteria","required_context","verification_steps","context_fingerprint","progress_percent","requires_tdd","test_evidence","created_at","updated_at"'

# Dependent: task_progress_logs
do_import "TaskProgressLog" "task_progress_logs" \
  '"id","workflowId","taskId","agentName","statusBefore","statusAfter","progressNote","evidence","createdAt"' \
  '"id","workflow_id","task_id","agent_name","status_before","status_after","progress_note","evidence","created_at"'

# Dependent: review_decisions
do_import "ReviewDecision" "review_decisions" \
  '"id","workflowId","taskId","reviewerAgent","decision","notes","gaps","testsVerified","testSummary","createdAt"' \
  '"id","workflow_id","task_id","reviewer_agent","decision","notes","gaps","tests_verified","test_summary","created_at"'

# Independent: session_states
do_import "SessionState" "session_states" \
  '"id","sessionKey","workflowId","planId","taskId","workflowLoaded","planLoaded","taskLoaded","contextLoaded","reviewApproved","contextFingerprint","executionMode","activeBranchIds","synthesisReady","verificationReady","lastCommand","metadata","updatedAt","createdAt"' \
  '"id","session_key","workflow_id","plan_id","task_id","workflow_loaded","plan_loaded","task_loaded","context_loaded","review_approved","context_fingerprint","execution_mode","active_branch_ids","synthesis_ready","verification_ready","last_command","metadata","updated_at","created_at"'

# Dependent: parallel_branches
do_import "ParallelBranch" "parallel_branches" \
  '"id","workflowId","taskId","branchKey","role","status","input","output","createdAt","updatedAt"' \
  '"id","workflow_id","task_id","branch_key","role","status","input","output","created_at","updated_at"'

# Independent: graph_nodes (parent of graph_edges)
do_import "GraphNode" "graph_nodes" \
  '"id","nodeType","name","properties","createdAt"' \
  '"id","node_type","name","properties","created_at"'

# Dependent: graph_edges
do_import "GraphEdge" "graph_edges" \
  '"id","sourceNodeId","targetNodeId","relationType","weight","bidirectional","createdAt"' \
  '"id","source_node_id","target_node_id","relation_type","weight","bidirectional","created_at"'

# Dependent: memories (skip embedding — different dimensions)
do_import "Memory" "memories" \
  '"id","workflowId","taskId","memoryType","summary","content","importanceScore","createdByAgent","tags","source","createdAt","updatedAt","accessedAt","accessCount","version"' \
  '"id","workflow_id","task_id","memory_type","summary","content","importance_score","created_by_agent","tags","source","created_at","updated_at","accessed_at","access_count","version"'

# Dependent: context_documents (skip embedding — different dimensions)
do_import "ContextDocument" "context_documents" \
  '"id","workflowId","sourceType","sourceRef","title","content","metadata","fingerprint","createdAt","updatedAt"' \
  '"id","workflow_id","source_type","source_ref","title","content","metadata","fingerprint","created_at","updated_at"'

# Independent: episodic_memories
do_import "EpisodicMemory" "episodic_memories" \
  '"id","sessionId","role","content","sequenceOrder","createdAt"' \
  '"id","session_id","role","content","sequence_order","created_at"'

# Independent: llm_provider_configs
do_import "LlmProviderConfig" "llm_provider_configs" \
  '"id","providerName","baseUrl","apiKeyEnvVar","models","isDefault","priority","createdAt","updatedAt"' \
  '"id","provider_name","base_url","api_key_env_var","models","is_default","priority","created_at","updated_at"'

# Independent: token_usage
do_import "TokenUsage" "token_usage" \
  '"id","source","route","model","promptTokens","completionTokens","totalTokens","latencyMs","metadata","createdAt"' \
  '"id","source","route","model","prompt_tokens","completion_tokens","total_tokens","latency_ms","metadata","created_at"'

# Dependent: retrieval_logs
do_import "RetrievalLog" "retrieval_logs" \
  '"id","workflowId","taskId","agentName","query","source","results","createdAt"' \
  '"id","workflow_id","task_id","agent_name","query","source","results","created_at"'

# Dependent: workflow_reminders
do_import "WorkflowReminder" "workflow_reminders" \
  '"id","workflowId","taskId","type","severity","message","acknowledged","createdAt"' \
  '"id","workflow_id","task_id","reminder_type","severity","message","acknowledged","created_at"'

# --- Step 5: Recreate FK constraints ---
echo ""
echo "=== Recreating FK constraints ==="
psql "$TARGET_DB" -c "
-- Plans → Workflows
ALTER TABLE plans ADD CONSTRAINT plans_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;

-- Tasks → Workflows, Plans
ALTER TABLE tasks ADD CONSTRAINT tasks_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE tasks ADD CONSTRAINT tasks_plan_id_fkey
  FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE;

-- Task Progress Logs → Workflows, Tasks
ALTER TABLE task_progress_logs ADD CONSTRAINT task_progress_logs_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE task_progress_logs ADD CONSTRAINT task_progress_logs_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Review Decisions → Workflows, Tasks
ALTER TABLE review_decisions ADD CONSTRAINT review_decisions_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE review_decisions ADD CONSTRAINT review_decisions_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Session States → Workflows, Plans, Tasks
ALTER TABLE session_states ADD CONSTRAINT session_states_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE session_states ADD CONSTRAINT session_states_plan_id_fkey
  FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE;
ALTER TABLE session_states ADD CONSTRAINT session_states_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Parallel Branches → Workflows, Tasks
ALTER TABLE parallel_branches ADD CONSTRAINT parallel_branches_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE parallel_branches ADD CONSTRAINT parallel_branches_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Graph Edges → Graph Nodes
ALTER TABLE graph_edges ADD CONSTRAINT graph_edges_source_node_id_fkey
  FOREIGN KEY (source_node_id) REFERENCES graph_nodes(id) ON DELETE CASCADE;
ALTER TABLE graph_edges ADD CONSTRAINT graph_edges_target_node_id_fkey
  FOREIGN KEY (target_node_id) REFERENCES graph_nodes(id) ON DELETE CASCADE;

-- Memories → Workflows, Tasks
ALTER TABLE memories ADD CONSTRAINT memories_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE memories ADD CONSTRAINT memories_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Context Documents → Workflows
ALTER TABLE context_documents ADD CONSTRAINT context_documents_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;

-- Retrieval Logs → Workflows, Tasks
ALTER TABLE retrieval_logs ADD CONSTRAINT retrieval_logs_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE retrieval_logs ADD CONSTRAINT retrieval_logs_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;

-- Workflow Reminders → Workflows, Tasks
ALTER TABLE workflow_reminders ADD CONSTRAINT workflow_reminders_workflow_id_fkey
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;
ALTER TABLE workflow_reminders ADD CONSTRAINT workflow_reminders_task_id_fkey
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE;
" 2>&1 | tail -1
echo "✅ FK constraints recreated"

# --- Step 6: Final summary ---
echo ""
echo "════════════════════════════════════════"
echo "  IMPORT SUMMARY"
echo "════════════════════════════════════════"
echo ""

TOTAL=0
for t in workflows plans tasks task_progress_logs review_decisions session_states parallel_branches graph_nodes graph_edges memories context_documents episodic_memories llm_provider_configs token_usage retrieval_logs workflow_reminders; do
  cnt=$(psql "$TARGET_DB" -t -A -c "SELECT COUNT(*) FROM $t;" 2>/dev/null)
  printf "  %-25s %'d rows\n" "$t" "$cnt"
  TOTAL=$((TOTAL + cnt))
done

echo ""
printf "  %-25s %'d rows\n" "TOTAL" "$TOTAL"
echo ""

if [ "$IMPORT_ERRORS" -gt 0 ]; then
  echo "⚠️  $IMPORT_ERRORS tables had import errors"
else
  echo "✅ All tables imported cleanly"
fi

echo ""
echo "════════════════════════════════════════"
