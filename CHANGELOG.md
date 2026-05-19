# Changelog

All notable changes to masday-workflow-rebuild.

## [0.3.0] - 2026-05-19

### Added
- **Reminder hooks**: Workflow lifecycle reminder engine detecting stale, stuck, and failed workflows/tasks
  - `reminder.check` — Detect STALE_EXECUTION, STUCK_TASK, FAILED_WORKFLOW, FAILED_TASK, IDLE_EXECUTION
  - `reminder.list` — List reminders with filtering (workflowId, acknowledged, limit)
  - `reminder.acknowledge` — Acknowledge or dismiss reminders
- **WorkflowReminder Prisma model**: 15th database table for persisting reminder state (type, severity, message, acknowledged)
- **ReminderEngine module** (`packages/workflow-engine/src/reminders.ts`): Time-based and state-change detection with configurable thresholds

### Changed
- **87 MCP tools** (was 86) across 17 namespaces (was 16) with `projectRules` namespace added
- **15 Prisma tables actively populated** (was 14) — added WorkflowReminder
- Fixed AnthropicProvider `tokensUsed` calculation to correctly sum input + output tokens

### Documentation
- Updated all docs: CLAUDE.md, GEMINI.md, AGENTS.md, README.md, docs/* to reflect 87 tools, 15 tables, projectRules namespace

## [0.2.0] - 2026-05-19

### Changed
- **Unified MCP server**: Consolidated 6 separate MCP apps into single `apps/agent-runner` (83 tools)
- **DualWriteStore pattern**: All workflow operations replicate to PostgreSQL in real-time via Prisma
- **14 Prisma tables actively populated**: Workflow, Task, Plan, Memory, ReviewDecision, SessionState, ParallelBranch, ContextDocument, TaskProgressLog, RetrievalLog, TokenUsage, EpisodicMemory, GraphNode, GraphEdge
- **Status normalization**: All status values UPPERCASE in PostgreSQL (Workflow: INIT/EXECUTE/DONE..., Task: PENDING/RUNNING/DONE/FAILED, Plan: ACTIVE/PENDING/READY/DONE, Review: APPROVED/REWORK_REQUIRED/BLOCKED)
- **Module system**: ESM (`"type": "module"`, NodeNext resolution) across all packages
- **Tool naming**: camelCase dot-namespaced format (`workflow.getActive`, `memory.store`)
- **Package scope**: All packages unified under `@mcp-rebuild/*`

### Added
- EpisodicMemory persistence to PostgreSQL via `setEpisodicPrisma()`
- GraphNode/GraphEdge persistence to PostgreSQL via `setGraphPrisma()`
- ContextDocument creation on `memory.store_research`
- TaskProgressLog population via `saveProgressDb()` on `workflow.saveProgress`
- RetrievalLog population via `logRetrieval()` on `memory.search`, `semantic-search.code_search`, `search_hybrid_context_pack`
- TokenUsage tracking via `trackTokens()` on `workflow.saveProgress`, `memory.store_research`
- `@mcp-rebuild/memory` dependency added to `apps/agent-runner`
- 26 specialist agents registered in `.claude/agents/`
- 25+ skills registered in `.claude/skills/`

### Fixed
- DualWriteStore status mapping: Task states normalized from lowercase to UPPERCASE before Prisma writes
- workflow-engine status values: `task.ts`, `plan.ts`, `review.ts`, `workflow-create.ts` all emit UPPERCASE
- All `.claude/` skill and agent .md files updated with UPPERCASE status conventions

### Documentation
- README.md: Updated to 83 tools, single MCP server, DualWriteStore, 14 Prisma tables
- docs/architecture.md: DualWriteStore + PostgreSQL, ESM modules, updated monorepo structure
- docs/getting-started.md: PostgreSQL setup steps, DualWriteStore description
- docs/reference/state-model.md: 14 Prisma tables, UPPERCASE status conventions
- docs/reference/mcp-tools.md: Status normalization
- docs/workflows/lifecycle.md: Status normalization
- CLAUDE.md, GEMINI.md, AGENTS.md: 14-table wiring reference

## [0.1.0] - 2026-05-18

### Added
- Initial unified codebase merging msd-mcp and masday-workflow-reborn
- 12 packages under @mcp-rebuild/* scope
- 4-layer memory system (working, episodic, long-term, graph)
- 3-tier workflow engine (basic, enhanced, orchestrating)
- Multi-platform support: Claude Code, Codex CLI, Gemini CLI, Continue, GitHub Copilot
- Prisma + PostgreSQL + pgvector database layer
- Official MCP SDK pattern with McpServer
- Docker Compose for PostgreSQL + pgvector
- Vitest test suite with registry validation
- Setup scripts for bash and PowerShell
