---
name: masday-code-analyze
description: >
  Analyze codebase structure, dependencies, patterns, and affected files. Performs semantic
  search, builds context fingerprints, and identifies module relationships. Use when starting
  a new feature, debugging, or understanding the codebase before planning.
allowed-tools:
  - search.code_search
  - search.hybrid_context_pack
  - filesystem.read
  - filesystem.list
  - filesystem.stat
  - filesystem.delete
  - git.status
  - git.diff
---

# Masday Code Analyze

Analyze codebase for Masday workflow context.

## Steps

1. **Scan project structure**
   - Call `filesystem.list` with `recursive: true` on the project root
   - Identify top-level directories and package structure

2. **Get file metadata**
   - Call `filesystem.stat` for key files: package.json, tsconfig.json, entry points
   - Note file sizes and modification dates for change detection

3. **Read key configuration**
   - Call `filesystem.read` on package.json for dependencies and scripts
   - Call `filesystem.read` on tsconfig.json for compiler settings
   - Identify the monorepo package layout (16 packages in this project)

4. **Semantic search**
   - Call `search.code_search` with queries related to the task domain
   - Example: `search.code_search({ query: "workflow engine state machine" })`
   - Identify related modules, shared types, and dependency chains

5. **Build context fingerprint**
   - Call `search.hybrid_context_pack` with the relevant workflow/task IDs
   - This generates a comprehensive context bundle for downstream tasks

6. **Check git state**
   - Call `git.status` for current branch and uncommitted changes
   - Call `git.diff` for staged and unstaged modifications
   - Identify files that have been modified but not yet committed

7. **Identify patterns**
   - Module structure and exports (index.ts barrel files)
   - Dependencies between packages (imports in package.json)
   - Test coverage areas (test file locations)
   - Configuration files (vitest.config.ts, .env patterns)

8. **Summarize findings**
   ```
   Project: masday-workflow-reborn
   Packages: 16 (core, store, db, orchestrator, memory, llm, ...)
   Key deps: @modelcontextprotocol/sdk, zod, pino, better-sqlite3
   Entry: apps/agent-runner (70 MCP tools via stdio)
   Tests: 1017+ across 82+ files (vitest)
   Git: <branch>, <N> modified files
   ```

9. **Clean up**
   - Call `filesystem.delete` for any temporary files created during analysis

## Never

- Never modify source files during analysis -- read-only operation
- Never skip the git state check -- uncommitted changes affect context
- Never fabricate file paths -- only reference files found via filesystem tools
- Never skip the semantic search -- it reveals relationships not visible in file listings
