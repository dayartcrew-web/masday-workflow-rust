---
name: masday-parallel-execution
description: >
  Run independent subtasks in parallel using agent dispatch and parallel branch management.
  Creates branches for independent work, dispatches agents, and synthesizes results.
  Use when the user says "run in parallel", "parallel execution", "dispatch agents",
  "multiple tasks at once", or "fan-out execution".
allowed-tools:
  - Agent
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - workflow.createParallelBranches
  - workflow.completeParallelBranch
  - workflow.listParallelBranches
  - workflow.saveProgress
  - memory.recall_documents
  - memory.store
---

# Masday Parallel Execution

Run independent subtasks in parallel using agent dispatch.

## Steps

1. **Identify parallelizable tasks**
   - Call `workflow.get` and `workflow.listTasks` to see the current plan
   - Identify tasks with no dependencies on each other
   - Group independent tasks into parallel branches

2. **Recall context for all branches**
   - Call `memory.recall_documents` for shared workflow context
   - Ensure each branch has sufficient context to execute independently

3. **Create parallel branches**
   - Call `workflow.createParallelBranches` with:
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
     - Shared context from `memory.recall_documents`
     - Specific tools needed for the branch task
   - Agents execute independently without shared mutable state

5. **Monitor branch progress**
   - Call `workflow.listParallelBranches` to check branch statuses
   - Call `workflow.saveProgress` for each branch milestone
   - Do not wait for all branches -- process completions as they arrive

6. **Complete each branch**
   - As each agent finishes, call `workflow.completeParallelBranch` with:
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
   - Call `memory.store` with `memory_type: "artifact"` containing the synthesis
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
