---
name: masday-workflow-init
description: >
  Initialize a new workflow from a user prompt. Checks system readiness, searches memory for
  related past work, analyzes relevant code, and creates the workflow with initial context.
  Use when the user says "start workflow", "create workflow", "new workflow",
  "initialize workflow", or "begin workflow".
allowed-tools:
  - workflow.create
  - workflow.get
  - capability.system_readiness
  - search.code_search
  - memory.search
  - memory.recall_recent
  - memory.recall_documents
  - memory.store
  - filesystem.read
  - filesystem.list
  - filesystem.stat
---

# Masday Workflow Init

Initialize a new Masday workflow from the user's prompt.

## Steps

1. **Parse the user's prompt**
   - Extract: intent (what to accomplish), scope (which packages/files), and constraints
   - Identify key nouns and verbs as search terms

2. **Check system readiness**
   - Call `capability.system_readiness` to verify database connection, schema, and dependencies
   - If any check fails, report the specific issue and stop:
     - Database: check connection string and migrations
     - Schema: verify pgvector extension and table structure
     - Env vars: confirm required environment variables are set

3. **Search for related past work**
   - Call `memory.search` with keywords from the prompt to find similar past workflows
   - Call `memory.recall_recent` to get context from recent sessions
   - Call `memory.recall_documents` to find stored research or decisions

4. **Scan relevant code**
   - Call `search.code_search` with queries derived from the prompt
   - Call `filesystem.list` to verify the project structure matches expectations
   - Call `filesystem.read` to inspect key configuration files (package.json, tsconfig)
   - Call `filesystem.stat` to check file sizes and modification dates

5. **Create the workflow**
   - Call `workflow.create` with:
     - `name`: a concise descriptive name from the prompt
     - `description`: the full scope and intent
   - Record the returned workflow ID

6. **Store initial context**
   - Call `memory.store` with `memory_type: "decision"` for the initial scope and constraints
   - Call `memory.store` with `memory_type: "artifact"` for the related code analysis
   - Include the workflow ID in all stored memories for traceability

7. **Report to the user**
   ```
   Workflow initialized: [wf-001] "Add authentication middleware"
   ID: wf-abc123

   System: Ready (database connected, schema current)
   Related past work: 2 similar workflows found
   Affected packages: orchestrator, core, store
   Key files: packages/core/src/auth.ts, packages/store/src/middleware.ts

   Next steps: Use /masday-workflow-plan to create a task plan
   ```

## Never

- Never create a workflow if system readiness checks fail
- Never skip the memory search for related past work
- Never omit the workflow ID from stored memories
- Never assume the project structure -- always verify with filesystem tools
