Auto-pilot task execution — run all pending tasks sequentially or in parallel.

## Purpose

Execute all pending tasks in the active workflow automatically, without manual intervention between tasks. The orchestrator becomes a loop controller: start task → dispatch agent → review → complete → next task.

## Pre-Conditions (STOP if any fail)

```
Step 0a — Init .msd/:
  mcp__workflow-orchestrator__local_init({ cwd: process.cwd() })

Step 0b — Get active workflow:
  mcp__workflow-orchestrator__workflow_get_active({ cwd: process.cwd() })
  If none: STOP — "No active workflow. Run /msd-start-work first."
  Record: workflowId

Step 0c — Get plan:
  mcp__workflow-orchestrator__workflow_get_plan({ workflow_id: workflowId })
  If no plan: STOP — "No plan found. Run /msd-plan first."
  Record: planId, tasks[]

Step 0d — Count pending:
  pending = tasks where status is "todo"
  If pending.length === 0: STOP — "All tasks completed."
```

## Configuration (AskUserQuestion)

Ask the user:

1. **Mode**: Sequential or Parallel?
2. **Worktree**: Yes (per-task isolation) or No (work on main)? (default: Yes)
3. **Review gate**: Auto / Semi-auto / Manual? (default: Semi-auto)
4. **Max tasks**: number (default: 10)

| Review Gate | APPROVED | REWORK_REQUIRED | BLOCKED |
|-------------|----------|-----------------|---------|
| Auto | Auto-complete | Auto-fix up to 2x | Stop |
| Semi-auto | Auto-complete | Auto-fix once, then stop | Stop |
| Manual | Pause for user ack | Pause for user ack | Stop |

## Sequential Mode

```
For each task in pending (ordered by createdAt), up to max_tasks:

  === TASK LOOP START ===

  STEP 1: Start task
    mcp__workflow-orchestrator__workflow_start_task({
      workflow_id: workflowId,
      task_id: task.id
    })

  STEP 1b: Create worktree (if worktree enabled)
    If worktree mode is Yes:
      Generate slug from task.title
      Bash: git worktree add .msd/worktrees/{slug} -b task/{slug} HEAD
      Write .msd/worktrees/{slug}.json with task info
      worktreePath = ".msd/worktrees/{slug}"
    Else:
      worktreePath = "." (current directory)

  STEP 2: Dispatch executor
    Agent({
      subagent_type: "msd-executor",
      prompt: "Execute task '{task.title}' for workflow {workflowId}.
        TaskId: {task.id}
        Working directory: {worktreePath}
        Acceptance criteria: {task.acceptanceCriteria}
        Required context: {task.requiredContext}

        CRITICAL:
        - All file operations must happen in {worktreePath}
        - Write implementation report to .msd/reports/progress-{slug}.md using Write tool
        - List all files modified as evidence"
    })

  STEP 3: Save progress
    mcp__workflow-orchestrator__workflow_save_progress({
      workflow_id: workflowId,
      task_id: task.id,
      agent_name: "msd-executor",
      progress_note: "<summary from executor output>",
      evidence: ["<files modified>"],
      status_after: "reviewing"
    })

  STEP 4: Dispatch reviewer
    Agent({
      subagent_type: "msd-reviewer",
      prompt: "Review task '{task.title}' for workflow {workflowId}.
        TaskId: {task.id}
        Acceptance criteria: {task.acceptanceCriteria}
        Implementation evidence: <files modified>

        Review the implementation against acceptance criteria.
        Return your verdict as one of:
        - APPROVED: <notes>
        - REWORK_REQUIRED: <notes> | Gaps: <gap list>
        - BLOCKED: <reason>"
    })

  STEP 5: Parse review and submit
    Extract decision from reviewer output (APPROVED / REWORK_REQUIRED / BLOCKED).
    mcp__workflow-orchestrator__review_submit({
      workflow_id: workflowId,
      task_id: task.id,
      reviewer_agent: "msd-reviewer",
      decision: <extracted decision>,
      notes: <extracted notes>,
      gaps: <extracted gaps or []>
    })

  STEP 6: Handle review result

    IF APPROVED:
      STEP 6a: Dispatch verifier
        Agent({
          subagent_type: "msd-verifier",
          prompt: "Verify task '{task.title}' for workflow {workflowId}.
            TaskId: {task.id}
            Acceptance criteria: {task.acceptanceCriteria}
            Check: build passes, tests pass, no regressions.
            Return: PASS or FAIL with evidence."
        })

      STEP 6b: Handle verification
        IF PASS:
          STEP 6b-1: Complete task in workflow
            mcp__workflow-orchestrator__workflow_complete_task({
              workflow_id: workflowId, task_id: task.id
            })

          STEP 6b-2: Finish worktree (if worktree enabled)
            If worktree mode is Yes:
              Auto-commit any uncommitted changes in worktree
              Verify tests pass from worktree directory
              Auto-create PR (Option 2 from /msd-worktree done):
                Bash: cd {worktreePath} && git push -u origin task/{slug}
                Bash: gh pr create --title "{task.title}" --body "..."
              Update .msd/worktrees/{slug}.json: status "pr-created"
              Keep worktree alive (don't remove — PR still open)
            Else:
              No worktree cleanup needed

          Print: "[DONE] Task N: {title} {PR url if worktree}"
        IF FAIL:
          Print: "[FAIL] Verification failed for: {title}"
          IF review_gate is Manual: pause for user
          ELSE: STOP — report failure details

    IF REWORK_REQUIRED:
      IF rework_attempts < max_rework (2 for Auto, 1 for Semi-auto):
        Print: "[REWORK] Task N: {title} — fixing..."
        Re-dispatch msd-executor with fix instructions:
          "REWORK task '{title}'. Previous review notes: {notes}.
           Required fixes: {gaps}. Address these specific issues."
        Increment rework_attempts
        Go to STEP 3 (save progress after rework)
      ELSE:
        Print: "[STOP] Task {title} failed review after {attempts} attempts."
        STOP

    IF BLOCKED:
      Print: "[BLOCKED] Task N: {title} — {reason}"
      STOP

  STEP 7: Sync state
    mcp__workflow-orchestrator__local_sync({
      cwd: process.cwd(), workflow_id: workflowId
    })

  STEP 8: Print progress
    Print progress bar showing all tasks and their statuses.

  === TASK LOOP END ===

After loop ends (all done or stopped):
  Print final summary:
  "Autopilot complete: {done}/{total} tasks done, {blocked} blocked, {remaining} remaining"
```

