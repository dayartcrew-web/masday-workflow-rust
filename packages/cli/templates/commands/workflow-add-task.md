---
name: masday-workflow-add-task
description: Add a single task to an existing workflow manually
argument-hint: [workflow-id] [agent-type] [skill] [description]
disable-model-invocation: true
allowed-tools: workflow.get workflow.addTask
---

Add a task to an existing workflow with full control over parameters.

## Input
$ARGUMENTS — format: `[workflow-id] [agent] [skill] [description]`

Or natural language: "add test task to workflow abc123"

## Steps

1. **Parse input**:
   - Extract workflow ID
   - Extract agent type: backend | frontend | qa | general-purpose
   - Extract skill: filesystem.*, git.*, tests.*, npm.*, code.*
   - Extract description
2. **Verify workflow exists** using `workflow.get`
3. **Add task** using `workflow.addTask`:
   ```json
   {
     "workflowId": "<id>",
     "name": "<description>",
     "agent": "<agent>",
     "skill": "<skill>",
     "dependencies": [],
     "input": {}
   }
   ```
4. **Confirm**:
   ```
   ✅ Task added
   📝 #5 [qa] Run integration tests
   🆔 Workflow: <id>
   Tasks total: 5
   
   → /masday-workflow-run <id> to execute
   ```
