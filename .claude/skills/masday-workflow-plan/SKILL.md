---
name: masday-workflow-plan
description: >
  Generate a structured task plan for a workflow without executing it. Analyzes the codebase,
  searches for related context, matches agents, and creates a detailed task breakdown.
  Use when the user says "plan workflow", "create plan", "task breakdown",
  "design workflow", or "plan without running".
allowed-tools:
  - workflow_get
  - workflow_create
  - workflow_createPlan
  - workflow_addTask
  - workflow_listTasks
  - capability_system_readiness
  - capability_match_agent
  - capability_list_agents
  - semantic-search_code_search
  - semantic-search_search_hybrid_context_pack
  - memory_search
  - memory_recall_documents
  - memory_recall_recent
  - memory_store
---

# Masday Workflow Plan

Generate a task plan for a Masday workflow. No execution -- planning only.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Get or create workflow**
   - If the user provides a workflow ID: call `workflow_get` to verify it exists
   - Otherwise: call `workflow_create` with a descriptive name and metadata from the prompt

2. **Check system readiness**
   - Call `capability_system_readiness` to confirm database, schema, and dependencies are healthy
   - If readiness fails, report the issue and stop

3. **Gather context**
   - Call `memory_search` with keywords from the user's request to find related past workflows
   - Call `memory_recall_documents` to load any stored research or decisions
   - Call `memory_recall_recent` to check for context from the current session

4. **Analyze codebase**
   - Call `semantic-search_code_search` with queries matching the task domain
   - Call `semantic-search_search_hybrid_context_pack` with the workflow ID to build a full context pack
   - Identify affected packages, files, and dependencies

5. **Match agents to task types**
   - Call `capability_list_agents` to see available agents
   - For each identified task type, call `capability_match_agent` with a task description
   - Record the best-matching agent for each task


**GATE**: Verify steps 1-5 are complete before proceeding.

6. **Create the plan**
   - Call `workflow_createPlan` with:
     - `workflow_id`: the workflow ID
     - `plan`: `{ tasks: [{ title, agent, skill, dependencies, input }] }`
   - Include dependencies between tasks where applicable

7. **Add individual tasks**
   - For each task in the plan, call `workflow_addTask` with:
     - `name`, `agent` (matched agent), `skill` (appropriate skill), `input` (parameters)
     - `dependencies` array referencing prerequisite task IDs

8. **Store planning artifacts**
   - Call `memory_store` with `memory_type: "artifact"` to save the plan summary
   - Call `memory_store` with `memory_type: "decision"` to record key design choices

9. **Present the plan**
   - Display the task list with agents, dependencies, and acceptance criteria
   - Ask the user to review before proceeding to execution

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never execute tasks -- this skill is planning only
- Never skip the readiness check
- Never create tasks without matching agents first
- Never omit acceptance criteria from tasks

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
