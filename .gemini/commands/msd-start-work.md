Start a new structured workflow or load an existing one.

## Purpose

Entry point for all work. Initializes the full MCP pipeline: workflow state, plan, task, context, and agent delegation.

## Steps

### 0. Initialize Local State (MANDATORY — do this first)
```
CRITICAL: You MUST create .msd/ before proceeding. Use ONE of these:

Option A (preferred): Call MCP tool workflow-orchestrator local.init:
  mcp__workflow-orchestrator__local_init({ cwd: process.cwd() })

Option B (fallback): Run via bash:
  pnpm manage init

After init, verify .msd/ exists:
  ls .msd/ should show: context/ plans/ reports/ artifacts/

DO NOT skip this step. If .msd/ does not exist after this step, STOP and report the error.
```

### 1. Check for Active Workflow
```
Call mcp__workflow-orchestrator__workflow_get_active with { cwd: process.cwd() }
```
- If active workflow exists → proceed to step 3 (resume)
- If no active workflow → proceed to step 2 (create)

### 2. Create New Workflow (if none exists)
```
Ask user for:
- Title (short, imperative: "Add JWT auth to API endpoints")
- Description (1-3 sentences of what and why)

Call mcp__workflow-orchestrator__workflow_create with { name: title, cwd: process.cwd() }
Record the returned workflowId
```

### 3. Load Plan
```
Call mcp__workflow-orchestrator__workflow_get_plan with { workflow_id: workflowId }

If no plan exists:
  → Inform user that planning is needed
  → Suggest running /msd-plan first
  → STOP here (do not proceed without a plan)

If plan exists:
  → Read all tasks and their statuses
  → Identify which task is CURRENT (first non-completed)
```

### 4. Load Current Task
```
Call mcp__workflow-orchestrator__workflow_get_current_task with { workflow_id: workflowId }

If no current task:
  → Call mcp__workflow-orchestrator__workflow_list_tasks({ workflow_id: workflowId }) to find first pending task
  → Call mcp__workflow-orchestrator__workflow_start_task({ workflow_id: workflowId, task_id: taskId }) to activate it
  → Record taskId

If current task exists:
  → Read title, acceptance criteria, required context
  → Record taskId
```

### 5. Assemble Context
```
Call mcp__semantic-search__search_hybrid_context_pack with:
{
  workflow_id: "{workflowId}",
  plan_id: "{planId}",
  task_id: "{taskId}",
  cwd: process.cwd()
}

Review returned context:
- If sufficient → mark context as loaded via mcp__workflow-orchestrator__session_patch_state
- If insufficient → proceed to step 6 (research)

Context is auto-saved to .msd/context/codebase/context-pack.md by the MCP tool.
Verify it was saved:
  Read .msd/context/codebase/context-pack.md
  If missing → call mcp__workflow-orchestrator__local_save_artifact({
    cwd: process.cwd(), category: "context/codebase", filename: "context-pack.md",
    content: "<summary of assembled context>"
  })
```

### 6. Research (if context insufficient)
```
Identify knowledge gaps from context pack review.

For library/framework questions:
  → Use Context7: resolve-library-id → query-docs
  → AFTER research, save findings:
    1. DB: mcp__memory__memory_store_research({ workflow_id: workflowId, summary, content, created_by_agent: "msd-researcher" })
    2. File: Write tool → .msd/context/research/<topic>.md

For general knowledge:
  → Use WebSearch with specific queries
  → Store findings via mcp__memory__memory_store_research (DB) + Write tool → .msd/context/research/

For codebase patterns:
  → Use Grep/Glob to find existing patterns
  → Read 3-5 relevant source files

After research:
  → Call mcp__workflow-orchestrator__session_patch_state with { contextLoaded: true }
```

### 7. Delegate to Agent
```
Based on task type, dispatch to the appropriate agent:

| Task Type | Agent | Model |
|-----------|-------|-------|
| Planning | msd-planner | sonnet |
| Research | msd-researcher | sonnet |
| Implementation | msd-executor | sonnet |
| Review | msd-reviewer | sonnet |
| Testing | msd-tester | sonnet |
| Debugging | msd-debugger | sonnet |

Include in dispatch:
- workflowId
- taskId
- Loaded context summary
- Acceptance criteria
- Specific instructions for the agent role
- CRITICAL: Agent MUST write artifacts to .msd/ using Write tool:
  - Plans → Write tool → .msd/plans/<filename>.md
  - Research → Write tool → .msd/context/research/<filename>.md
  - Reports → Write tool → .msd/reports/<filename>.md
  - Analysis → Write tool → .msd/context/codebase/<filename>.md
```

### 8. Save Progress and Sync
```
After agent completes its work:

Step 8a — Save progress to DB:
  mcp__workflow-orchestrator__workflow_save_progress({
    workflow_id: workflowId,
    task_id: taskId,
    agent_name: "{delegated agent}",
    progress_note: "Summary of what was accomplished",
    evidence: ["files modified", "tests run", "build status"]
  })

Step 8b — Save progress summary as local artifact:
  Write tool → .msd/reports/progress-<task-title-slug>.md
  Content: "<progress note, evidence, and key decisions>"

Step 8c — Push local changes to DB (if any local state divergence):
  mcp__workflow-orchestrator__local_push({ cwd: process.cwd(), workflow_id: workflowId })

Step 8d — Sync DB state to local:
  Option A: mcp__workflow-orchestrator__local_sync({ cwd: process.cwd(), workflow_id: workflowId })
  Option B: pnpm manage sync

Step 8e — Update session state:
  mcp__workflow-orchestrator__session_patch_state({ contextLoaded: true, taskLoaded: true })

This 4-step save ensures:
- DB has structured state (save_progress)
- .msd/ has human-readable reports (save_artifact)
- state.json is in sync (local_sync)
- Session flags are current (patch_state)
```

## Error Handling

| Error | Action |
|-------|--------|
| No database connection | Check Docker, verify .env DATABASE_URL |
| workflow.create fails | Check required fields, retry |
| No plan found | Stop and suggest /msd-plan |
| Context pack empty | Expand search query, try broader terms |
| Agent dispatch fails | Verify agent exists in .claude/agents/ |
| Write to .msd/ fails | Check .msd/ exists (re-run local.init if needed) |

## Output

After completion, report:
```
Workflow: {title} ({workflowId})
Plan: {task count} tasks, {completed} completed
Current Task: {title} ({status})
Context: {loaded/researched}
Artifacts: {list files written to .msd/}
Agent: {delegated agent name}
Next Step: {what the user should do next}
```
