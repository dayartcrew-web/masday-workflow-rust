---
name: masday-context-retrieval
description: >
  Build hybrid context packs by combining semantic search, vector similarity, and exact
  context with fingerprinting. Retrieves the full context needed for a workflow task including
  codebase analysis, stored research, and prior decisions. Use when the user says "build context",
  "load context", "get task context", "context pack", or "prepare context".
allowed-tools:
  - search.hybrid_context_pack
  - search.code_search
  - search.context_fingerprint
  - workflow.get_active
  - workflow.get_plan
  - workflow.list_tasks
  - memory.recall_documents
  - memory.recall_by_task
  - memory.recall_recent
  - memory.search
---

# Masday Context Retrieval

Build comprehensive context packs for workflow task execution.

## Steps

1. **Identify the active workflow**
   - Call `workflow.get_active` to find the current workflow
   - If no active workflow, ask the user which workflow to build context for

2. **Get the plan and tasks**
   - Call `workflow.get_plan` to retrieve the plan structure
   - Call `workflow.list_tasks` to see all tasks and their statuses
   - Identify which task needs context loaded

3. **Build the hybrid context pack**
   - Call `search.hybrid_context_pack` with:
     - `workflow_id`: the active workflow ID
     - `plan_id`: the plan ID from step 2
     - `task_id`: the specific task ID needing context
     - `cwd`: the project root directory
   - This combines: vector similarity search + exact context + fingerprinting

4. **Get context fingerprint**
   - Call `search.context_fingerprint` with:
     - `workflow_id`, `plan_id`, `task_id`
   - This checks if the current context is sufficient or needs refresh
   - Compare fingerprint against prior executions to detect staleness

5. **Augment with memory**
   - Call `memory.recall_documents` for stored research related to the workflow
   - Call `memory.recall_by_task` for task-specific prior context
   - Call `memory.recall_recent` for session-level context
   - Call `memory.search` with task-relevant queries for additional context

6. **Search for related code**
   - Call `search.code_search` with queries derived from the task description
   - Identify the most relevant files, functions, and types

7. **Assemble and report**
   - Combine all sources into a structured context summary:
   ```
   === Context Pack ===
   Workflow: [wf-001] "Add auth module"
   Task: "Implement JWT middleware"

   Codebase hits:
   - packages/core/src/types.ts (shared types)
   - packages/orchestrator/src/middleware.ts (existing middleware pattern)
   - packages/store/src/index.ts (storage interface)

   Memory:
   - 3 decisions from prior tasks
   - 2 research documents
   - 1 related artifact

   Fingerprint: current (no refresh needed)
   Sufficiency: HIGH (all required context loaded)
   ```

## Never

- Never execute tasks -- this skill only retrieves context
- Never skip the fingerprint check -- stale context leads to errors
- Never return raw search results -- always synthesize into a coherent summary
- Never assume context is sufficient without checking the fingerprint
