---
name: masday-workflow-new
description: >
  Create and execute a workflow in one shot. Combines init, plan, and run into a single
  streamlined process. Validates at each stage and stores all artifacts. Use when the user
  says "new workflow and run", "create and execute", "do it all", "quick workflow",
  or "end-to-end workflow".
allowed-tools:
  - workflow.create
  - workflow.get
  - workflow.getStatus
  - workflow.execute
  - workflow.createPlan
  - workflow.addTask
  - workflow.listTasks
  - workflow.startTask
  - workflow.completeTask
  - workflow.saveProgress
  - workflow.getCurrentTask
  - capability.system_readiness
  - capability.match_agent
  - capability.list_agents
  - policy.validate_execution
  - policy.validate_completion
  - policy.detect_scope_drift
  - semantic-search.code_search
  - semantic-search.search_hybrid_context_pack
  - memory.search
  - memory.recall_recent
  - memory.recall_documents
  - memory.recall_by_task
  - memory.store
  - tests.run
  - npm.run
  - npm.install
---

# Masday Workflow New

Create and execute a workflow end-to-end in a single session.

## Steps

1. **Parse the prompt and check readiness**
   - Extract intent, scope, and constraints from the user's request
   - Call `capability.system_readiness` to verify the system is ready

2. **Search context**
   - Call `memory.search` for related past workflows
   - Call `memory.recall_recent` for session context
   - Call `semantic-search.code_search` to find related code

3. **Create the workflow**
   - Call `workflow.create` with name and description
   - Record the workflow ID

4. **Build context pack**
   - Call `semantic-search.search_hybrid_context_pack` with the workflow ID
   - Call `memory.recall_documents` for stored research

5. **Plan tasks**
   - Call `capability.list_agents` to see available agents
   - Call `capability.match_agent` for each task type
   - Call `workflow.createPlan` with `workflow_id` and `plan: { tasks: [...] }`
   - Call `workflow.addTask` for each planned task
   - Present the plan briefly and ask for confirmation before executing

6. **Execute the workflow**
   - Call `workflow.execute` with the workflow ID
   - For each task:
     - Call `policy.validate_execution` before starting
     - Call `workflow.getCurrentTask` to track progress
     - Call `memory.recall_by_task` to load task context
     - Perform the work
     - Call `policy.detect_scope_drift` to check for deviations
     - Call `workflow.saveProgress` with notes and evidence
     - Call `policy.validate_completion` after completing
     - Call `workflow.completeTask` to mark done

7. **Store artifacts**
   - Call `memory.store` with key decisions and outputs

8. **Report final status**
   - Call `workflow.getStatus` for the final state
   - Summarize all tasks, any failures, and recommended follow-ups

## Never

- Never skip user confirmation between planning and execution
- Never skip policy validation at task boundaries
- Never proceed if system readiness fails
- Never ignore scope drift -- pause and report to the user

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
