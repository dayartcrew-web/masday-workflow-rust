---
name: masday-context-retrieval
description: >
  Build hybrid context packs by combining semantic search, vector similarity, and exact
  context with fingerprinting. Retrieves the full context needed for a workflow task including
  codebase analysis, stored research, and prior decisions. Use when the user says "build context",
  "load context", "get task context", "context pack", or "prepare context".
allowed-tools:
  - semantic-search_search_hybrid_context_pack
  - semantic-search_code_search
  - semantic-search_search_context_fingerprint
  - workflow_getActive
  - workflow_getPlan
  - workflow_listTasks
  - memory_recall_documents
  - memory_recall_by_task
  - memory_recall_recent
  - memory_search
---

# Masday Context Retrieval

Build comprehensive context packs for workflow task execution.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Identify the active workflow**
   - Call `workflow_getActive` to find the current workflow
   - If no active workflow, ask the user which workflow to build context for

2. **Get the plan and tasks**
   - Call `workflow_getPlan` to retrieve the plan structure
   - Call `workflow_listTasks` to see all tasks and their statuses
   - Identify which task needs context loaded

3. **Build the hybrid context pack**
   - Call `semantic-search_search_hybrid_context_pack` with:
     - `workflow_id`: the active workflow ID
     - `plan_id`: the plan ID from step 2
     - `task_id`: the specific task ID needing context
     - `cwd`: the project root directory
   - This combines: vector similarity search + exact context + fingerprinting

4. **Get context fingerprint**
   - Call `semantic-search_search_context_fingerprint` with:
     - `workflow_id`, `plan_id`, `task_id`
   - This checks if the current context is sufficient or needs refresh
   - Compare fingerprint against prior executions to detect staleness


**GATE**: Verify steps 1-4 are complete before proceeding.

5. **Augment with memory**
   - Call `memory_recall_documents` for stored research related to the workflow
   - Call `memory_recall_by_task` for task-specific prior context
   - Call `memory_recall_recent` for session-level context
   - Call `memory_search` with task-relevant queries for additional context

6. **Search for related code**
   - Call `semantic-search_code_search` with queries derived from the task description
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
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never execute tasks -- this skill only retrieves context
- Never skip the fingerprint check -- stale context leads to errors
- Never return raw search results -- always synthesize into a coherent summary
- Never assume context is sufficient without checking the fingerprint

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow_completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
