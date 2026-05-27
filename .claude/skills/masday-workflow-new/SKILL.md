---
name: masday-workflow-new
description: >
  Create and execute a workflow in one shot. Combines init, plan, and run into a single
  streamlined process. Validates at each stage and stores all artifacts. Use when the user
  says "new workflow and run", "create and execute", "do it all", "quick workflow",
  or "end-to-end workflow".
allowed-tools:
  - workflow_create
  - workflow_get
  - workflow_getStatus
  - workflow_execute
  - workflow_createPlan
  - workflow_addTask
  - workflow_listTasks
  - workflow_startTask
  - workflow_completeTask
  - workflow_saveProgress
  - workflow_getCurrentTask
  - capability_system_readiness
  - capability_match_agent
  - capability_list_agents
  - policy_validate_execution
  - policy_validate_completion
  - policy_detect_scope_drift
  - semantic-search_code_search
  - semantic-search_search_hybrid_context_pack
  - memory_search
  - memory_recall_recent
  - memory_recall_documents
  - memory_recall_by_task
  - memory_store
  - tests_run
  - npm_run
  - npm_install
---

# Masday Workflow New

Create and execute a workflow end-to-end in a single session.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Parse the prompt and check readiness**
   - Extract intent, scope, and constraints from the user's request
   - Call `capability_system_readiness` to verify the system is ready

2. **Search context**
   - Call `memory_search` for related past workflows
   - Call `memory_recall_recent` for session context
   - Call `semantic-search_code_search` to find related code

3. **Create the workflow**
   - Call `workflow_create` with name and description
   - Record the workflow ID

4. **Build context pack**
   - Call `semantic-search_search_hybrid_context_pack` with the workflow ID
   - Call `memory_recall_documents` for stored research

5. **Match the best agent**
   - Call `capability_list_agents` to see all available agents
   - Call `capability_match_agent` with a description of the each task type
    - For example, if the workflow involves coding, match an agent with strong coding capabilities
   - Select the agent with the highest relevance score


**GATE**: Verify steps 1-5 are complete before proceeding.

6. **Verify the skill exists**
   - Call `capability_list_skills` to confirm the required skill is registered
   - If the skill does not exist, suggest creating it with `masday-create-skill`

7. **Plan tasks**
   - Call `workflow_createPlan` with `workflow_id` and `plan: { tasks: [...] }`
   - Call `workflow_addTask` for each planned task
   - Present the plan briefly and ask for confirmation before executing

8. **Execute the workflow**
   - Call `workflow_execute` with the workflow ID
   - For each task:
     - Call `policy_validate_execution` before starting
     - Call `workflow_getCurrentTask` to track progress
     - Call `memory_recall_by_task` to load task context
     - Perform the work
     - Call `policy_detect_scope_drift` to check for deviations
     - Call `workflow_saveProgress` with notes and evidence
     - Call `policy_validate_completion` after completing
     - Call `workflow_completeTask` to mark done

9. **Store artifacts**
   - Call `memory_store` with key decisions and outputs

10. **Report final status**
   - Call `workflow_getStatus` for the final state
   - Summarize all tasks, any failures, and recommended follow-ups

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never skip user confirmation between planning and execution
- Never skip policy validation at task boundaries
- Never proceed if system readiness fails
- Never ignore scope drift -- pause and report to the user

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow_saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review_submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow_saveProgress)
  - Re-submit review (review_submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy_validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow_completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local_sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow_completeTask without review_submit (APPROVED)
- Never skip policy_validate_completion before completion
- Never skip local_sync after completing a task
- Never claim done without saving progress to PostgreSQL
