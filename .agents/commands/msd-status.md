Show the current workflow state, task progress, and session health.

## Purpose

Quick diagnostic that shows where things stand without starting or modifying any work. Like `git status` but for workflows.

## Steps

### 1. Check Active Workflow
```
Call workflow.getActive

If no active workflow:
  → Report: "No active workflow. Use /msd-start-work to begin."
  → STOP
```

### 2. Get Workflow Overview
```
Call workflow.getPlan → list all tasks and statuses
Call workflow.getCurrentTask → active task details

Display summary:
┌─────────────────────────────────────────────┐
│ Workflow: {title}                            │
│ Status: {status}                             │
│ ID: {workflowId}                             │
├─────────────────────────────────────────────┤
│ Tasks: {total} total, {completed} done       │
│ Progress: [{bar}] {percentage}%              │
│ Current: {task title} ({status})             │
└─────────────────────────────────────────────┘
```

### 3. Show Task List
```
For each task in the plan:

| # | Task | Status | Agent | Depends On |
|---|------|--------|-------|------------|
| 1 | {title} | DONE | msd-executor | — |
| 2 | {title} | IN_PROGRESS | msd-executor | 1 |
| 3 | {title} | PENDING | — | 2 |
| ... |

Status indicators:
  DONE = completed and verified
  IN_PROGRESS = currently being worked on
  PENDING = not started
  BLOCKED = waiting on dependency or issue
```

### 4. Check Review Status
```
Call review.get_latest for the current task

If review exists:
  → Show: "Last review: {decision} by {reviewer}"
  → If APPROVED → "Ready for verification"
  → If REWORK_REQUIRED → "Needs fixes before proceeding"
  → If BLOCKED → "Blocked: {blocker description}"

If no review:
  → Show: "No review submitted yet"
```

### 5. Check Session Health
```
Call session.get_state → check loaded flags

Report session state:
  Context loaded: {yes/no}
  Review completed: {yes/no}
  Evidence collected: {yes/no}

Quick health checks:
  pnpm build → {pass/fail}
  pnpm test → {pass/fail, X tests}
  pnpm lint → {pass/fail}
```

### 6. Check Memory/Research
```
Call memory.recall_recent with { workflowId, limit: 3 }

Show recent activity:
  - {timestamp}: {agent} — {progress note}
  - {timestamp}: {agent} — {progress note}
  ...
```

## Output Format

```
═══════════════════════════════════════════
  MSD Workflow Status
═══════════════════════════════════════════

Workflow: {title}
ID: {workflowId}
Status: {status}

Progress: [{visual bar}] {percentage}%
Tasks: {done}/{total} completed

Current Task: {title}
  Status: {status}
  Review: {decision or "pending"}
  Context: {loaded/not loaded}

Recent Activity:
  • {latest progress note}
  • {previous progress note}

Health:
  Build: {pass/fail}
  Tests: {pass/fail}
  Lint: {pass/fail}

Next Step: {recommended action}
═══════════════════════════════════════════
```

## This Command Does NOT

- Start any workflow
- Modify any state
- Run any agent
- Change any files

It is purely a READ-ONLY diagnostic.
