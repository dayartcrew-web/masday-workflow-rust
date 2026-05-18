Merge parallel agent outputs into one coherent result.

## Purpose

Take multiple independent agent outputs (from parallel execution or research branches) and produce a single, contradiction-free deliverable that stays aligned with the active task.

## When to Use

- After parallel research completes (msd-parallel-research)
- After parallel implementation of independent modules
- When two agents produced conflicting outputs that need reconciliation
- When consolidating multiple review findings

## Steps

### 1. Identify Branches to Merge
```
Determine what needs synthesizing:
- Read workflow state to identify completed parallel tasks
- Call memory.recall_recent to find stored outputs
- List each branch with: agent name, task, output location

If no branches found:
  → Report: "No parallel outputs found to synthesize"
  → STOP
```

### 2. Collect Branch Outputs
```
For each branch:

1. Read the output file or memory entry
2. Note which agent produced it
3. Record the task each branch was working on
4. Identify the key contribution of each branch

Create a summary table:
| Branch | Agent | Task | Key Output |
|--------|-------|------|------------|
| 1 | msd-researcher | Research X | API format docs |
| 2 | msd-researcher | Research Y | Schema patterns |
| ... | ... | ... | ... |
```

### 3. Detect and Resolve Conflicts
```
Compare every pair of branches for:

Factual contradictions:
  → Both claim different facts about the same thing
  → Resolution: prefer authoritative source (official docs > blog > forum)

Implementation conflicts:
  → Both propose different approaches for the same functionality
  → Resolution: prefer simpler approach, document reasoning

Scope overlap:
  → Both cover overlapping ground
  → Resolution: merge overlapping parts, keep unique parts separate

Unresolvable conflicts:
  → Both have equal merit and contradict
  → Action: FLAG for human decision, do NOT silently pick one
```

### 4. Remove Duplication
```
Common duplication patterns to clean:

- Same utility function in multiple branches → keep one, note which
- Same research findings stored multiple times → merge citations
- Same code pattern repeated → extract to shared reference
- Same test covering same behavior → keep most comprehensive version

Document all removals in conflict resolution log.
```

### 5. Align to Active Task
```
Verify the merged result:

1. Does it address ALL acceptance criteria from the active task?
   → Check each criterion against merged output
   → If any not addressed → flag as gap

2. Does it stay within task scope?
   → No scope creep from individual branches
   → Remove any tangential findings (note them separately)

3. Is it internally consistent?
   → No contradictions remaining
   → All claims supported by evidence

4. Can it be verified by acceptance criteria?
   → Each criterion has corresponding evidence in merged output
```

### 6. Produce Unified Output
```
Write the synthesized result:

## Synthesis Report: {Task Title}

**Branches merged:** {count}
**Conflicts resolved:** {count}
**Duplications removed:** {count}

### Merged Result
{Unified output combining all branch contributions,
organized by topic/section matching the task structure}

### Conflict Resolution Log
| Conflict | Branch A | Branch B | Resolution |
|----------|----------|----------|------------|
| {issue} | {position} | {position} | {why this was chosen} |

### Evidence
- Files produced/modified: {list}
- Tests passing: {status}
- Build status: {status}

### Attribution
- Section 1: contributed by Branch 1 ({agent})
- Section 2: contributed by Branch 2 ({agent})
- Section 3: merged from Branch 1 + Branch 2
```

### 7. Save and Hand Off
```
Call workflow.saveProgress with:
{
  workflowId,
  taskId,
  agentName: "msd-synthesizer",
  progressNote: "Synthesized {N} branches. {conflicts} conflicts resolved.",
  evidence: ["synthesis report written", "conflicts resolved: {count}"]
}

Then hand off to msd-reviewer for validation.
```

## Anti-Patterns

- Never silently discard a branch's contribution — document why if removed
- Never introduce new information not present in any branch
- Never proceed if unresolvable conflicts exist — escalate to orchestrator
- Never complete tasks — you only merge outputs, completion is separate
