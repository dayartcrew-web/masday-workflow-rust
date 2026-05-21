# Agents Reference

Complete reference for all 26 specialist agents in the masday-workflow-rebuild platform.

Agents are registered in the capability registry and dispatched by the OrchestratingEngine via the AgentCoordinator.

---

## Workflow Agents

### masday-orchestrator
- **Role:** Full lifecycle coordinator
- **When to use:** Managing end-to-end workflow execution across all 6 phases (INIT, ANALYZE, PLAN, EXECUTE, VERIFY, DONE)
- **MCP tools:** All workflow tools, session tools, plan tools, task tools, review tools, progress tools
- **Description:** Routes tasks to specialist agents, manages state machine transitions, handles FIX retry loops, and ensures workflow completion. The primary agent for coordinating multi-step work.

### masday-planner
- **Role:** Task decomposition and planning
- **When to use:** Breaking down requirements into ordered tasks with acceptance criteria
- **MCP tools:** `workflow_createPlan`, `workflow_listTasks`, `workflow_saveProgress`
- **Description:** Creates structured implementation plans with dependency ordering, acceptance criteria, required context, and verification steps. Produces plans consumable by the executor.

### masday-executor
- **Role:** Code implementation agent
- **When to use:** Implementing active tasks from an approved plan
- **MCP tools:** `workflow_startTask`, `workflow_saveProgress`, `workflow_completeTask`, filesystem tools, code skills
- **Description:** Works strictly on the current active task. Never jumps ahead or works outside scope. Reports progress and submits for review upon completion.

### masday-reviewer
- **Role:** Quality gate reviewer
- **When to use:** Reviewing completed task output before marking as done
- **MCP tools:** `review_submit`, `review_get_latest`, `policy_validate_completion`
- **Description:** Returns one of three decisions: APPROVED (task meets criteria), REWORK_REQUIRED (specific gaps listed), or BLOCKED (external dependency). Enforces quality standards.

### masday-verifier
- **Role:** Final validation and drift detection
- **When to use:** Before marking a workflow as complete, or when scope drift is suspected
- **MCP tools:** `policy_validate_completion`, `policy_detect_scope_drift`, `policy_require_context_refresh`, `policy_check_session_readiness`
- **Description:** Validates task completion readiness, detects scope drift, checks context freshness via fingerprinting, and ensures session state consistency.

### masday-synthesizer
- **Role:** Parallel branch output merger
- **When to use:** Merging outputs from parallel execution branches into a coherent result
- **MCP tools:** `workflow_completeParallelBranch`, `workflow_mark_synthesis_ready`, `workflow_listParallelBranches`, `policy_validate_parallel_completion`
- **Description:** Collects and merges results from parallel branches, resolves conflicts, and produces unified output. Validates all branches are complete before synthesis.

### masday-context-manager
- **Role:** State preservation across agent interactions
- **When to use:** Persisting decisions, artifacts, and learnings between agent handoffs
- **MCP tools:** `memory_store`, `memory_recall_recent`, `memory_recall_documents`, `memory_recall_document_by_type`, `memory_update`
- **Description:** Manages the memory stack (working, episodic, long-term, graph) to ensure context is preserved when agents hand off tasks. Stores decisions, artifacts, and learnings.

---

## Development Agents

### masday-backend
- **Role:** Backend development specialist
- **When to use:** Building API endpoints, database operations, Docker configuration, server-side logic
- **MCP tools:** Filesystem tools, code skills (`code.*`), `npm.*` tools
- **Description:** Handles all server-side implementation including REST API endpoints, database queries, middleware, authentication logic, and Dockerfile/container configuration.

### masday-frontend
- **Role:** Frontend development specialist
- **When to use:** Building UI components, state management, responsive design, client-side logic
- **MCP tools:** Filesystem tools, code skills (`code.*`), `npm.*` tools
- **Description:** Implements user interface components with proper state management, responsive layouts, accessibility compliance, and integration with backend APIs.

### masday-integrator
- **Role:** Cross-module integration specialist
- **When to use:** Wiring features together across packages, resolving dependency conflicts, integration testing
- **MCP tools:** Filesystem tools, `workflow_saveProgress`, code skills
- **Description:** Connects independently developed modules into a cohesive system. Handles package dependency resolution, API contract alignment, and cross-module testing.

