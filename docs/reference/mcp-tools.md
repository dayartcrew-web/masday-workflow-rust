# MCP Tools Reference

> **Last updated:** v0.3.69 (2026-06-12) — 96 tools across 20 domains, 243 API routes

This page is the canonical contributor-facing reference for the MCP tool surface. The server is `masday-mcp` (Rust, JSON-RPC 2.0 over stdio) with `masday-api` (Axum HTTP API on port 30101). All tools are connected to PostgreSQL via `tokio-postgres` with repos in `masday-db/src/repos/`.

## Operation Modes

| Mode | Tools | Architecture |
|------|-------|--------------|
| **HTTP Proxy** | 31 | Tool → HTTP API → Service → Repo → PostgreSQL |
| **Standalone** | 15 | Tool → Direct Functions → SQLite (stdio mode) |
| **External CLI** | 50 | Tool → Shell Commands → External Systems |

## Persistence

- **WorkflowRepo**: all workflow/task/plan operations via `tokio-postgres` with PascalCase tables
- **Memory**: `MemoryRepo` with importance scoring, text search, and embedding support
- **BranchRepo**: parallel branch CRUD with real DB persistence
- **Review tools**: real PostgreSQL writes to `ReviewDecision` table
- **Session tools**: real PostgreSQL reads/writes to `SessionState` table
- **Policy tools**: real validation against DB (workflow status, review decisions, branch status, fingerprints)
- **Semantic search**: feature-hashing vectorizer (768-dim) + cosine similarity; falls back to SQLite in stdio mode
- **Shell tools**: real CLI calls (git, pnpm, docker, gh, test runner)
- **Capability tools**: real `.claude/` directory reads with frontmatter parsing

## workflow (23 tools)

DualWriteStore + OrchestratingEngine with PostgreSQL real-time replication.

- `workflow_create` -- Create workflow
- `workflow_execute` -- Execute workflow
- `workflow_getStatus` -- Get workflow status
- `workflow_get` -- Get workflow by ID
- `workflow_list` -- List workflows
- `workflow_addTask` -- Add task
- `workflow_startTask` -- Start task
- `workflow_completeTask` -- Complete task
- `workflow_saveProgress` -- Save progress
- `workflow_listTasks` -- List tasks
- `workflow_getCurrentTask` -- Current task
- `workflow_getPlan` -- Get plan
- `workflow_getActive` -- Active workflow
- `workflow_createPlan` -- Create plan
- `workflow_createParallelBranches` -- Create parallel branches
- `workflow_completeParallelBranch` -- Complete branch
- `workflow_listParallelBranches` -- List branches
- `workflow_delete` -- Delete workflow
- `workflow_ping` -- Health check (returns backend type + PostgreSQL status)
- `workflow_set_execution_mode` -- Set execution mode (sequential/parallel)
- `workflow_mark_synthesis_ready` -- Mark synthesis ready
- `workflow_mark_verification_ready` -- Mark verification ready
- `workflow_resume_suggestion` -- Get resume suggestion

## memory (11 tools)

PostgreSQL-first with JSON cache fallback (hybrid mode).

- `memory_store` -- Store memory (writes to both PostgreSQL and JSON cache)
- `memory_store_research` -- Store research results
- `memory_recall_recent` -- Recall recent memories (PostgreSQL query, JSON fallback)
- `memory_recall_documents` -- Recall docs for a workflow
- `memory_recall_document_by_type` -- Recall by source type
- `memory_recall_by_task` -- Recall by task ID
- `memory_update` -- Update memory
- `memory_delete` -- Delete memory
- `memory_delete_by_workflow` -- Delete all memories for a workflow
- `memory_search` -- Search memories (case-insensitive text search, PostgreSQL or JSON)
- `memory_stats` -- Memory stats (total count, by type)

## semantic-search (4 tools)

Feature-hashing vectorizer (768-dim) with cosine similarity. In stdio mode, falls back to SQLite feature hashing when API server is not available (v0.3.69+).

- `semantic-search_search_hybrid_context_pack` -- Build hybrid context pack (combines memory, code, and graph search)
- `semantic-search_search_context_fingerprint` -- Compute context fingerprint for change detection
- `semantic-search_code_search` -- Code search (queries code_chunks table, returns ranked results)
- `semantic-search_make_fingerprint` -- Generate deterministic fingerprint for task context

