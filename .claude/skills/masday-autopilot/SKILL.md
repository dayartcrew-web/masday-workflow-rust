---
name: masday-autopilot
description: >
  Auto-pilot task execution — run all pending tasks sequentially or in parallel with worktree isolation.
  Dispatches executor agents, runs review gates, and completes tasks automatically.
  Use when the user says "autopilot", "run all tasks", "auto execute", or "fly through the plan".
allowed-tools:
  - workflow.getActive
  - workflow.getPlan
  - workflow.listTasks
  - workflow.startTask
  - workflow.completeTask
  - workflow.saveProgress
  - review.submit
  - review.get_latest
  - local.init
  - local.sync
  - semantic-search.search_hybrid_context_pack
  - memory.recall_by_task
  - memory.store
  - Agent
  - Bash
  - AskUserQuestion
  - TodoWrite
  - EnterWorktree
  - ExitWorktree
---

# Masday Autopilot

Execute all pending tasks in the active workflow automatically. Dispatches executor agents, runs review gates, handles rework loops, and completes tasks — all without manual intervention between tasks.

## Pre-Conditions (STOP if any fail)

```
Step 0a — Init .masday/:
  local.init({ cwd: process.cwd() })

Step 0b — Get active workflow:
  workflow.getActive({ cwd: process.cwd() })
  If none: STOP — "No active workflow. Run /masday-workflow-new first."

Step 0c — Get plan:
  workflow.getPlan({ workflow_id: workflowId })
  If no plan: STOP — "No plan found. Run /masday-workflow-plan first."

Step 0d — Count pending:
  pending = tasks where status is "PENDING"
  If pending.length === 0: STOP — "All tasks completed."
```

## Configuration (AskUserQuestion)

Ask the user with these options:

1. **Execution Mode**:
   - Sequential (default) — one task at a time, ordered by priority
   - Parallel — batch independent tasks, max 4 simultaneous

2. **Worktree Isolation**:
   - `.masday/worktrees/` (default) — each task gets a git worktree in `.masday/worktrees/{slug}/`, auto-creates branch `task/{slug}`, auto-creates PR on completion
   - Built-in project — work directly in the current project directory, no isolation

3. **Review Gate**:
   - Semi-auto (default) — auto-complete on APPROVED, auto-fix once on REWORK then stop
   - Auto — auto-complete on APPROVED, auto-fix up to 2x on REWORK
   - Manual — pause for user confirmation on every review result

| Review Gate | APPROVED | REWORK_REQUIRED | BLOCKED |
|-------------|----------|-----------------|---------|
| Auto | Auto-complete | Auto-fix up to 2x | Stop |
| Semi-auto | Auto-complete | Auto-fix once, then stop | Stop |
| Manual | Pause for user ack | Pause for user ack | Stop |

4. **Max tasks**: number (default: 10)

## Sequential Mode