### masday-refactor-cleaner
- **Role:** Code cleanup and consolidation
- **When to use:** Removing dead code, consolidating duplicates, simplifying complex modules
- **MCP tools:** Filesystem tools, code skills (`code.*`)
- **Description:** Identifies and removes unused code, consolidates duplicate logic, simplifies overly complex functions, and enforces file size limits (under 400 lines).

### masday-linter
- **Role:** Code style and type enforcement
- **When to use:** Before commits, during code review, when fixing type errors
- **MCP tools:** Filesystem tools, `npm.*` tools (lint scripts)
- **Description:** Enforces TypeScript strict mode, coding conventions, naming standards, and linting rules. Catches style violations and type errors before they reach review.

### masday-doc-updater
- **Role:** Documentation generation and maintenance
- **When to use:** Updating API docs, README files, architecture diagrams, inline documentation
- **MCP tools:** Filesystem tools, `memory_recall_documents`
- **Description:** Generates and maintains documentation including API references, architecture diagrams, setup guides, and inline code documentation. Keeps docs in sync with code changes.

---

## Testing Agents

### masday-qa
- **Role:** Testing strategy and coverage
- **When to use:** Designing test plans, ensuring coverage meets 80% threshold, writing unit and integration tests
- **MCP tools:** Filesystem tools, `tests.*` code skills
- **Description:** Creates testing strategies, writes unit and integration tests, validates coverage thresholds, and ensures test isolation and reliability.

### masday-e2e-tester
- **Role:** End-to-end testing with Playwright
- **When to use:** Testing critical user flows, cross-browser validation, visual regression
- **MCP tools:** Filesystem tools, `tests.*` code skills, `docker.*` tools
- **Description:** Writes and executes end-to-end tests using Playwright. Covers critical user journeys, cross-browser compatibility, and visual regression testing.

### masday-debugger
- **Role:** Root cause investigation
- **When to use:** Investigating bugs, tracing errors through call stacks, diagnosing test failures
- **MCP tools:** Filesystem tools, `memory_search`, `memory_recall_by_task`
- **Description:** Systematically traces bugs backward through call stacks, adds instrumentation when needed, and identifies the source of invalid data or incorrect behavior before proposing fixes.

---

## Security and Performance

### masday-security
- **Role:** Vulnerability analysis and compliance
- **When to use:** Before commits with auth/payment changes, periodic security audits, OWASP compliance checks
- **MCP tools:** Filesystem tools, `memory_store` (security findings)
- **Description:** Performs security analysis including OWASP Top 10 checks, secret detection, input validation review, authentication/authorization verification, and dependency vulnerability scanning.

### masday-performance
- **Role:** Bottleneck identification and optimization
- **When to use:** When response times degrade, memory usage spikes, or before production deployment
- **MCP tools:** Filesystem tools, `memory_store` (performance baselines)
- **Description:** Profiles application performance, identifies bottlenecks in database queries, API endpoints, and resource usage. Recommends and implements optimizations including caching, indexing, and query tuning.

---

## Research and Intelligence

### masday-researcher
- **Role:** External information gathering
- **When to use:** When domain knowledge is needed beyond the codebase, technology evaluation, best practice discovery
- **MCP tools:** `memory_store_research`, `memory_recall_documents`, `memory_recall_document_by_type`
- **Description:** Gathers external information via web search, evaluates technologies and libraries, and produces research summaries stored as context documents for other agents.

### masday-codebase-mapper
- **Role:** Architecture documentation and pattern analysis
- **When to use:** Onboarding new agents, understanding system structure, identifying architectural patterns
- **MCP tools:** `semantic-search_search_hybrid_context_pack`, `semantic-search_search_context_fingerprint`, `memory_store`
- **Description:** Maps codebase architecture, documents patterns and conventions, and builds context packs that enable other agents to understand system structure without reading every file.

### masday-intel-updater
- **Role:** Intelligence file management
- **When to use:** Updating context documents, refreshing fingerprints, maintaining knowledge base
- **MCP tools:** `semantic-search_search_hybrid_context_pack`, `semantic-search_search_context_fingerprint`, `memory_update`, `memory_store`
- **Description:** Manages intelligence files including context packs, codebase fingerprints, and research documents. Ensures knowledge base stays current with codebase changes.

