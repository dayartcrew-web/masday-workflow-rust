# Migration Guide

This guide covers migrating from the individual codebases (msd-mcp, agentic-llm-mem, standalone masday-workflow-reborn) to the unified platform.

## From msd-mcp

msd-mcp provided workflow orchestration, policy enforcement, capability management, and semantic search. These have been merged into dedicated packages.

### Package Mapping

| msd-mcp Location | Unified Location | Notes |
|---|---|---|
| `packages/shared-types/src/index.ts` | `packages/core/src/types.ts` | All types merged into core |
| `packages/workflow-engine/src/` | `packages/orchestrator/` | Session, review, parallel, drift, fingerprint added |
| `apps/workflow-orchestrator-mcp/` | `apps/agent-runner/` | Unified into single MCP server |
| `apps/policy-mcp/` | `packages/policy/` | PolicyValidator + WorkflowAuditor |
| `apps/capability-mcp/` | `packages/capability/` | Registry, Scaffolder, Health |
| `apps/semantic-search-mcp/` | `packages/intelligence/` | SemanticSearcher, Context, CodeIndexer |
| `packages/db/` | `packages/db/` | Drizzle schema preserved, new models added |

### API Changes

- `createWorkflow()` now returns a `Workflow` object directly (no wrapper)
- Session readiness uses `SessionManager` class instead of raw SQL
- Reviews use `ReviewManager` with `submitReview()` method
- Parallel execution uses `ParallelExecutor` with branch management
- Drift detection uses `detectScopeDrift()` from `@masday-workflow-reborn/orchestrator`

### Breaking Changes

1. **Import paths changed**: All `@msd-mcp/*` imports now use `@masday-workflow-reborn/*`
2. **StorageBackend required**: Session and review managers require a `StorageBackend` parameter
3. **Event structure**: EventBus now wraps data in `Event` objects with `{ type, timestamp, data }`
4. **Tool names**: MCP tools are namespaced (e.g., `policy.validateExecution` not `validate_execution`)

## From agentic-llm-mem

agentic-llm-mem provided 4-layer memory, LLM providers, and intelligence features.

### Package Mapping

| agentic-llm-mem Location | Unified Location | Notes |
|---|---|---|
| `packages/memory/` | `packages/memory/` | All 4 layers preserved |
| `packages/scoring/` | `packages/memory/src/scoring.ts` | ScoringEngine merged into memory |
| `packages/llm/` | `packages/llm/` | All providers preserved |
| `packages/core/react/` | `packages/intelligence/src/react.ts` | ReAct agent moved to intelligence |
| `packages/types/` | `packages/core/src/types.ts` | Types merged into core |

### API Changes

- `MemoryStore` constructor now takes `MemoryStoreConfig` with `filePath` and optional `embeddingService`
- `GraphStore` uses `GraphStoreConfig` with optional `filePath` and `autoLinkThreshold`
- LLM providers implement `ILLMProvider` interface with `complete()` and `chat()` methods
- `FallbackProvider` wraps primary and fallback with circuit breaker

### Breaking Changes

1. **Import paths changed**: All `@agentic-llm/*` imports now use `@masday-workflow-reborn/*`
2. **Memory initialization**: `MemoryStore` requires `await store.init()` before use
3. **Graph initialization**: `GraphStore` requires `await graph.init()` for file persistence
4. **Embedding provider**: `EmbeddingService` requires `apiKey` config (use `MockEmbeddingService` for testing)
5. **Scoring weights**: Default weights are `similarity: 0.6, recency: 0.15, importance: 0.15, usage: 0.1`

## From Standalone masday-workflow-reborn

The standalone version provided the basic workflow engine, skills, and MCP server.

### Package Mapping

| Standalone Location | Unified Location | Notes |
|---|---|---|
| `packages/core/` | `packages/core/` | Extended with new types |
| `packages/orchestrator/` | `packages/orchestrator/` | Extended with session, review, parallel |
| `packages/mcp-server/` | `packages/mcp-server/` | Unchanged |
| `packages/skills/` | `packages/skills/` | Unchanged |
| `packages/code-skills/` | `packages/code-skills/` | Unchanged |
| `packages/store/` | `packages/store/` | Extended with Drizzle adapter |
| `packages/agents/` | `packages/agents/` | Unchanged |
| `packages/intelligence/` | `packages/intelligence/` | Extended with ReAct, Guardrails, search |

### New Packages Added

These packages are entirely new:

- `packages/db/` -- Drizzle schema (packages/db/src/schema.ts) and client
- `packages/memory/` -- 4-layer memory system
- `packages/llm/` -- Multi-provider LLM with resilience
- `packages/policy/` -- Policy enforcement and auditing
- `packages/capability/` -- Capability registry and scaffolding

### Breaking Changes

1. **New dependencies**: Projects using only the standalone engine will need to install new peer dependencies
2. **Event types expanded**: `EventType` union has new members for memory, LLM, policy events
3. **Test count**: Tests now include integration tests in `tests/integration/` and benchmarks in `tests/benchmarks/`

## General Migration Steps

1. **Update imports**: Change all package imports from old names to `@masday-workflow-reborn/*`
2. **Run `pnpm install`**: Install all new workspace dependencies
3. **Run `pnpm build`**: Build all packages (16 packages + 1 app)
4. **Update configuration**: Update `.mcp.json` to point to `apps/agent-runner`
5. **Run `pnpm test`**: Verify all tests pass (1017+ tests)
6. **Update CI**: Update build scripts to use the unified monorepo

## Environment Variables

| Variable | Purpose | Default |
|---|---|---|
| `MASDAY_DB_PATH` | SQLite database path | `./data/masday-workflow.db` |
| `MASDAY_RUNTIME_PROFILE` | Runtime profile (local/docker/remote) | `local` |
| `LLM_MODEL_CHEAP` | Cheap tier model name | `GLM-4.5-Air` |
| `LLM_MODEL_MEDIUM` | Medium tier model name | `glm-4.7` |
| `LLM_MODEL_POWERFUL` | Powerful tier model name | `glm-5` |
| `OPENAI_API_KEY` | OpenAI API key (for LLM only; embeddings now use local fastembed) | -- |
| `ANTHROPIC_API_KEY` | Anthropic API key | -- |

## Getting Help

- Check [CLAUDE.md](../CLAUDE.md) for architecture details and conventions
- Check [README.md](../README.md) for quick start and tool listing
- Run `pnpm test` to verify your setup
- Check integration tests in `tests/integration/` for usage examples
