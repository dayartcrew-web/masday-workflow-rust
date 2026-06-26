---
name: masday-workflow-add-task
description: >
  Add a new task to an existing workflow. Finds the best matching agent, determines
  dependencies, and integrates the task into the plan. Use when the user says "add task",
  "insert task", "new task to workflow", or "extend the plan".
allowed-tools:
  - workflow_get
  - workflow_getStatus
  - workflow_listTasks
  - workflow_addTask
  - capability_match_agent
  - capability_list_agents
  - capability_list_skills
  - memory_store
  - memory_recall_by_task
---

# Masday Workflow Add Task

Add a new task to an existing workflow.

## Steps

This skill enforces **mandatory step completion**. Each step must be completed before proceeding. Do not skip steps.


1. **Verify the workflow exists**
   - Call `workflow_get` with the workflow ID to confirm it exists
   - Call `workflow_getStatus` to check it is not in DONE state
   - If the workflow is already completed, inform the user and suggest creating a new one

2. **Review current tasks**
   - Call `workflow_listTasks` to see existing tasks and their statuses
   - Identify the last task in the sequence to determine where to insert the new task
   - Note any tasks already in progress or completed that might be dependencies

3. **Match the best agent**
   - Call `capability_list_agents` to see all available agents
   - Call `capability_match_agent` with a description of the new task
   - Select the agent with the highest relevance score

4. **Verify the skill exists**
   - Call `capability_list_skills` to confirm the required skill is registered
   - If the skill does not exist, suggest creating it with `masday-create-skill`

5. **Recall related context**
   - Call `memory_recall_by_task` with related task IDs for context


**GATE**: Verify steps 1-5 are complete before proceeding.

6. **Determine dependencies**
   - Based on the task description, determine which existing tasks must complete first
   - Reference task IDs from the `workflow_listTasks` result
   - If the task is independent, set dependencies to an empty array

7. **Add the task**
   - Call `workflow_addTask` with:
     - `workflowId`: the target workflow ID
     - `name`: clear, concise task title
     - `agent`: the matched agent name
     - `skill`: the appropriate skill for the task type
     - `input`: parameters derived from the user's description
     - `dependencies`: array of prerequisite task IDs

8. **Store the change**
   - Call `memory_store` with `memory_type: "decision"` recording the task addition
   - Include the reason for adding the task and the chosen agent

9. **Report**
   ```
   Task added to workflow [wf-001]:
   - Name: "Implement caching layer"
   - Agent: architect (matched 0.92 relevance)
   - Skill: masday-code-analyze
   - Dependencies: [task-003]
   - Status: Pending (will execute after dependencies complete)
   ```

## Never
- Never skip any step — complete each step before proceeding
- Never bypass a GATE marker without validating prior steps
- Never claim completion without executing all steps in order

- Never add tasks to a completed (DONE) workflow
- Never skip the agent matching step
- Never add a task without verifying its dependencies exist
- Never modify existing tasks -- only add new ones

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