## Parallel Mode

```
STEP 1: Group tasks into batches
  - Read all pending tasks from plan
  - Group consecutive tasks into parallel batches (max 4 per batch)
  - Tasks that depend on previous tasks stay sequential
  - Independent tasks go into the same batch

  Grouping heuristic:
    - Tasks modifying different files/modules → parallel-safe
    - Tasks modifying the same files → sequential
    - Testing tasks → parallel-safe with other testing tasks
    - If unsure → keep sequential (safe default)

STEP 2: For each batch:

  STEP 2a: Start all tasks in batch
    For each task: workflow.startTask({ workflowId, task.id })

  STEP 2b: Dispatch all agents SIMULTANEOUSLY
    Agent({ subagent_type: "msd-executor", prompt: "Execute task 1..." })
    Agent({ subagent_type: "msd-executor", prompt: "Execute task 2..." })
    Agent({ subagent_type: "msd-executor", prompt: "Execute task 3..." })
    (All in one message — Claude runs them in parallel)

  STEP 2c: Save progress for each
    For each completed agent result:
      workflow.saveProgress({ workflowId, taskId, agentName: "msd-executor", ... })

  STEP 2d: Synthesize results
    Agent({
      subagent_type: "msd-synthesizer",
      prompt: "Merge {N} parallel branch outputs for tasks: {titles}.
        Resolve any conflicts. Produce unified result."
    })

  STEP 2e: Review synthesized output
    Agent({
      subagent_type: "msd-reviewer",
      prompt: "Review {N} parallel tasks: {titles}.
        Check each against its acceptance criteria.
        Return verdict per task."
    })

  STEP 2f: Handle review results per task
    For each task in batch:
      - If APPROVED: workflow.completeTask
      - If REWORK: mark for re-dispatch
      - If BLOCKED: skip and report

    If any tasks need rework and attempts < max:
      Re-dispatch ONLY failed tasks (not entire batch)
      Go to STEP 2c

  STEP 2g: Sync state
    local.sync({ cwd, workflow_id })

STEP 3: Process any remaining sequential tasks
  Fall back to sequential loop for remaining tasks.

STEP 4: Print final summary
```

## Safety Caps

| Cap | Value | Purpose |
|-----|-------|---------|
| Max tasks per run | 10 (configurable) | Prevent runaway execution |
| Max rework per task | 2 | Prevent infinite fix loops |
| Max parallel branches | 4 | Prevent resource exhaustion |
| Max total dispatches | 30 | Hard ceiling on agent calls |

## Error Handling

| Error | Action |
|-------|--------|
| Agent dispatch fails | Retry once, then skip task and continue |
| MCP tool call fails | Retry once, then STOP with error details |
| Build/test failure during execution | Log in progress, mark task as needs-rework |
| Context insufficient for task | Auto-research via web search + Context7, then retry |
| Session context limit approaching | STOP after current task, suggest /msd-continue |

## Progress Output

After each task completion, print:

```
=== Autopilot: {done}/{total} tasks ===
  [DONE] Task 1: {title}
  [DONE] Task 2: {title}
  [>>>>] Task 3: {title} (CURRENT)
  [    ] Task 4: {title}
  ...
Mode: {Sequential|Parallel} | Gate: {Auto|Semi|Manual} | Worktree: {Yes|No}
```

When autopilot ends:

```
=== Autopilot Complete ===
Workflow: {name}
Tasks: {done} done, {blocked} blocked, {remaining} remaining
PRs: {count} created | Worktrees: {active} active, {completed} done
Artifacts: {count} files in .msd/reports/

{if worktree enabled: "Run /msd-worktree list to see open PRs and worktrees."}
{if blocked or remaining: "Run /msd-autopilot to resume, or /msd-continue for manual control."}
{if all done: "All tasks complete! Run /msd-status for final overview."}
```
