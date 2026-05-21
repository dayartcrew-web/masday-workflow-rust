---
name: masday-memory-search
description: >
  Search and recall stored memories, research, decisions, and artifacts. Supports semantic
  search, filtered recall by type or task, and memory statistics. Use when the user says
  "search memory", "recall decisions", "find research", "what did we decide", or "show past work".
allowed-tools:
  - memory_search
  - memory_recall_recent
  - memory_recall_documents
  - memory_recall_document_by_type
  - memory_recall_by_task
  - memory_stats
  - memory_update
  - memory_delete
---

# Masday Memory Search

Search and manage Masday workflow memory.

## Steps

1. **If type is specified** (e.g., "show decisions", "show artifacts")
   - Call `memory_recall_document_by_type` with:
     - `workflow_id`: if scoped to a specific workflow
     - `source_type`: the type filter (e.g., "decision", "artifact", "research", "codebase")
     - `limit`: max results (default: 10)
   - Return the filtered results

2. **If a search query is provided**
   - Call `memory_search` with:
     - `query`: the search query (semantic similarity matching)
     - `limit`: max results (default: 10)
   - Results are ranked by relevance score

3. **If asking about recent activity**
   - Call `memory_recall_recent` with:
     - `workflow_id`: optional workflow scope
     - `limit`: max results (default: 10)
   - Shows the most recent memories in chronological order

4. **If asking about a specific task**
   - Call `memory_recall_by_task` with:
     - `task_id`: the task ID to recall context for
     - `limit`: max results

5. **Memory statistics**
   - Call `memory_stats` to show:
     - Total memory count
     - Breakdown by type (decision, artifact, learning, blocker)
     - Average importance score

6. **Manage memories** (if requested)
   - Call `memory_update` to modify an existing memory's content or tags
   - Call `memory_delete` to remove an outdated memory by ID

7. **Report findings**
   - Format results clearly with type, date, relevance score, and summary
   - Group by type if multiple types returned
   ```
   === Memory Results ===
   Query: "authentication decisions"

   1. [decision] 2026-05-16 | score: 0.92
      "Use JWT with RS256 for auth tokens"

   2. [artifact] 2026-05-15 | score: 0.85
      "Auth middleware implementation at packages/core/src/auth.ts"

   Memory stats: 47 entries | avg importance: 0.72
   ```

## Never

- Never delete memories without user confirmation
- Never show raw JSON -- format as human-readable results
- Never modify memories without preserving the original context
- Never skip the relevance score display for search results

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
