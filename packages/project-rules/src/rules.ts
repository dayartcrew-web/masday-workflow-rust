import type { RuleSet, RefactorChecklist } from "./types.js";

export const PROJECT_RULES: RuleSet = {
  version: "1.0.0",
  updated: "2026-05-19",
  rules: [
    // ── NAMING ──
    {
      id: "NAMING-001",
      title: "Package scope must be @mcp-rebuild/*",
      description:
        "All packages use @mcp-rebuild/* scope. Never use @masday-workflow-reborn/* or @cap/*.",
      category: "NAMING",
      severity: "CRITICAL",
      check: "has-match",
      targets: ["packages/*/package.json"],
      pattern: '"name": "@mcp-rebuild/',
      fixHint:
        "Update package.json name field to use @mcp-rebuild/ prefix.",
    },
    {
      id: "NAMING-002",
      title: "MCP tools use camelCase dot-namespaced format",
      description:
        "Tool names: workflow.create, memory.store, review.submit. NEVER snake_case.",
      category: "NAMING",
      severity: "CRITICAL",
      negPattern: "server\\.tool\\([\"'][a-z_]+_[a-z_]+",
      fixHint:
        "Use camelCase dot-namespaced: workflow.createPlan not workflow_create_plan.",
    },
    {
      id: "NAMING-003",
      title: "ESM imports use .js extensions",
      description:
        "All relative imports must end with .js extension (TypeScript ESM convention).",
      category: "NAMING",
      severity: "HIGH",
      negPattern: 'from\\s+["\']\\..*(?<!\\.js)["\']',
      fixHint:
        'Add .js extension: import { x } from "./module.js" not "./module".',
    },
    {
      id: "NAMING-004",
      title: "Status values are UPPERCASE in PostgreSQL",
      description:
        "Workflow: INIT/DONE/FAILED. Task: PENDING/RUNNING/DONE/FAILED. Review: APPROVED/REWORK_REQUIRED/BLOCKED.",
      category: "NAMING",
      severity: "HIGH",
      fixHint:
        "Map in-memory lowercase to UPPERCASE before Prisma write.",
    },
    {
      id: "NAMING-005",
      title: "listWorkflows exported as listWorkflowsDb",
      description:
        "The listWorkflows function from workflow-engine must be imported as listWorkflowsDb.",
      category: "NAMING",
      severity: "MEDIUM",
      fixHint:
        'import { listWorkflows as listWorkflowsDb } from "@mcp-rebuild/workflow-engine".',
    },

    // ── PATTERN ──
    {
      id: "PATTERN-001",
      title: "Use DualWriteWorkflowStore for persistence",
      description:
        "Wrap WorkflowStore with DualWriteWorkflowStore for real-time PostgreSQL replication.",
      category: "PATTERN",
      severity: "CRITICAL",
      fixHint:
        "const store = new DualWriteWorkflowStore(new WorkflowStore(backend));",
    },
    {
      id: "PATTERN-002",
      title: "Immutable patterns - spread, never mutate",
      description:
        "Always create new objects. Never mutate in-place. Use spread operators.",
      category: "PATTERN",
      severity: "HIGH",
      fixHint:
        "return { ...original, field: newValue } instead of original.field = newValue.",
    },
    {
      id: "PATTERN-003",
      title: "Tool handler format returns content array",
      description:
        'All MCP tool handlers return { content: [{ type: "text", text: JSON.stringify(result) }] }.',
      category: "PATTERN",
      severity: "CRITICAL",
      fixHint:
        'return { content: [{ type: "text", text: JSON.stringify(result) }] }',
    },
    {
      id: "PATTERN-004",
      title: "Functions under 50 lines, files under 400 lines",
      description:
        "Keep functions focused (<50 lines) and files cohesive (<400 lines).",
      category: "PATTERN",
      severity: "MEDIUM",
      fixHint: "Extract utilities from large modules.",
    },
    {
      id: "PATTERN-005",
      title: "EventBus for pub/sub",
      description:
        "Use EventBus from @mcp-rebuild/core for pub/sub patterns.",
      category: "PATTERN",
      severity: "MEDIUM",
    },
    {
      id: "PATTERN-006",
      title: "Zod for all validation",
      description:
        "Use Zod schemas for input validation. Infer types from schemas.",
      category: "PATTERN",
      severity: "HIGH",
      fixHint: "const schema = z.object({...}); type X = z.infer<typeof schema>;",
    },

    // ── TOOLS ──
    {
      id: "TOOLS-001",
      title: "MCP server uses official McpServer SDK",
      description:
        'Import from @modelcontextprotocol/sdk/server/mcp.js. Use McpServer class.',
      category: "TOOLS",
      severity: "CRITICAL",
      fixHint:
        'import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";',
    },
    {
      id: "TOOLS-002",
      title: "All 83 tools in single server",
      description:
        "All tools are registered in apps/agent-runner/src/runtime/mcp.ts.",
      category: "TOOLS",
      severity: "HIGH",
      fixHint: "Do not split into multiple MCP server files.",
    },
    {
      id: "TOOLS-003",
      title: "setDualWritePrisma and setTokenPrisma called on startup",
      description:
        "After Prisma connects, call setDualWritePrisma(prisma) and setTokenPrisma(prisma).",
      category: "TOOLS",
      severity: "HIGH",
    },
    {
      id: "TOOLS-004",
      title: "Code skills are plain async functions",
      description:
        "Code skills in packages/code-skills are plain async functions, not class-based Skill objects.",
      category: "TOOLS",
      severity: "MEDIUM",
    },

    // ── DOCS ──
    {
      id: "DOCS-001",
      title: "CLAUDE.md is source of truth",
      description:
        "CLAUDE.md at project root is the authoritative reference. All other docs must match.",
      category: "DOCS",
      severity: "HIGH",
    },
    {
      id: "DOCS-002",
      title: "Tool names in docs use camelCase dot-namespaced",
      description:
        "Documentation must reference tools as workflow.createPlan, not workflow_create_plan.",
      category: "DOCS",
      severity: "HIGH",
      negPattern: "workflow_[a-z_]+\\(",
      fixHint:
        "Replace snake_case tool refs: workflow_create_plan → workflow.createPlan",
    },
    {
      id: "DOCS-003",
      title: "Setup scripts clean before copy",
      description:
        "setup.sh and setup.ps1 must rm -rf destination before copying to prevent stale files.",
      category: "DOCS",
      severity: "MEDIUM",
    },
    {
      id: "DOCS-004",
      title: "Tool count must match actual registration",
      description:
        "Docs claiming tool count (e.g. '83 tools') must match actual registered tools in mcp.ts.",
      category: "DOCS",
      severity: "MEDIUM",
    },

    // ── TYPESCRIPT ──
    {
      id: "TS-001",
      title: "No 'any' types - use 'unknown' with Zod",
      description:
        "Never use any. Use unknown and narrow with Zod validation or type guards.",
      category: "TYPESCRIPT",
      severity: "HIGH",
      negPattern: ":\\s*any\\b",
      fixHint: "Replace 'any' with 'unknown' and narrow with Zod.",
    },
    {
      id: "TS-002",
      title: "TypeScript strict mode enabled",
      description: "All packages use strict: true in tsconfig.",
      category: "TYPESCRIPT",
      severity: "HIGH",
    },
    {
      id: "TS-003",
      title: "ESM modules with NodeNext resolution",
      description:
        '"type": "module" in package.json, moduleResolution: "NodeNext".',
      category: "TYPESCRIPT",
      severity: "HIGH",
    },
    {
      id: "TS-004",
      title: "No console.log in production code",
      description: "Use Pino logger from @mcp-rebuild/core instead.",
      category: "TYPESCRIPT",
      severity: "MEDIUM",
      negPattern: "console\\.log\\(",
      fixHint: "Replace console.log with logger.info() from @mcp-rebuild/core.",
    },

    // ── TESTING ──
    {
      id: "TEST-001",
      title: "Vitest with globals enabled",
      description: "All tests use Vitest framework with globals config.",
      category: "TESTING",
      severity: "MEDIUM",
    },
    {
      id: "TEST-002",
      title: "Integration tests in tests/integration/",
      description:
        "Integration tests live in tests/integration/ at project root.",
      category: "TESTING",
      severity: "MEDIUM",
    },

    // ── SECURITY ──
    {
      id: "SEC-001",
      title: "No hardcoded secrets",
      description:
        "Never hardcode API keys, passwords, tokens. Use environment variables.",
      category: "SECURITY",
      severity: "CRITICAL",
      negPattern: "(sk-proj-|api_key\\s*=|password\\s*=).*[\"']",
      fixHint: "Move secrets to .env and access via process.env.",
    },
    {
      id: "SEC-002",
      title: "Environment variables via env() utility",
      description:
        "Use env() from @mcp-rebuild/shared-utils for env access with validation.",
      category: "SECURITY",
      severity: "HIGH",
    },

    // ── ARCHITECTURE ──
    {
      id: "ARCH-001",
      title: "Monorepo: pnpm workspaces",
      description:
        "Project uses pnpm workspaces with Turbo build. All packages under packages/.",
      category: "ARCHITECTURE",
      severity: "HIGH",
    },
    {
      id: "ARCH-002",
      title: "14 Prisma models actively populated",
      description:
        "Workflow, Task, Plan, Memory, ReviewDecision, SessionState, ParallelBranch, ContextDocument, TaskProgressLog, RetrievalLog, TokenUsage, EpisodicMemory, GraphNode, GraphEdge.",
      category: "ARCHITECTURE",
      severity: "MEDIUM",
    },
    {
      id: "ARCH-003",
      title: "4-layer memory stack",
      description:
        "Working → Episodic → Long-term → Knowledge Graph. Scoring: similarity*0.6 + recency*0.15 + importance*0.15 + usage*0.1.",
      category: "ARCHITECTURE",
      severity: "MEDIUM",
    },

    // ── MCP ──
    {
      id: "MCP-001",
      title: "StdioServerTransport for MCP communication",
      description:
        "MCP server uses StdioServerTransport from the official SDK.",
      category: "MCP",
      severity: "HIGH",
    },
    {
      id: "MCP-002",
      title: "14 namespaces in single server",
      description:
        "workflow(23), memory(11), semantic-search(3), policy(6), capability(11), filesystem(5), review(2), session(3), local(4), git(3), npm(2), docker(3), cicd(3), github(3), tests(1).",
      category: "MCP",
      severity: "MEDIUM",
    },

    // ── DATABASE ──
    {
      id: "DB-001",
      title: "Prisma schema at packages/db/prisma/schema.prisma",
      description:
        "Single Prisma schema with 14 models + pgvector extension.",
      category: "DATABASE",
      severity: "HIGH",
    },
    {
      id: "DB-002",
      title: "Memory hybrid mode: Prisma first, JSON fallback",
      description:
        "When PostgreSQL is unavailable, memory falls back to JSON cache.",
      category: "DATABASE",
      severity: "MEDIUM",
    },

    // ── IMPORTS ──
    {
      id: "IMPORT-001",
      title: "Import from workspace packages via @mcp-rebuild/*",
      description:
        "All internal package imports use @mcp-rebuild/* scope with workspace:* protocol.",
      category: "IMPORTS",
      severity: "HIGH",
      fixHint:
        '"@mcp-rebuild/core": "workspace:*" in dependencies.',
    },
    {
      id: "IMPORT-002",
      title: "Never import from @masday-workflow-reborn/* or @cap/*",
      description:
        "Old package scopes are fully migrated. Use @mcp-rebuild/* only.",
      category: "IMPORTS",
      severity: "CRITICAL",
      negPattern: 'from\\s+["\']@masday-workflow-reborn/',
      fixHint: "Replace with @mcp-rebuild/* imports.",
    },

    // ── GIT ──
    {
      id: "GIT-001",
      title: "Conventional commit messages",
      description:
        "Types: feat, fix, refactor, docs, test, chore, perf, ci.",
      category: "GIT",
      severity: "MEDIUM",
    },
  ],
};

