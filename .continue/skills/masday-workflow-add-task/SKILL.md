---
name: masday-workflow-add-task
description: >
  Add a new task to an existing workflow. Finds the best matching agent, determines
  dependencies, and integrates the task into the plan. Use when the user says "add task",
  "insert task", "new task to workflow", or "extend the plan".
allowed-tools:
  - workflow.get
  - workflow.getStatus
  - workflow.listTasks
  - workflow.addTask
  - capability.match_agent
  - capability.list_agents
  - capability.list_skills
  - memory.store
  - memory.recall_by_task
---

# Masday Workflow Add Task

Add a new task to an existing workflow.

## Steps

1. **Verify the workflow exists**
   - Call `workflow.get` with the workflow ID to confirm it exists
   - Call `workflow.getStatus` to check it is not in DONE state
   - If the workflow is already completed, inform the user and suggest creating a new one

2. **Review current tasks**
   - Call `workflow.listTasks` to see existing tasks and their statuses
   - Identify the last task in the sequence to determine where to insert the new task
   - Note any tasks already in progress or completed that might be dependencies

3. **Match the best agent**
   - Call `capability.list_agents` to see all available agents
   - Call `capability.match_agent` with a description of the new task
   - Select the agent with the highest relevance score

4. **Verify the skill exists**
   - Call `capability.list_skills` to confirm the required skill is registered
   - If the skill does not exist, suggest creating it with `masday-create-skill`

5. **Recall related context**
   - Call `memory.recall_by_task` with related task IDs for context

6. **Determine dependencies**
   - Based on the task description, determine which existing tasks must complete first
   - Reference task IDs from the `workflow.listTasks` result
   - If the task is independent, set dependencies to an empty array

7. **Add the task**
   - Call `workflow.addTask` with:
     - `workflowId`: the target workflow ID
     - `name`: clear, concise task title
     - `agent`: the matched agent name
     - `skill`: the appropriate skill for the task type
     - `input`: parameters derived from the user's description
     - `dependencies`: array of prerequisite task IDs

8. **Store the change**
   - Call `memory.store` with `memory_type: "decision"` recording the task addition
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

- Never add tasks to a completed (DONE) workflow
- Never skip the agent matching step
- Never add a task without verifying its dependencies exist
- Never modify existing tasks -- only add new ones
