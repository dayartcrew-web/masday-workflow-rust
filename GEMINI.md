<!-- masday-session-context -->
# masday-workflow-rebuild Active Session

This project uses **masday-workflow-rebuild** for workflow management.

## Commands

| Command | Purpose |
|---------|---------|
| "/msd-start-work" | Start new or load existing workflow |
| "/msd-continue" | Resume active workflow |
| "/msd-implement" | Execute current task with scope discipline |
| "/msd-plan" | Create or refine implementation plan |
| "/msd-review" | Quality gate against acceptance criteria |
| "/msd-verify" | Pre-completion verification |
| "/msd-status" | Read-only workflow diagnostic |
| "/msd-debug" | Deep diagnosis + auto-fix |
| "/msd-health" | Quick infrastructure check |

## Workflow Pattern

1. `workflow_getActive` → `workflow_getPlan` → `workflow_getCurrentTask`
2. `semantic-search_search_hybrid_context_pack` to build task context
3. Execute / Research / Review
4. `workflow_saveProgress` → `review_submit` → `workflow_completeTask`

## Key MCP Namespace

- `mcp__masday__*` — Unified MCP server (87 tools across 17 namespaces)

## Persistence

All tools use **DualWriteStore**: local cache (SQLite/JSON) + PostgreSQL (Drizzle) in real-time.
- Workflow operations replicate to Supabase PostgreSQL via `DualWriteWorkflowStore`
- Memory tools use hybrid mode: Drizzle first, JSON cache fallback
- Review, session, policy tools read/write directly to PostgreSQL tables
- Shell tools (git, npm, docker, cicd, github, tests) use real `execSync` calls
- **TaskProgressLog** populated via `saveProgressDb()` on `workflow_saveProgress`
- **RetrievalLog** populated via `logRetrieval()` on `memory_search`, `semantic-search_code_search`, `search_hybrid_context_pack`
- **ContextDocument** populated via `drizzle().insert(contextDocument)` on `memory_store_research`
- **TokenUsage** populated via `trackTokens()` on key tool calls (saveProgress, store_research)
- **EpisodicMemory** populated via `setEpisodicDrizzle()` in `EpisodicMemory.add()`
- **GraphNode/GraphEdge** populated via `setGraphDrizzle()` in `GraphStore.addNode()/addEdge()`

**Status Conventions (ALL UPPERCASE in PostgreSQL):**
- Workflow: INIT, ANALYZE, PLAN, EXECUTE, VERIFY, FIX, DONE, FAILED, PAUSED
- Task: PENDING, RUNNING, DONE, FAILED
- Plan: ACTIVE, PENDING, READY, DONE
- Review: APPROVED, REWORK_REQUIRED, BLOCKED

## Tool Namespaces (87 tools)

| Namespace | Count | Key Tools |
|-----------|-------|-----------|
| workflow | 23 | create, execute, getStatus, get, list, addTask, startTask, completeTask, saveProgress, listTasks, getCurrentTask, getPlan, getActive, createPlan, createParallelBranches, completeParallelBranch, listParallelBranches, delete, ping, set_execution_mode, mark_synthesis_ready, mark_verification_ready, resume_suggestion |
| memory | 11 | store, store_research, recall_recent, recall_documents, recall_document_by_type, recall_by_task, update, delete, delete_by_workflow, search, stats |
| semantic-search | 3 | search_hybrid_context_pack, search_context_fingerprint, code_search |
| policy | 6 | check_session_readiness, validate_execution, validate_completion, validate_parallel_completion, detect_scope_drift, require_context_refresh |
| capability | 11 | list_agents, list_skills, list_templates, match_agent, system_readiness, workflow_audit, create_agent, create_skill, scaffold_feature, scaffold_mcp_server, ping |
| filesystem | 5 | read, write, list, delete, stat |
| review | 2 | submit, get_latest |
| session | 3 | get_state, patch_state, init_context |
| local | 4 | init, sync, push, save_artifact |
| git | 3 | status, diff, commit |
| npm | 2 | install, run |
| docker | 3 | build, run, ps |
| cicd | 3 | pipeline_status, pipeline_trigger, runs_view |
| github | 3 | pr_create, pr_list, issue_list |
| tests | 1 | run |
| reminder | 3 | check, list, acknowledge |
| projectRules | 1 | check |

## Naming Convention

- Tool names use **camelCase** dot-namespaced format: `workflow_getActive`, `memory_store`
- MCP SDK resolves: dots → underscores → `mcp__masday__workflow_getActive`
- In .md docs, always use the logical name: `workflow_getActive`
- **NEVER** use snake_case: `workflow_get_active` is WRONG

## Package Scope

All packages use `@mcp-rebuild/*` scope.

## Build

```bash
pnpm install
pnpm db:generate
pnpm build
```

<!-- masday-session-context -->


<!-- msd-mcp-session-context -->
# msd-mcp Active Session

This project uses **msd-mcp** (Multi-Agent MCP Runtime) for workflow management.

For ALL tasks, use msd-mcp commands and MCP tools.

## Commands

| Command | Purpose |
|---------|---------|
| `/msd-start-work` | Start new or load existing workflow |
| `/msd-continue` | Resume active workflow |
| `/msd-implement` | Execute current task with scope discipline |
| `/msd-plan` | Create or refine implementation plan |
| `/msd-review` | Quality gate against acceptance criteria |
| `/msd-verify` | Pre-completion verification |
| `/msd-status` | Read-only workflow diagnostic |
| `/msd-doctor` | Deep diagnosis + auto-fix |
| `/msd-health` | Quick infrastructure check |
| `/msd-autopilot` | Auto-execute all tasks |

## Workflow Pattern

1. `workflow_get_active` → `workflow_get_plan` → `workflow_get_current_task`
2. `search.hybrid_context_pack` to build task context
3. Execute / Research / Review
4. `workflow_save_progress` → `review_submit` → `workflow_complete_task`

## Key MCP Namespaces

- `mcp__workflow-orchestrator__*` — Workflow, plan, task, session, parallel, local (26 tools)
- `mcp__memory__*` — Store, recall, research memory (9 tools)
- `mcp__semantic-search__*` — Context packs, fingerprints (2 tools)
- `mcp__policy__*` — Validation, drift, readiness (6 tools)
- `mcp__capability__*` — Health, factory, scaffold (10 tools)

<!-- msd-mcp-session-context -->