```
For each task in pending (ordered by priority, then createdAt), up to max_tasks:

  === TASK LOOP START ===

  STEP 1: Start task
    workflow.startTask({
      workflow_id: workflowId, task_id: task.id
    })

  STEP 1b: Create worktree (if worktree mode is .masday/worktrees/)
    Generate slug from task.title (lowercase, hyphens, max 40 chars)

    Bash: git worktree add .masday/worktrees/{slug} -b task/{slug} HEAD
    Write .masday/worktrees/{slug}.json:
      { "taskId": task.id, "title": task.title, "branch": "task/{slug}", "status": "ACTIVE" }
    worktreePath = ".masday/worktrees/{slug}"

    If worktree mode is built-in project:
      worktreePath = "." (current directory)

  STEP 1c: Load context pack
    semantic-search.search_hybrid_context_pack({
      workflow_id: workflowId, plan_id: planId,
      task_id: task.id, cwd: process.cwd()
    })

  STEP 2: Dispatch executor agent
    Agent({
      subagent_type: "masday-executor",
      prompt: "Execute task '{task.title}' for workflow {workflowId}.
        TaskId: {task.id}
        Working directory: {worktreePath}
        Acceptance criteria: {task.acceptanceCriteria}
        Required context: {task.requiredContext}

        CRITICAL:
        - All file operations must happen in {worktreePath}
        - Write progress report to .masday/reports/progress-{slug}.md
        - List all files modified as evidence"
    })

  STEP 3: Save progress
    workflow.saveProgress({
      workflow_id: workflowId, task_id: task.id,
      agent_name: "masday-executor",
      progress_note: "<summary from executor>",
      evidence: ["<files modified>"]
    })

  STEP 4: Dispatch reviewer agent
    Agent({
      subagent_type: "masday-reviewer",
      prompt: "Review task '{task.title}' for workflow {workflowId}.
        TaskId: {task.id}
        Acceptance criteria: {task.acceptanceCriteria}
        Implementation evidence: <files modified>

        Return verdict as one of:
        - APPROVED: <notes>
        - REWORK_REQUIRED: <notes> | Gaps: <gap list>
        - BLOCKED: <reason>"
    })

  STEP 5: Submit review
    Extract decision from reviewer output.
    review.submit({
      workflow_id: workflowId, task_id: task.id,
      reviewer_agent: "masday-reviewer",
      decision: <extracted decision>,
      notes: <extracted notes>,
      gaps: <extracted gaps or []>
    })

  STEP 6: Handle review result

    IF APPROVED:
      STEP 6a: Dispatch verifier
        Agent({
          subagent_type: "masday-verifier",
          prompt: "Verify task '{task.title}' for workflow {workflowId}.
            TaskId: {task.id}
            Acceptance criteria: {task.acceptanceCriteria}
            Check: build passes, tests pass, no regressions.
            Return: PASS or FAIL with evidence."
        })

      STEP 6b: Handle verification
        IF PASS:
          Complete task:
            workflow.completeTask({ workflow_id: workflowId, task_id: task.id })

          If worktree mode is .masday/worktrees/:
            Auto-commit uncommitted changes in worktree
            Push and create PR:
              Bash: cd .masday/worktrees/{slug} && git add -A && git commit -m "feat: {task.title}"
              Bash: git push -u origin task/{slug}
              Bash: gh pr create --title "{task.title}" --body "Auto-generated by masday-autopilot"
            Update .masday/worktrees/{slug}.json: status "pr-created"
          Else (built-in project mode):
            No cleanup needed

          Print: "[DONE] Task N: {title} {PR url if worktree}"

        IF FAIL:
          Print: "[FAIL] Verification failed for: {title}"
          IF review_gate is Manual: pause for user
          ELSE: STOP — report failure details

    IF REWORK_REQUIRED:
      IF rework_attempts < max_rework:
        Print: "[REWORK] Task N: {title} — fixing..."
        Re-dispatch executor with fix instructions
        Increment rework_attempts
        Go to STEP 3
      ELSE:
        Print: "[STOP] Task {title} failed review after {attempts} attempts."
        STOP

    IF BLOCKED:
      Print: "[BLOCKED] Task N: {title} — {reason}"
      STOP

  STEP 7: Sync state
    local.sync({ cwd: process.cwd(), workflow_id: workflowId })

  STEP 8: Print progress bar

  === TASK LOOP END ===
```

## Parallel Mode

```
STEP 1: Group tasks into batches
  - Read all pending tasks from plan
  - Group into parallel batches (max 4 per batch)
  - Tasks modifying different files/modules → parallel-safe
  - Tasks modifying same files → sequential
  - If unsure → keep sequential (safe default)

STEP 2: For each batch:
  Start all tasks → Dispatch all agents simultaneously (in one message)
  → Save progress for each → Synthesize results → Review → Handle per-task
  → Sync state

STEP 3: Process any remaining sequential tasks using sequential mode.
```

## Safety Caps

| Cap | Value | Purpose |
|-----|-------|---------|
| Max tasks per run | 10 (configurable) | Prevent runaway execution |
| Max rework per task | 2 | Prevent infinite fix loops |
| Max parallel branches | 4 | Prevent resource exhaustion |
| Max total agent dispatches | 30 | Hard ceiling on agent calls |

## Error Handling

| Error | Action |
|-------|--------|
| Agent dispatch fails | Retry once, then skip task and continue |
| MCP tool call fails | Retry once, then STOP with error details |
| Build/test failure | Log in progress, mark task as needs-rework |
| Context insufficient | Auto-research via web + Context7, then retry |
| Context limit approaching | STOP after current task |

## Progress Output

After each task:

```
=== Autopilot: {done}/{total} tasks ===
  [DONE] Task 1: {title}
  [DONE] Task 2: {title}
  [>>>>] Task 3: {title} (CURRENT)
  [    ] Task 4: {title}
Mode: {Sequential|Parallel} | Gate: {Auto|Semi|Manual} | Worktree: {isolated|project}
```

When autopilot ends:

```
=== Autopilot Complete ===
Workflow: {name}
Tasks: {done} done, {blocked} blocked, {remaining} remaining
PRs: {count} created | Worktrees: {active} active, {completed} done
Artifacts: {count} files in .masday/reports/

Next: /masday-workflow-status for overview
```

## Never

- Never skip AskUserQuestion before starting execution
- Never skip policy validation at task boundaries
- Never ignore BLOCKED verdicts — always stop
- Never exceed max_tasks or max_rework limits
- Never remove worktrees with unmerged PRs

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
