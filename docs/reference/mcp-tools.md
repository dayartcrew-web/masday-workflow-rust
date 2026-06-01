# MCP Tools Reference

This page is the canonical contributor-facing reference for the MCP tool surface. The server is `masday-mcp` (Rust, JSON-RPC 2.0 over stdio) with `masday-api` (Axum HTTP API on port 3010). All tools are connected to PostgreSQL via `tokio-postgres` with repos in `masday-db/src/repos/`.

## Persistence

- **WorkflowRepo**: all workflow/task/plan operations via `tokio-postgres` with PascalCase tables
- **Memory**: `MemoryRepo` with importance scoring and text search
- **BranchRepo**: parallel branch CRUD with real DB persistence
- **Review tools**: real PostgreSQL writes to `ReviewDecision` table
- **Session tools**: real PostgreSQL reads/writes to `SessionState` table
- **Policy tools**: real validation against DB (workflow status, review decisions, branch status, fingerprints)
- **Shell tools**: real `execSync` calls (git, pnpm, docker, gh CLI, test runner)
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

Drizzle-first with JSON cache fallback (hybrid mode).

- `memory_store` -- Store memory (writes to both Drizzle and JSON cache)
- `memory_store_research` -- Store research results
- `memory_recall_recent` -- Recall recent memories (Drizzle query, JSON fallback)
- `memory_recall_documents` -- Recall docs for a workflow
- `memory_recall_document_by_type` -- Recall by source type
- `memory_recall_by_task` -- Recall by task ID
- `memory_update` -- Update memory
- `memory_delete` -- Delete memory
- `memory_delete_by_workflow` -- Delete all memories for a workflow
- `memory_search` -- Search memories (case-insensitive text search, Drizzle or JSON)
- `memory_stats` -- Memory stats (total count, by type)

## semantic-search (3 tools)

- `semantic-search_search_hybrid_context_pack` -- Context pack
- `semantic-search_search_context_fingerprint` -- Fingerprint
- `semantic-search_code_search` -- Code search

## policy (6 tools)

Real Drizzle validation against DB state.

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
- `capability_workflow_audit` -- Audit (Drizzle query for running tasks with no progress)
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

Real Drizzle writes to `ReviewDecision` table.

- `review_submit` -- Submit review (creates ReviewDecision row in DB)
- `review_get_latest` -- Get latest review (queries ReviewDecision in DB)

## session (3 tools)

Real Drizzle reads/writes to `SessionState` table.

- `session_get_state` -- Get session state (finds SessionState in DB)
- `session_patch_state` -- Patch session state (upserts SessionState in DB)
- `session_init_context` -- Init session context + check for stale/stuck/failed workflows (returns reminders and reminderStats)

## local (4 tools)

File-based `.masday/` state dir + Drizzle sync/push.

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

Stale/stuck workflow detection, reminder listing, and acknowledgment (Drizzle WorkflowReminder table). **Auto-runs on startup** after Drizzle connects + **periodic background check every 15 minutes** via setInterval.

- `reminder_check` -- Detect stale executions, stuck tasks, failed workflows/tasks, idle executions (configurable thresholds). Runs automatically on server start and every 15 minutes.
- `reminder_list` -- List reminders with optional filters (workflowId, acknowledged, limit)
- `reminder_acknowledge` -- Acknowledge or dismiss reminders by ID or workflowId

## projectRules (1 tool)

Real refactor rules validation from `@mcp-rebuild/project-rules`.

- `projectRules_check` -- Validate project against refactor rules and conventions (14 checks: naming, patterns, tools, docs, TypeScript, security, imports). Returns a report of passed/failed checks.

## use_masday (1 tool)

Universal entry point -- parses any user instruction, classifies intent, and returns routing plan with recommended skill/agent/complexity.

- `use_masday` -- Parse user instruction, classify intent (fix/build/test/deploy/research/scaffold/analyze/workflow/git/quick), return routing plan (intent, recommendedSkill, recommendedAgent, complexity)

## Workflow lifecycle behavior

The MCP server runs on `OrchestratingEngine` with full agent dispatch enabled:

- **VERIFY** phase checks for failed tasks before transitioning to DONE
- **FIX** phase resets failed tasks to PENDING and retries execution (configurable `maxFixRetries`)
- **Task output piping** -- dependent tasks receive `dependencyOutputs` from completed prerequisites
- **Agent routing** -- tasks dispatched through SkillRouter with 3-tier fallback to appropriate agent worker

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
