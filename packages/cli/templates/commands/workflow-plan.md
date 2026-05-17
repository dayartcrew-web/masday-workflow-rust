---
name: masday-workflow-plan
description: Generate a task plan for a workflow without executing — analyzes codebase and creates task breakdown
argument-hint: [workflow-id or prompt]
disable-model-invocation: true
allowed-tools: workflow.get workflow.create workflow.addTask filesystem.read filesystem.list
---

Plan tasks for a Masday workflow. No execution.

## Input
$ARGUMENTS

## Steps

1. **Determine input type**:
   - If UUID format → use `workflow.get` to load existing workflow
   - If text prompt → create new workflow first with `workflow.create`

2. **Analyze** — scan relevant codebase areas with filesystem tools

3. **Generate task plan** — for each task specify:
   - Name and description
   - Agent: backend | frontend | qa | general-purpose
   - Skill: filesystem.read, filesystem.write, git.*, tests.*, npm.*, code.*
   - Dependencies (task IDs or order numbers)
   - Input parameters

4. **Add tasks** to workflow using `workflow.addTask`

5. **Output the plan**:
   ```
   📋 Workflow: <name>
   🆔 ID: <id>
   State: PLAN (ready to execute)
   
   Tasks:
   #1 [backend] Create user model
       → skill: filesystem.write
       → input: { path: "src/models/user.ts", content: "..." }
   
   #2 [backend] Create user API routes  
       → skill: filesystem.write
       → deps: [#1]
   
   #3 [frontend] Create user list component
       → skill: code.generate
       → deps: [#1]
   
   #4 [qa] Write user API tests
       → skill: tests.run
       → deps: [#2]
   
   Parallel: #2 and #3 can run together
   Critical path: #1 → #2 → #4
   
   Execute with: /masday-workflow-run <id>
   ```

Do NOT execute. Plan only.