## policy (6 tools)

Real PostgreSQL validation against DB state.

- `policy_check_session_readiness` -- Session readiness (checks SessionState in DB)
- `policy_validate_execution` -- Validate execution (checks workflow/task status in DB)
- `policy_validate_completion` -- Validate completion (checks ReviewDecision in DB)
- `policy_validate_parallel_completion` -- Validate parallel (checks branch status + synthesisReady in DB)
- `policy_detect_scope_drift` -- Detect drift (keyword analysis + optional task lookup)
- `policy_require_context_refresh` -- Context refresh (fingerprint comparison against DB)

## capability (11 tools)

Real `.claude/` directory reads with frontmatter parsing.

- `capability_list_agents` -- List agents (reads `.claude/agents/*.md`, parses frontmatter)
- `capability_list_skills` -- List skills (reads `.claude/skills/*.md`, parses frontmatter)
- `capability_list_templates` -- List templates
- `capability_match_agent` -- Match agent (scores agents against task description)
- `capability_system_readiness` -- System readiness (backend type + PostgreSQL status)
- `capability_workflow_audit` -- Audit (PostgreSQL query for running tasks with no progress)
- `capability_create_agent` -- Create agent (writes frontmatter `.md` file)
- `capability_create_skill` -- Create skill (writes frontmatter `.md` file)
- `capability_scaffold_feature` -- Scaffold feature (creates agent + skill files)
- `capability_scaffold_mcp_server` -- Scaffold MCP server (creates package.json + index.ts)
- `capability_ping` -- Capability health check

## filesystem (5 tools)

Real `fs` operations.

- `filesystem_read` -- Read file (readFileSync)
- `filesystem_write` -- Write file (writeFileSync, creates dirs)
- `filesystem_list` -- List dir (readdirSync with file/dir type)
- `filesystem_delete` -- Delete file (unlinkSync)
- `filesystem_stat` -- File stat (size, isFile)

## review (2 tools)

Real PostgreSQL writes to `ReviewDecision` table.

- `review_submit` -- Submit review (creates ReviewDecision row in DB)
- `review_get_latest` -- Get latest review (queries ReviewDecision in DB)

## session (3 tools)

Real PostgreSQL reads/writes to `SessionState` table.

- `session_get_state` -- Get session state (finds SessionState in DB)
- `session_patch_state` -- Patch session state (upserts SessionState in DB)
- `session_init_context` -- Init session context + check for stale/stuck/failed workflows (returns reminders and reminderStats)

## local (4 tools)

File-based `.masday/` state dir + PostgreSQL sync/push.

- `local_init` -- Init local state dir (creates `.masday/`)
- `local_sync` -- Sync from local `.masday/state/workflows/` (auto-creates dirs, returns null for missing state)
- `local_push` -- Push to DB (uploads JSON cache to PostgreSQL)
- `local_save_artifact` -- Save artifact file locally

## git (3 tools)

Real `execSync` calls to git CLI.

- `git_status` -- `git status --porcelain`
- `git_diff` -- `git diff --stat && git diff`
- `git_commit` -- `git commit -m "<message>"`

## npm (2 tools)

Real `execSync` calls to pnpm CLI.

- `npm_install` -- `pnpm add <packages>` or `pnpm install`
- `npm_run` -- `pnpm run <script>`

## docker (3 tools)

Real `execSync` calls to docker CLI.

- `docker_build` -- `docker build [-t <tag>] .`
- `docker_run` -- `docker run --rm <image>`
- `docker_ps` -- `docker ps --format json`

## cicd (3 tools)

Real `execSync` calls to `gh` CLI.

- `cicd_pipeline_status` -- `gh run list --limit 5 --json ...`
- `cicd_pipeline_trigger` -- `gh workflow run <pipeline>`
- `cicd_runs_view` -- `gh run list --limit 20 --json ...`

## github (3 tools)

Real `execSync` calls to `gh` CLI.

- `github_pr_create` -- `gh pr create --title ... --body ...`
- `github_pr_list` -- `gh pr list --json ...`
- `github_issue_list` -- `gh issue list --json ...`

