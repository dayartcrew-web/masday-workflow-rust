---
name: masday-workflow-plan
description: >
  Generate a structured task plan for a workflow without executing it. Analyzes the codebase,
  searches for related context, matches agents, and creates a detailed task breakdown.
  Use when the user says "plan workflow", "create plan", "task breakdown",
  "design workflow", or "plan without running".
allowed-tools:
  - workflow.get
  - workflow.create
  - workflow.create_plan
  - workflow.addTask
  - workflow.listTasks
  - capability.system_readiness
  - capability.match_agent
  - capability.list_agents
  - semantic-search.code_search
  - semantic-search.search_hybrid_context_pack
  - memory.search
  - memory.recall_documents
  - memory.recall_recent
  - memory.store
---

# Masday Workflow Plan

Generate a task plan for a Masday workflow. No execution -- planning only.

## Steps

1. **Get or create workflow**
   - If the user provides a workflow ID: call `workflow.get` to verify it exists
   - Otherwise: call `workflow.create` with a descriptive name and metadata from the prompt

2. **Check system readiness**
   - Call `capability.system_readiness` to confirm database, schema, and dependencies are healthy
   - If readiness fails, report the issue and stop

3. **Gather context**
   - Call `memory.search` with keywords from the user's request to find related past workflows
   - Call `memory.recall_documents` to load any stored research or decisions
   - Call `memory.recall_recent` to check for context from the current session

4. **Analyze codebase**
   - Call `semantic-search.code_search` with queries matching the task domain
   - Call `semantic-search.search_hybrid_context_pack` with the workflow ID to build a full context pack
   - Identify affected packages, files, and dependencies

5. **Match agents to task types**
   - Call `capability.list_agents` to see available agents
   - For each identified task type, call `capability.match_agent` with a task description
   - Record the best-matching agent for each task

6. **Create the plan**
   - Call `workflow.create_plan` with:
     - A summary of the plan
     - An array of tasks, each with `title`, `ownerAgent`, `priority`, `acceptanceCriteria`
   - Include dependencies between tasks where applicable

7. **Add individual tasks**
   - For each task in the plan, call `workflow.addTask` with:
     - `name`, `agent` (matched agent), `skill` (appropriate skill), `input` (parameters)
     - `dependencies` array referencing prerequisite task IDs

8. **Store planning artifacts**
   - Call `memory.store` with `memory_type: "artifact"` to save the plan summary
   - Call `memory.store` with `memory_type: "decision"` to record key design choices

9. **Present the plan**
   - Display the task list with agents, dependencies, and acceptance criteria
   - Ask the user to review before proceeding to execution

## Never

- Never execute tasks -- this skill is planning only
- Never skip the readiness check
- Never create tasks without matching agents first
- Never omit acceptance criteria from tasks
