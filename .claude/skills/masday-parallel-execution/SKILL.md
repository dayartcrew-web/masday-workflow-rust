---
name: masday-parallel-execution
description: >
  Run independent subtasks in parallel using agent dispatch and parallel branch management.
  Creates branches for independent work, dispatches agents, and synthesizes results.
  Use when the user says "run in parallel", "parallel execution", "dispatch agents",
  "multiple tasks at once", or "fan-out execution".
allowed-tools:
  - Agent
  - workflow_get
  - workflow_getStatus
  - workflow_listTasks
  - workflow_createParallelBranches
  - workflow_completeParallelBranch
  - workflow_listParallelBranches
  - workflow_saveProgress
  - memory_recall_documents
  - memory_store
---

# Masday Parallel Execution

Run independent subtasks in parallel using agent dispatch.

## Steps

1. **Identify parallelizable tasks**
   - Call `workflow_get` and `workflow_listTasks` to see the current plan
   - Identify tasks with no dependencies on each other
   - Group independent tasks into parallel branches

2. **Recall context for all branches**
   - Call `memory_recall_documents` for shared workflow context
   - Ensure each branch has sufficient context to execute independently

3. **Create parallel branches**
   - Call `workflow_createParallelBranches` with:
     - `workflow_id`: the target workflow
     - `task_id`: the parent task that owns the branches
     - `branches`: array of branch definitions:
       ```
       [
         { branchKey: "branch-a", role: "researcher", scope: "analyze dependencies" },
         { branchKey: "branch-b", role: "coder", scope: "implement feature" },
         { branchKey: "branch-c", role: "tester", scope: "write tests" }
       ]
       ```
   - Each branch gets its own scope and role

4. **Dispatch agents for each branch**
   - For each branch, dispatch a subagent with:
     - The branch scope and role
     - Shared context from `memory_recall_documents`
     - Specific tools needed for the branch task
   - Agents execute independently without shared mutable state

5. **Monitor branch progress**
   - Call `workflow_listParallelBranches` to check branch statuses
   - Call `workflow_saveProgress` for each branch milestone
   - Do not wait for all branches -- process completions as they arrive

6. **Complete each branch**
   - As each agent finishes, call `workflow_completeParallelBranch` with:
     - `branch_id`: the completed branch ID
     - `output`: the branch results
     - `agent_name`: the agent that completed the work

7. **Synthesize results**
   - After all branches complete, synthesize the outputs:
     - Merge findings from research branches
     - Integrate code from implementation branches
     - Incorporate test results from testing branches
   - Resolve any conflicts between branch outputs

8. **Store synthesis**
   - Call `memory_store` with `memory_type: "artifact"` containing the synthesis
   - Include: branch outputs, conflicts resolved, and final deliverables

9. **Report**
   ```
   === Parallel Execution Complete ===
   Workflow: [wf-001]

   Branches:
   - branch-a (researcher): COMPLETE - 5 dependencies analyzed
   - branch-b (coder): COMPLETE - 3 files modified
   - branch-c (tester): COMPLETE - 12 tests written

   Synthesis: All branches converged successfully
   Conflicts: 0
   ```

## Never

- Never create branches with dependencies between them -- branches must be independent
- Never allow branches to modify the same files simultaneously
- Never skip the synthesis step after all branches complete
- Never assume branch order -- process completions as they arrive

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
