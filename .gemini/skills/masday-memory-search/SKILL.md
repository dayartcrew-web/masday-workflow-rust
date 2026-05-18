---
name: masday-memory-search
description: >
  Search and recall stored memories, research, decisions, and artifacts. Supports semantic
  search, filtered recall by type or task, and memory statistics. Use when the user says
  "search memory", "recall decisions", "find research", "what did we decide", or "show past work".
allowed-tools:
  - memory.search
  - memory.recall_recent
  - memory.recall_documents
  - memory.recall_document_by_type
  - memory.recall_by_task
  - memory.stats
  - memory.update
  - memory.delete
---

# Masday Memory Search

Search and manage Masday workflow memory.

## Steps

1. **If type is specified** (e.g., "show decisions", "show artifacts")
   - Call `memory.recall_document_by_type` with:
     - `source_type`: the type filter (e.g., "decision", "artifact", "research", "codebase")
     - `workflow_id`: if scoped to a specific workflow
   - Return the filtered results

2. **If a search query is provided**
   - Call `memory.search` with:
     - `query`: the search query (semantic similarity matching)
     - `type`: optional type filter
     - `tags`: optional tag filter
     - `limit`: max results (default: 10)
     - `threshold`: minimum similarity score (default: 0.5)
   - Results are ranked by relevance score

3. **If asking about recent activity**
   - Call `memory.recall_recent` with:
     - `workflow_id`: optional workflow scope
     - `limit`: max results (default: 10)
   - Shows the most recent memories in chronological order

4. **If asking about a specific task**
   - Call `memory.recall_by_task` with:
     - `task_id`: the task ID to recall context for
     - `limit`: max results

5. **Memory statistics**
   - Call `memory.stats` to show:
     - Total memory count
     - Breakdown by type (decision, artifact, learning, blocker)
     - Average importance score

6. **Manage memories** (if requested)
   - Call `memory.update` to modify an existing memory's content or tags
   - Call `memory.delete` to remove an outdated memory by ID

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