## tests (1 tool)

Real `execSync` calls to pnpm test runner.

- `tests_run` -- `pnpm test [-- <pattern>]`

## reminder (3 tools)

Stale/stuck workflow detection, reminder listing, and acknowledgment (WorkflowReminder table). **Auto-runs on startup** + periodic background check.

- `reminder_check` -- Detect stale executions, stuck tasks, failed workflows/tasks, idle executions (configurable thresholds)
- `reminder_list` -- List reminders with optional filters (workflowId, acknowledged, limit)
- `reminder_acknowledge` -- Acknowledge or dismiss reminders by ID or workflowId

## projectRules (1 tool)

Real refactor rules validation from `@mcp-rebuild/project-rules`.

- `projectRules_check` -- Validate project against refactor rules and conventions (14 checks: naming, patterns, tools, docs, TypeScript, security, imports). Returns a report of passed/failed checks.

## use_masday (1 tool)

Universal entry point -- parses any user instruction, classifies intent, and returns routing plan with recommended skill/agent/complexity.

- `use_masday` -- Parse user instruction, classify intent (fix/build/test/deploy/research/scaffold/analyze/workflow/git/quick), return routing plan (intent, recommendedSkill, recommendedAgent, complexity)

## Workflow lifecycle behavior

The MCP server enforces a strict state machine (v0.3.69+ validates all transitions):

```
INIT → ANALYZE → PLAN → EXECUTE → VERIFY → DONE
  │                 │      │          │
  └→ DONE/FAILED    │      └→ PAUSED  └→ FIX → EXECUTE
                   └→ FAILED          └→ FAILED
```

- **State validation** (v0.3.69+): `POST /api/workflows/{id}/update` validates transitions via `transition_status()`, rejects invalid states with HTTP 400
- **VERIFY** phase checks for failed tasks before transitioning to DONE
- **FIX** phase resets failed tasks to PENDING and retries execution
- **Auto-transition**: workflow auto-transitions to DONE when all tasks complete
- **Task output piping** -- dependent tasks receive `dependencyOutputs` from completed prerequisites
- **Agent routing** -- tasks dispatched through SkillRouter with 3-tier fallback

## Tool implementation status

| Domain | Tools | Fully Implemented | Notes |
|--------|-------|-------------------|-------|
| workflow | 23 | 22 | `workflow_ping` is mock |
| memory | 11 | 11 | Full DB integration |
| semantic-search | 4 | 4 | Safe fallback to SQLite in stdio mode |
| policy | 6 | 6 | All validations work |
| capability | 11 | 1 | Only `workflow_audit` calls real logic; rest reads `.claude/` files |
| review | 2 | 2 | Full DB integration |
| session | 3 | 3 | Full DB integration |
| reminder | 3 | 3 | Full DB integration |
| graph | 2 | 2 | Full DB integration |
| filesystem | 5 | 5 | CLI wrappers |
| git | 3 | 3 | CLI wrappers |
| npm | 2 | 2 | CLI wrappers |
| docker | 3 | 3 | CLI wrappers |
| cicd | 3 | 3 | CLI wrappers (gh CLI) |
| github | 3 | 3 | CLI wrappers (gh CLI) |
| tests | 1 | 1 | CLI wrapper (pnpm) |
| local | 4 | 4 | Mixed CLI + DB |
| project_rules | 1 | 1 | Validation logic |
| use_masday | 1 | 1 | Intent router |
| **TOTAL** | **96** | **96** | All tools functional |

## Event types

| Event                       | Payload                    |
| --------------------------- | -------------------------- |
| `workflow.started`          | `{ workflow }`             |
| `workflow.completed`        | `{ workflow }`             |
| `workflow.failed`           | `{ workflow, error }`      |
| `workflow.fixing`           | `{ workflow, retryCount }` |
| `workflow.state.transition` | `{ from, to, workflowId }` |
| `task.started`              | `{ task }`                 |
| `task.completed`            | `{ task }`                 |
| `task.failed`               | `{ task, error }`          |

## Related docs

- [CLI commands](./cli-commands.md)
- [State model](./state-model.md)
- [Architecture](../architecture.md)