### masday-ideation
- **Role:** Feature brainstorming and improvement suggestions
- **When to use:** Exploring new features, suggesting improvements, evaluating design alternatives
- **MCP tools:** `memory_store`, `memory_recall_recent`
- **Description:** Generates creative feature ideas, suggests improvements to existing functionality, and evaluates design alternatives through structured brainstorming.

---

## Infrastructure

### masday-git-master
- **Role:** Version control operations
- **When to use:** Branch management, commit creation, PR workflows, merge conflict resolution
- **MCP tools:** `git.*` code skills, `github.*` code skills
- **Description:** Handles all git operations including branching strategies, conventional commit formatting, pull request creation, and merge conflict resolution. Enforces git workflow standards.

### masday-ci-cd-pipeline
- **Role:** Deployment automation
- **When to use:** Setting up CI/CD pipelines, configuring deployment stages, managing release processes
- **MCP tools:** `cicd.*` code skills, `docker.*` tools
- **Description:** Configures and maintains CI/CD pipelines including build stages, test execution, deployment automation, and environment management. Manages Docker-based deployment workflows.

### masday-config
- **Role:** Configuration management
- **When to use:** Environment variable setup, MCP server configuration, package configuration changes
- **MCP tools:** Filesystem tools, `memory_store` (config decisions)
- **Description:** Manages application configuration across environments including environment variables, MCP server settings, TypeScript configuration, and package.json dependencies.

### masday-database-arch
- **Role:** Schema design and query optimization
- **When to use:** Designing database schemas, optimizing queries, migration planning, pgvector configuration
- **MCP tools:** Filesystem tools, `memory_store` (schema decisions)
- **Description:** Designs database schemas with proper indexing, writes optimized queries, plans migrations, and configures pgvector for semantic search. Manages Drizzle schema evolution.

---

## Agent Dispatch Flow

> **All 16 Drizzle tables are actively populated.** See CLAUDE.md for the full table wiring reference.
> **Status values are UPPERCASE** in PostgreSQL: Workflow (INIT, EXECUTE, DONE, FAILED...), Task (PENDING, RUNNING, DONE, FAILED), Plan (ACTIVE, PENDING, READY, DONE), Review (APPROVED, REWORK_REQUIRED, BLOCKED).

1. OrchestratingEngine receives a task
2. AgentCoordinator queries the capability registry
3. SkillRouter scores agents by relevance to task description
4. Highest-scoring agent is dispatched with task context
5. Agent executes within session enforcement and policy validation
6. Output is reviewed and stored in memory

## Agent Registration

Agents are registered via the capability MCP tools:

```
capability_create_agent({ name, role, description, instructions })
```

Or via CLI templates in `packages/cli/templates/agents/`.

## Pre-Commit Discipline (No Hooks Available)

Non-Claude platforms do not support Claude-style hooks. You MUST enforce these checks manually before every commit:

1. **Typecheck**: Run `pnpm typecheck` — zero errors
2. **Tests**: Run `pnpm test` — all tests pass
3. **No secrets**: No hardcoded API keys, passwords, or tokens
4. **No console.log**: No debug statements in production code
5. **Review pipeline**: After completing a task in a workflow:
   - `review_submit` → `policy_validate_completion` → `workflow_completeTask`
6. **Tool names**: All MCP tool names use underscore format (`workflow_create`, not `workflow.create`)

## Universal Guard Script

Since Claude Code hooks are NOT available on this platform, use the guard script to replicate their behavior:

```bash
# Before editing a source file — checks TDD, secrets, console.log
node scripts/guard.mjs pre-edit <file-path>

# After editing a source file — reminds to saveProgress/review
node scripts/guard.mjs post-edit <file-path>

# Before completing a workflow task — checks review gate
node scripts/guard.mjs pre-complete
```

**You MUST run `pre-edit` before writing code and `post-edit` after writing code.** This replaces the Claude hooks that enforce TDD, secret detection, and the review pipeline.

## Git Hooks (Cross-Platform)

Git hooks are installed via `scripts/install-hooks.sh` and run automatically:
- `pre-commit`: guard checks + typecheck + tests
- `pre-push`: build + full test suite

Install hooks: `bash scripts/install-hooks.sh`
