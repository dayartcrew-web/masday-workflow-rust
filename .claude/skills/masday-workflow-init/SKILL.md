---
name: masday-workflow-init
description: >
  Initialize a new workflow from a user prompt. Searches memory (local + remote) for related
  past work, scans relevant code, checks system readiness, and creates the workflow.
  If invoked without a prompt, auto-continues through all steps and asks user to pick next step.
  Use when the user says "start workflow", "create workflow", "new workflow",
  "initialize workflow", or "begin workflow".
allowed-tools:
  - workflow.create
  - workflow.get
  - capability.system_readiness
  - semantic-search.code_search
  - memory.search
  - memory.recall_recent
  - memory.recall_documents
  - memory.store
  - filesystem.read
  - filesystem.list
  - filesystem.stat
---

# Masday Workflow Init

Initialize a new Masday workflow. Searches memory and relevant code, creates the workflow record.

## Steps

1. **Parse the user's prompt**
   - If user provided a prompt: extract intent, scope, and constraints. Use key nouns/verbs as search terms.
   - If invoked without a prompt (bare command): use "recent project work" as default search term and continue through all steps automatically.

2. **Search memory (local + remote)**
   - Call `memory.search` with keywords from the prompt to find similar past workflows
   - Call `memory.recall_recent` to get context from recent sessions
   - Call `memory.recall_documents` to find stored research or decisions

3. **Scan relevant code**
   - Call `semantic-search.code_search` with queries derived from the prompt
   - Call `filesystem.list` to verify project structure
   - Call `filesystem.read` to inspect key config files (package.json, tsconfig)
   - Call `filesystem.stat` to check file sizes and modification dates

4. **Check system readiness**
   - Call `capability.system_readiness` to verify database connection, schema, and dependencies
   - If any check fails, report the specific issue and stop

5. **Create the workflow**
   - Call `workflow.create` with:
     - `name`: a concise descriptive name from the prompt
     - `description`: the full scope and intent
   - Record the returned workflow ID

6. **Store initial context**
   - Call `memory.store` with `memory_type: "decision"` for the initial scope and constraints
   - Call `memory.store` with `memory_type: "artifact"` for the related code analysis
   - Include the workflow ID in all stored memories for traceability

7. **Report and ask next step**
   Use AskUserQuestion to present results and let the user pick:
   ```
   Workflow initialized: [wf-001] "Add authentication middleware"
   ID: wf-abc123

   Memory: 2 similar workflows found (or: none found)
   Code: packages/core/src/auth.ts, packages/store/src/middleware.ts
   System: Ready (database connected, schema current)
   ```

   Ask user:
   - "/masday-workflow-plan — create a task plan"
   - "/masday-workflow-new — plan and execute in one pass"
   - "Continue with another task"

## Never

- Never create a workflow if system readiness checks fail
- Never skip the memory search — always search local AND remote
- Never omit the workflow ID from stored memories

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow.saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review.submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow.saveProgress)
  - Re-submit review (review.submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy.validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow.completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local.sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow.completeTask without review.submit (APPROVED)
- Never skip policy.validate_completion before completion
- Never skip local.sync after completing a task
- Never claim done without saving progress to PostgreSQL