export const REFACTOR_CHECKLIST: RefactorChecklist = {
  version: "1.0.0",
  items: [
    {
      id: "CHK-001",
      label: "Package scope check",
      category: "NAMING",
      required: true,
      description:
        "Verify all package.json files use @mcp-rebuild/* scope.",
    },
    {
      id: "CHK-002",
      label: "Import scope check",
      category: "IMPORTS",
      required: true,
      description:
        "No imports from @masday-workflow-reborn/* or @cap/*.",
    },
    {
      id: "CHK-003",
      label: "ESM .js extensions",
      category: "NAMING",
      required: true,
      description:
        "All relative imports end with .js extension.",
    },
    {
      id: "CHK-004",
      label: "No 'any' types",
      category: "TYPESCRIPT",
      required: true,
      description: "No 'any' types in source code. Use 'unknown' + Zod.",
    },
    {
      id: "CHK-005",
      label: "No console.log",
      category: "TYPESCRIPT",
      required: false,
      description:
        "No console.log in production code. Use Pino logger.",
    },
    {
      id: "CHK-006",
      label: "No hardcoded secrets",
      category: "SECURITY",
      required: true,
      description:
        "No API keys, passwords, tokens in source code.",
    },
    {
      id: "CHK-007",
      label: "MCP tool naming",
      category: "MCP",
      required: true,
      description:
        "MCP tools use camelCase dot-namespaced format.",
    },
    {
      id: "CHK-008",
      label: "Docs tool count accuracy",
      category: "DOCS",
      required: true,
      description:
        "Documentation tool counts match actual registrations.",
    },
    {
      id: "CHK-009",
      label: "Immutability check",
      category: "PATTERN",
      required: false,
      description:
        "No direct mutation of objects. Use spread operators.",
    },
    {
      id: "CHK-010",
      label: "Status UPPERCASE in DB",
      category: "DATABASE",
      required: true,
      description:
        "All status values written to PostgreSQL are UPPERCASE.",
    },
    {
      id: "CHK-011",
      label: "File/function size check",
      category: "PATTERN",
      required: false,
      description:
        "Functions <50 lines, files <400 lines.",
    },
    {
      id: "CHK-012",
      label: "Build passes",
      category: "ARCHITECTURE",
      required: true,
      description: "pnpm build completes without errors.",
    },
    {
      id: "CHK-013",
      label: "Tests pass",
      category: "TESTING",
      required: true,
      description: "pnpm test completes without failures.",
    },
    {
      id: "CHK-014",
      label: "Typecheck passes",
      category: "TYPESCRIPT",
      required: true,
      description: "pnpm typecheck completes without errors.",
    },
  ],
};
