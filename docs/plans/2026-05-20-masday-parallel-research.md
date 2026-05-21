# Masday Parallel Research Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a new `masday-parallel-research` skill that ports the branch-dispatch/synthesis pattern from `msd-parallel-research` into the Masday workflow system, while keeping tool names on the Masday MCP pattern and writing only the final synthesized report locally.

**Architecture:** Keep `masday-research` as the simple single-entry research skill. Add a new orchestration skill that splits one research request into independent branches, stores branch outputs in `memory_store_research`, synthesizes them with `masday-synthesizer`, and writes one final local artifact via `local_save_artifact`. Avoid MCP runtime changes unless a real gap is discovered during implementation, because the required primitives already exist in `apps/agent-runner/src/runtime/mcp.ts`.

**Tech Stack:** Claude skills in `.agents/skills`, agent prompts in `.agents/agents`, Masday MCP tools (`workflow.*`, `memory.*`, `local.*`), TypeScript/Vitest for validation, Markdown artifact output.

---

### Task 1: Add the new orchestrator skill scaffold

**Files:**
- Create: `.agents/skills/masday-parallel-research/SKILL.md`
- Reference: `.agents/skills/masday-research/SKILL.md`
- Reference: `.agents/skills/masday-parallel-execution/SKILL.md`
- Reference: `.claude/commands/msd-parallel-research.md`

**Step 1: Write the failing test**

Create `tests/masday-parallel-research-skill.test.ts` with assertions that the new skill file exists and contains the required orchestration phases:

```ts
import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const skillPath = ".agents/skills/masday-parallel-research/SKILL.md";

describe("masday-parallel-research skill", () => {
  it("defines the parallel research orchestration flow", () => {
    expect(existsSync(skillPath)).toBe(true);
    const content = readFileSync(skillPath, "utf8");
    expect(content).toContain("workflow_createParallelBranches");
    expect(content).toContain("masday-researcher");
    expect(content).toContain("masday-synthesizer");
    expect(content).toContain("local_save_artifact");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/masday-parallel-research-skill.test.ts`

Expected: FAIL because `.agents/skills/masday-parallel-research/SKILL.md` does not exist yet.

**Step 3: Write minimal implementation**

Create `.agents/skills/masday-parallel-research/SKILL.md` with these sections:
- frontmatter: `name`, `description`, `allowed-tools`
- entry rule: use only for 2+ independent research questions
- workflow context step: `workflow_getActive`, `workflow_getCurrentTask`, `workflow_getPlan`
- branch split step
- branch creation step using `workflow_createParallelBranches`
- branch dispatch step using `masday-researcher`
- synthesis step using `masday-synthesizer`
- final artifact write step using `local_save_artifact`
- mandatory review pipeline

Use this frontmatter skeleton:

```md
---
name: masday-parallel-research
description: >
  Orchestrates multi-branch research in parallel using masday workflow tools,
  stores branch results in memory, synthesizes the outputs, and saves one final
  report locally.
allowed-tools:
  - Agent
  - workflow_getActive
  - workflow_getCurrentTask
  - workflow_getPlan
  - workflow_createParallelBranches
  - workflow_listParallelBranches
  - workflow_completeParallelBranch
  - workflow_saveProgress
  - memory_recall_documents
  - memory_recall_document_by_type
  - memory_store
  - local_save_artifact
---
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-parallel-research-skill.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-parallel-research-skill.test.ts .agents/skills/masday-parallel-research/SKILL.md
git commit -m "feat: add masday parallel research skill"
```

---

### Task 2: Define the branch output contract for `masday-researcher`

**Files:**
- Modify: `.agents/agents/masday-researcher.md`
- Test: `tests/masday-researcher-contract.test.ts`

**Step 1: Write the failing test**

Create `tests/masday-researcher-contract.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-researcher branch contract", () => {
  it("documents branch-only memory persistence and synthesis metadata", () => {
    const content = readFileSync(".agents/agents/masday-researcher.md", "utf8");
    expect(content).toContain("branch worker");
    expect(content).toContain("memory_store_research");
    expect(content).toContain("Do not write local artifacts");
    expect(content).toContain("branch_scope");
    expect(content).toContain("confidence");
    expect(content).toContain("gaps");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/masday-researcher-contract.test.ts`

Expected: FAIL because the current agent prompt does not define branch-worker output rules explicitly.

**Step 3: Write minimal implementation**

Update `.agents/agents/masday-researcher.md` in the persistence/reporting section to add a branch-worker contract. Document that when the agent is dispatched by `masday-parallel-research`, it must:
- store branch output through `memory_store_research`
- keep the result scoped to branch research only
- not call `local_save_artifact`
- include this structured payload in the stored content:

```md
## Branch Output Contract
- branch_key: stable branch identifier
- branch_scope: the exact research question this branch answered
- summary: one-paragraph answer
- findings: bullet list of concrete findings
- sources: URLs and codebase references
- confidence: high | medium | low
- gaps: unresolved questions for synthesis
```

Also add one line that branch workers must keep the content synthesis-friendly and non-duplicative.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-researcher-contract.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-researcher-contract.test.ts .agents/agents/masday-researcher.md
git commit -m "feat: define masday researcher branch contract"
```

---

### Task 3: Teach `masday-synthesizer` to produce the final research artifact

**Files:**
- Modify: `.agents/agents/masday-synthesizer.md`
- Test: `tests/masday-synthesizer-research-output.test.ts`

**Step 1: Write the failing test**

Create `tests/masday-synthesizer-research-output.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-synthesizer research output", () => {
  it("documents final-only local artifact output for parallel research", () => {
    const content = readFileSync(".agents/agents/masday-synthesizer.md", "utf8");
    expect(content).toContain("research synthesis");
    expect(content).toContain("local_save_artifact");
    expect(content).toContain("final-only local artifact");
    expect(content).toContain("memory_recall_document_by_type");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/masday-synthesizer-research-output.test.ts`

Expected: FAIL because the current synthesizer prompt only mentions `Write(...)` output and does not define the final-only research artifact behavior.

**Step 3: Write minimal implementation**

Update `.agents/agents/masday-synthesizer.md` so the research-synthesis path explicitly says:
- collect branch research documents by workflow and type
- merge/dedupe/resolve contradictions
- store synthesis summary in `memory_store`
- write exactly one final local artifact via `local_save_artifact`
- do not require branch-level local files

Add a concrete example call:

```md
local_save_artifact({
  cwd: process.cwd(),
  category: "reports",
  filename: "2026-05-20-topic-research-synthesis.md",
  content: "# Research Synthesis\n\n..."
})
```

Keep the existing general synthesis logic intact; only extend it with a specific research branch flow.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-synthesizer-research-output.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-synthesizer-research-output.test.ts .agents/agents/masday-synthesizer.md
git commit -m "feat: add final research artifact synthesis flow"
```

---

### Task 4: Align the existing `masday-research` skill with the new split of responsibilities

**Files:**
- Modify: `.agents/skills/masday-research/SKILL.md`
- Test: `tests/masday-research-skill-boundary.test.ts`

**Step 1: Write the failing test**

Create `tests/masday-research-skill-boundary.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-research skill boundaries", () => {
  it("directs multi-branch research to masday-parallel-research", () => {
    const content = readFileSync(".agents/skills/masday-research/SKILL.md", "utf8");
    expect(content).toContain("Use masday-parallel-research");
    expect(content).toContain("2+ independent research questions");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/masday-research-skill-boundary.test.ts`

Expected: FAIL because the current skill does not defer large research jobs to the new orchestrator.

**Step 3: Write minimal implementation**

Update `.agents/skills/masday-research/SKILL.md` near the top and in the “Never”/usage sections:
- keep it as the default simple research skill
- add a routing rule: if the question naturally decomposes into 2+ independent research branches, use `masday-parallel-research` instead
- do not add local artifact writing here

Suggested line:

```md
If the task requires 2+ independent research questions with separate branch outputs, use `masday-parallel-research` instead of this skill.
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-research-skill-boundary.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-research-skill-boundary.test.ts .agents/skills/masday-research/SKILL.md
git commit -m "refactor: separate simple and parallel research skills"
```

---

### Task 5: Verify the Masday MCP tool assumptions against the live runtime contract

**Files:**
- Modify: `apps/agent-runner/src/runtime/mcp.ts` (only if a real gap is found)
- Test: `tests/masday-mcp-parallel-research-contract.test.ts`

**Step 1: Write the failing test**

Create `tests/masday-mcp-parallel-research-contract.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const runtime = readFileSync("apps/agent-runner/src/runtime/mcp.ts", "utf8");

describe("masday MCP runtime contract", () => {
  it("exposes the tools required by parallel research orchestration", () => {
    expect(runtime).toContain('server.registerTool("workflow_createParallelBranches"');
    expect(runtime).toContain('server.registerTool("workflow_completeParallelBranch"');
    expect(runtime).toContain('server.registerTool("workflow_listParallelBranches"');
    expect(runtime).toContain('server.registerTool("memory_store_research"');
    expect(runtime).toContain('server.registerTool("local_save_artifact"');
  });
});
```

**Step 2: Run test to verify the current status**

Run: `pnpm vitest tests/masday-mcp-parallel-research-contract.test.ts`

Expected: PASS with the current codebase. If it fails, inspect `apps/agent-runner/src/runtime/mcp.ts` before changing anything.

**Step 3: Write minimal implementation only if needed**

Only modify `apps/agent-runner/src/runtime/mcp.ts` if the test reveals a real mismatch. Likely no code change is needed because the relevant tools already exist around:
- `workflow_createParallelBranches`
- `workflow_completeParallelBranch`
- `workflow_listParallelBranches`
- `memory_store_research`
- `local_save_artifact`

If a gap is found, fix only that gap. Do not refactor unrelated MCP tool registration.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-mcp-parallel-research-contract.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-mcp-parallel-research-contract.test.ts apps/agent-runner/src/runtime/mcp.ts
git commit -m "test: verify masday MCP parallel research contract"
```

---

### Task 6: End-to-end verification of the documentation contract

**Files:**
- Test: `tests/masday-parallel-research-e2e-docs.test.ts`

**Step 1: Write the failing test**

Create `tests/masday-parallel-research-e2e-docs.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const skill = readFileSync(".agents/skills/masday-parallel-research/SKILL.md", "utf8");
const researcher = readFileSync(".agents/agents/masday-researcher.md", "utf8");
const synthesizer = readFileSync(".agents/agents/masday-synthesizer.md", "utf8");

describe("parallel research documentation integration", () => {
  it("aligns orchestrator, branch worker, and synthesizer responsibilities", () => {
    expect(skill).toContain("memory_store_research");
    expect(skill).toContain("local_save_artifact");
    expect(researcher).toContain("Do not write local artifacts");
    expect(synthesizer).toContain("final-only local artifact");
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/masday-parallel-research-e2e-docs.test.ts`

Expected: FAIL until all prompt files are aligned.

**Step 3: Write minimal implementation**

Adjust wording in the three Markdown files until the contract is consistent:
- orchestrator creates and manages branches
- researcher stores branch findings in memory only
- synthesizer writes the final local report only

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/masday-parallel-research-e2e-docs.test.ts`

Expected: PASS.

**Step 5: Commit**

```bash
git add tests/masday-parallel-research-e2e-docs.test.ts \
  .agents/skills/masday-parallel-research/SKILL.md \
  .agents/agents/masday-researcher.md \
  .agents/agents/masday-synthesizer.md
git commit -m "test: verify parallel research prompt integration"
```

---

### Task 7: Final validation

**Files:**
- Review: `.agents/skills/masday-parallel-research/SKILL.md`
- Review: `.agents/agents/masday-researcher.md`
- Review: `.agents/agents/masday-synthesizer.md`
- Review: `.agents/skills/masday-research/SKILL.md`
- Review: `tests/*.test.ts`

**Step 1: Run the targeted tests**

Run:

```bash
pnpm vitest tests/masday-parallel-research-skill.test.ts \
  tests/masday-researcher-contract.test.ts \
  tests/masday-synthesizer-research-output.test.ts \
  tests/masday-research-skill-boundary.test.ts \
  tests/masday-mcp-parallel-research-contract.test.ts \
  tests/masday-parallel-research-e2e-docs.test.ts
```

Expected: All PASS.

**Step 2: Run full project tests that are cheap enough to run**

Run: `pnpm test`

Expected: PASS, or if unrelated failures already exist, capture them explicitly and do not hide them.

**Step 3: Review changes**

Run:

```bash
git diff -- .agents/agents/masday-researcher.md \
  .agents/agents/masday-synthesizer.md \
  .agents/skills/masday-research/SKILL.md \
  .agents/skills/masday-parallel-research/SKILL.md \
  tests/
```

Expected: Only the planned files changed.

**Step 4: Commit final integration work**

```bash
git add .agents/agents/masday-researcher.md \
  .agents/agents/masday-synthesizer.md \
  .agents/skills/masday-research/SKILL.md \
  .agents/skills/masday-parallel-research/SKILL.md \
  tests/ \
  docs/plans/2026-05-20-masday-parallel-research.md
git commit -m "feat: add masday parallel research orchestration"
```

---

## Data Contract Summary

### Branch worker (`masday-researcher`) output
Store via `memory_store_research`.

```md
branch_key: string
branch_scope: string
summary: string
findings:
  - string
sources:
  - string
confidence: high | medium | low
gaps:
  - string
```

### Final synthesizer output
Store via `memory_store`, then write once via `local_save_artifact`.

Filename format:
- `YYYY-MM-DD-<topic-slug>-research-synthesis.md`

Category:
- `reports`

Minimum content:
- title
- topic summary
- merged findings
- codebase context
- ranked recommendations
- open gaps
- sources

## Notes

- Keep all tool names on the Masday MCP pattern. Do not introduce `msd.*` naming.
- Do not add branch-level local files.
- Do not modify `apps/agent-runner/src/runtime/mcp.ts` unless the contract test proves a real runtime gap.
- Prefer additive prompt updates over broad rewrites.
- Keep the new skill focused on orchestration, not on doing research itself.
