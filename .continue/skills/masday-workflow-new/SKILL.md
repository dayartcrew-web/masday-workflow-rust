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
  - workflow.create_plan
  - workflow.addTask
  - workflow.list_tasks
  - workflow.start_task
  - workflow.complete_task
  - workflow.save_progress
  - workflow.get_current_task
  - capability.system_readiness
  - capability.match_agent
  - capability.list_agents
  - policy.validate_execution
  - policy.validate_completion
  - policy.detect_scope_drift
  - search.code_search
  - search.hybrid_context_pack
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
   - Call `search.code_search` to find related code

3. **Create the workflow**
   - Call `workflow.create` with name and description
   - Record the workflow ID

4. **Build context pack**
   - Call `search.hybrid_context_pack` with the workflow ID
   - Call `memory.recall_documents` for stored research

5. **Plan tasks**
   - Call `capability.list_agents` to see available agents
   - Call `capability.match_agent` for each task type
   - Call `workflow.create_plan` with the full task breakdown
   - Call `workflow.addTask` for each planned task
   - Present the plan briefly and ask for confirmation before executing

6. **Execute the workflow**
   - Call `workflow.execute` with the workflow ID
   - For each task:
     - Call `policy.validate_execution` before starting
     - Call `workflow.get_current_task` to track progress
     - Call `memory.recall_by_task` to load task context
     - Perform the work
     - Call `policy.detect_scope_drift` to check for deviations
     - Call `workflow.save_progress` with notes and evidence
     - Call `policy.validate_completion` after completing
     - Call `workflow.complete_task` to mark done

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
