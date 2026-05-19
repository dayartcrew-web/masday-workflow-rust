# MCP Tools Reference

This page is the canonical contributor-facing reference for the MCP tool surface. The server is `apps/agent-runner/src/runtime/mcp.ts` -- 83 real tools across 14 namespaces, all connected to PostgreSQL via DualWriteStore.

## Persistence

- **DualWriteWorkflowStore**: all workflow operations replicate to PostgreSQL in real-time via Prisma
- **Memory**: hybrid mode -- Prisma first, JSON cache fallback when PostgreSQL is unavailable
- **Review tools**: real Prisma writes to `ReviewDecision` table
- **Session tools**: real Prisma reads/writes to `SessionState` table
- **Policy tools**: real validation against DB (workflow status, review decisions, branch status, fingerprints)
- **Shell tools**: real `execSync` calls (git, pnpm, docker, gh CLI, test runner)
- **Capability tools**: real `.claude/` directory reads with frontmatter parsing

## workflow (23 tools)

DualWriteStore + OrchestratingEngine with PostgreSQL real-time replication.

- `workflow.create` -- Create workflow
- `workflow.execute` -- Execute workflow
- `workflow.getStatus` -- Get workflow status
- `workflow.get` -- Get workflow by ID
- `workflow.list` -- List workflows
- `workflow.addTask` -- Add task
- `workflow.startTask` -- Start task
- `workflow.completeTask` -- Complete task
- `workflow.saveProgress` -- Save progress
- `workflow.listTasks` -- List tasks
- `workflow.getCurrentTask` -- Current task
- `workflow.getPlan` -- Get plan
- `workflow.getActive` -- Active workflow
- `workflow.createPlan` -- Create plan
- `workflow.createParallelBranches` -- Create parallel branches
- `workflow.completeParallelBranch` -- Complete branch
- `workflow.listParallelBranches` -- List branches
- `workflow.delete` -- Delete workflow
- `workflow.ping` -- Health check (returns backend type + PostgreSQL status)
- `workflow.set_execution_mode` -- Set execution mode (sequential/parallel)
- `workflow.mark_synthesis_ready` -- Mark synthesis ready
- `workflow.mark_verification_ready` -- Mark verification ready
- `workflow.resume_suggestion` -- Get resume suggestion

## memory (11 tools)

Prisma-first with JSON cache fallback (hybrid mode).

- `memory.store` -- Store memory (writes to both Prisma and JSON cache)
- `memory.store_research` -- Store research results
- `memory.recall_recent` -- Recall recent memories (Prisma query, JSON fallback)
- `memory.recall_documents` -- Recall docs for a workflow
- `memory.recall_document_by_type` -- Recall by source type
- `memory.recall_by_task` -- Recall by task ID
- `memory.update` -- Update memory
- `memory.delete` -- Delete memory
- `memory.delete_by_workflow` -- Delete all memories for a workflow
- `memory.search` -- Search memories (case-insensitive text search, Prisma or JSON)
- `memory.stats` -- Memory stats (total count, by type)

## semantic-search (3 tools)

- `semantic-search.search_hybrid_context_pack` -- Context pack
- `semantic-search.search_context_fingerprint` -- Fingerprint
- `semantic-search.code_search` -- Code search

## policy (6 tools)

Real Prisma validation against DB state.

- `policy.check_session_readiness` -- Session readiness (checks SessionState in DB)
- `policy.validate_execution` -- Validate execution (checks workflow/task status in DB)
- `policy.validate_completion` -- Validate completion (checks ReviewDecision in DB)
- `policy.validate_parallel_completion` -- Validate parallel (checks branch status + synthesisReady in DB)
- `policy.detect_scope_drift` -- Detect drift (keyword analysis + optional task lookup)
- `policy.require_context_refresh` -- Context refresh (fingerprint comparison against DB)

## capability (11 tools)

Real `.claude/` directory reads with frontmatter parsing.

- `capability.list_agents` -- List agents (reads `.claude/agents/*.md`, parses frontmatter)
- `capability.list_skills` -- List skills (reads `.claude/skills/*.md`, parses frontmatter)
- `capability.list_templates` -- List templates
- `capability.match_agent` -- Match agent (scores agents against task description)
- `capability.system_readiness` -- System readiness (backend type + PostgreSQL status)
- `capability.workflow_audit` -- Audit (Prisma query for running tasks with no progress)
- `capability.create_agent` -- Create agent (writes frontmatter `.md` file)
- `capability.create_skill` -- Create skill (writes frontmatter `.md` file)
- `capability.scaffold_feature` -- Scaffold feature (creates agent + skill files)
- `capability.scaffold_mcp_server` -- Scaffold MCP server (creates package.json + index.ts)
- `capability.ping` -- Capability health check

## filesystem (5 tools)

Real `fs` operations.

- `filesystem.read` -- Read file (readFileSync)
- `filesystem.write` -- Write file (writeFileSync, creates dirs)
- `filesystem.list` -- List dir (readdirSync with file/dir type)
- `filesystem.delete` -- Delete file (unlinkSync)
- `filesystem.stat` -- File stat (size, isFile)

## review (2 tools)

Real Prisma writes to `ReviewDecision` table.

- `review.submit` -- Submit review (creates ReviewDecision row in DB)
- `review.get_latest` -- Get latest review (queries ReviewDecision in DB)

## session (3 tools)

Real Prisma reads/writes to `SessionState` table.

- `session.get_state` -- Get session state (finds SessionState in DB)
- `session.patch_state` -- Patch session state (upserts SessionState in DB)
- `session.init_context` -- Init session context

## local (4 tools)

File-based `.masday/` state dir + Prisma sync/push.

- `local.init` -- Init local state dir (creates `.masday/`)
- `local.sync` -- Sync from DB (downloads PostgreSQL to JSON cache)
- `local.push` -- Push to DB (uploads JSON cache to PostgreSQL)
- `local.save_artifact` -- Save artifact file locally

## git (3 tools)

Real `execSync` calls to git CLI.

- `git.status` -- `git status --porcelain`
- `git.diff` -- `git diff --stat && git diff`
- `git.commit` -- `git commit -m "<message>"`

## npm (2 tools)

Real `execSync` calls to pnpm CLI.

- `npm.install` -- `pnpm add <packages>` or `pnpm install`
- `npm.run` -- `pnpm run <script>`

## docker (3 tools)

Real `execSync` calls to docker CLI.

- `docker.build` -- `docker build [-t <tag>] .`
- `docker.run` -- `docker run --rm <image>`
- `docker.ps` -- `docker ps --format json`

## cicd (3 tools)

Real `execSync` calls to `gh` CLI.

- `cicd.pipeline_status` -- `gh run list --limit 5 --json ...`
- `cicd.pipeline_trigger` -- `gh workflow run <pipeline>`
- `cicd.runs_view` -- `gh run list --limit 20 --json ...`

## github (3 tools)

Real `execSync` calls to `gh` CLI.

- `github.pr_create` -- `gh pr create --title ... --body ...`
- `github.pr_list` -- `gh pr list --json ...`
- `github.issue_list` -- `gh issue list --json ...`

## tests (1 tool)

Real `execSync` calls to pnpm test runner.

- `tests.run` -- `pnpm test [-- <pattern>]`

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
