---
name: masday-workflow-init
description: Initialize a new workflow from a prompt — creates workflow and sets up initial context
argument-hint: [prompt — describe what you want to build]
disable-model-invocation: true
allowed-tools: workflow.create filesystem.read filesystem.list
context: fork
---

Initialize a new Masday workflow from the user's prompt.

## Input
$ARGUMENTS

## Steps

1. **Parse the prompt** — extract intent, scope, and target area
2. **Quick scan** — use `filesystem.list` and `filesystem.read` to understand current state
3. **Create workflow** using `workflow.create`:
   ```json
   {
     "name": "<short-name-from-prompt>",
     "description": "<full-prompt>",
     "metadata": {
       "source": "claude-code",
       "prompt": "<original-prompt>",
       "createdAt": "<now>"
     }
   }
   ```
4. **Report back**:
   ```
   ✅ Workflow initialized
   🆔 ID: <workflow-id>
   📝 Name: <name>
   
   Next steps:
   → /masday-workflow-plan <workflow-id>  — Plan tasks
   → /masday-workflow-run <workflow-id>   — Skip plan, auto-generate & run
   → /masday-workflow-status              — Check all workflows
   ```

Do NOT add tasks or execute yet. INIT phase only.
