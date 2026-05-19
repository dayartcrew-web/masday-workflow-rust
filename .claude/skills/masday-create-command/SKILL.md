---
name: masday-create-command
description: >
  Create a new slash command for Masday Workflow. Designs the command name, arguments,
  tool requirements, and invocation behavior. Saves to project commands directory.
  Use when the user says "create command", "new command", "add slash command", or "command definition".
allowed-tools:
  - filesystem.list
  - filesystem.write
---

# Masday Create Command

Create a new Masday slash command.

## Steps

1. **Check existing commands**
   - Call `filesystem.list` on `.claude/commands/` to see existing commands
   - Verify the proposed command name does not conflict

2. **Design the command**
   - Name: kebab-case (e.g., `masday-deploy`, `masday-status`)
   - Description: what the command does in under 250 characters
   - Arguments: required and optional argument hints
   - Auto/manual invocation: should it trigger automatically or only on slash?
   - Required tools: which MCP tools the command needs
   - Context mode: fork (for heavy tasks) or inline (for quick tasks)

3. **Generate command file** with YAML frontmatter:
   ```yaml
   ---
   name: <command-name>
   description: <what it does -- under 250 chars>
   argument-hint: [required-args] [optional-args]
   allowed-tools:
     - tool.name
   ---

   # <Command Name>

   <Instructions Claude follows when the command is invoked>

   ## Steps
   <numbered steps with tool call examples>

   ## Never
   <constraints>
   ```

4. **Save to project location**
   - Call `filesystem.write` to save:
     - `$ROOT/.claude/commands/<name>.md`

5. **Report**
   ```
   Command created: "/<name>"
   Arguments: <argument-hint>
   Tools: <list of allowed tools>
   Saved to: .claude/commands/<name>.md
   ```

## Never

- Never create a command without a clear argument specification
- Never use spaces in command names -- use kebab-case
- Never save commands outside the `.claude/commands/` directory
- Never create commands that duplicate existing skill functionality without good reason

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
