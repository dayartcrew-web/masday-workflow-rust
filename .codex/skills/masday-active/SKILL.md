---
name: masday-active
description: >
  Universal entry point for any user instruction. Understands natural language requests
  (even without / prefix) and routes them through the use_masday MCP tool and
  masday-orchestrator to execute the appropriate masday MCP workflow.
  Use when the user gives ANY instruction — coding, fixing, building, testing, deploying,
  researching, or managing workflows.
allowed-tools:
  - mcp__masday__capability_match_agent
  - mcp__masday__capability_list_agents
  - mcp__masday__capability_list_skills
  - mcp__masday__capability_system_readiness
  - mcp__masday__workflow_create
  - mcp__masday__workflow_createPlan
  - mcp__masday__workflow_addTask
  - mcp__masday__workflow_execute
  - mcp__masday__workflow_startTask
  - mcp__masday__workflow_saveProgress
  - mcp__masday__workflow_completeTask
  - mcp__masday__workflow_getStatus
  - mcp__masday__workflow_getCurrentTask
  - mcp__masday__workflow_getActive
  - mcp__masday__memory_search
  - mcp__masday__memory_store
  - mcp__masday__memory_recall_recent
  - mcp__masday__semantic-search_code_search
  - mcp__masday__semantic-search_search_hybrid_context_pack
  - mcp__masday__policy_validate_execution
  - mcp__masday__policy_validate_completion
  - mcp__masday__policy_detect_scope_drift
  - mcp__masday__review_submit
  - mcp__masday__local_sync
  - mcp__masday__session_init_context
---

# masday-use-masday

Universal entry point — understands any user instruction and routes it through masday workflows.

## Trigger

Any user instruction that involves coding, fixing, building, testing, deploying, researching,
or managing workflows — even without a `/` prefix.

Examples: "fix X", "build Y", "add feature Z", "deploy", "test this", "research X",
"create workflow", "analyze the codebase", "run tests", "commit changes".

## Steps

1. **Parse intent**
   - Determine the category: fix, build, test, deploy, research, scaffold, analyze, or workflow management
   - Extract scope (files, packages, features affected)
   - Identify if it's a quick task (single step) or complex task (multi-step workflow)

2. **Check system readiness**
   - Call `capability_system_readiness` to verify the masday system is operational
   - If readiness fails, report the issue and stop

3. **Match the best agent**
   - Call `capability_match_agent` with the task description
   - Review the matched agent's capabilities
   - For multi-step work, also call `capability_list_agents` to consider parallel delegation

4. **Route to the appropriate workflow**
   - **If a specific masday skill matches** (e.g., masday-tdd, masday-workflow-new, masday-research):
     - Invoke that skill directly via the Skill tool
     - The skill handles its own workflow lifecycle
   - **If no specific skill matches but task is complex**:
     - Create a workflow via `workflow_create`
     - Plan tasks via `workflow_createPlan` and `workflow_addTask`
     - Execute via `workflow_execute` using the matched agent
   - **If task is quick** (single-step fix, lookup, simple edit):
     - Execute directly using the appropriate MCP tools
     - Skip full workflow overhead

5. **Execute with review pipeline**
   - For each task in a workflow:
     - `policy_validate_execution` before starting
     - Perform the work
     - `workflow_saveProgress` with notes and evidence
     - `review_submit` for quality gate
     - If REWORK_REQUIRED: fix and re-submit (max 2 attempts)
     - If APPROVED: `policy_validate_completion` → `workflow_completeTask`
     - `local_sync` after completion

6. **Report results**
   - Clear summary of what was done
   - Files modified, tests run, any failures
   - Follow-up recommendations

## Intent Routing Table

| User Says | Routes To |
|-----------|-----------|
| "fix X", "bug in X" | masday-workflow-fix or direct fix |
| "build X", "add feature X" | masday-workflow-new |
| "test X", "write tests" | masday-tdd |
| "deploy", "check deploy" | masday-deploy-check |
| "research X", "look up X" | masday-research |
| "create agent/skill" | masday-create-agent / masday-create-skill |
| "run parallel" | masday-parallel-execution |
| "audit workflow" | masday-workflow-audit |
| "commit", "push", "PR" | masday-git-workflow / masday-github-pr |
| "analyze code" | masday-code-analyze |
| Any other instruction | workflow_create + matched agent |

## Never

- Never skip user confirmation between planning and execution
- Never skip policy validation at task boundaries
- Never proceed if system readiness fails
- Never ignore scope drift — pause and report to the user
- Never bypass the review pipeline (review_submit → policy_validate_completion → workflow_completeTask)
- Never claim done without saving progress to PostgreSQL
