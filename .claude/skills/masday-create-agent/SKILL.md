---
name: masday-create-agent
description: >
  Create a new Masday agent definition from a description. Designs the agent's role,
  preferred tools, task routing, and constraints. Registers and saves to all locations.
  Use when the user says "create agent", "new agent", "add agent", or "agent specialization".
allowed-tools:
  - capability.create_agent
  - capability.list_agents
  - capability.list_templates
  - capability.scaffold_feature
  - filesystem.write
  - filesystem.list
---

# Masday Create Agent

Create a new Masday Workflow agent.

## Steps

1. **Check existing agents**
   - Call `capability.list_agents` to see all registered agents
   - Verify the proposed agent name does not conflict with existing ones

2. **Check templates**
   - Call `capability.list_templates` for available scaffolding patterns
   - Select a template if one matches the agent type

3. **Design the agent based on user description**
   - Name: kebab-case (e.g., `security-reviewer`, `api-designer`)
   - Role: concise description of specialization
   - Preferred skills: which skills this agent should use
   - Preferred tools: which MCP tools this agent needs
   - Task routing rules: what types of tasks to assign to this agent
   - Constraints: what the agent must never do

4. **Register the agent**
   - Call `capability.create_agent` with:
     - `name`: the agent name
     - `role`: the agent's role description
     - `description`: what this agent does
     - `instructions`: detailed instructions including constraints and workflow

5. **Save to project location**
   - Call `filesystem.write` to save the agent definition:
     - `$ROOT/.claude/agents/<name>.md`

6. **Or use scaffold for full feature**
   - Call `capability.scaffold_feature` if the agent needs an accompanying skill and command

7. **Report**
   ```
   Agent created: "<name>"
   Role: <role description>
   Registered via capability API
   Saved to: .claude/agents/<name>.md
   ```

## Never

- Never create an agent with the same name as an existing one
- Never omit the task routing rules -- agents need clear routing criteria
- Never skip registration with `capability.create_agent`
- Never create agents without clear constraints

## Mandatory Review Pipeline

When this skill completes work on a workflow task, it MUST follow this pipeline:

`
STEP 1: Save progress to PostgreSQL
  workflow.saveProgress({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    agent_name: "<current-agent>",
    progress_note: "<summary of work done>",
    evidence: ["<files modified>", "<tests run>"]
  })

STEP 2: Submit for review
  review.submit({
    workflow_id: "<workflowId>",
    task_id: "<taskId>",
    reviewer_agent: "masday-reviewer",
    decision: "<APPROVED | REWORK_REQUIRED | BLOCKED>",
    notes: "<what was done, key decisions>",
    gaps: ["<any gaps found>"]
  })

STEP 3: If REWORK_REQUIRED — fix and loop
  - Fix the gaps identified in the review
  - Re-save progress (workflow.saveProgress)
  - Re-submit review (review.submit)
  - Max 2 rework attempts, then STOP

STEP 4: If APPROVED — validate completion
  policy.validate_completion({
    workflow_id: "<workflowId>",
    task_id: "<taskId>"
  })

STEP 5: Complete task
  workflow.completeTask({ workflow_id: "<workflowId>", task_id: "<taskId>" })

STEP 6: Sync local state
  local.sync({ cwd: process.cwd(), workflow_id: "<workflowId>" })
`

### Never
- Never call workflow.completeTask without review.submit (APPROVED)
- Never skip policy.validate_completion before completion
- Never skip local.sync after completing a task
- Never claim done without saving progress to PostgreSQL
