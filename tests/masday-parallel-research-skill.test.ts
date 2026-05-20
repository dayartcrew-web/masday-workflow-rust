import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const skillPath = ".claude/skills/masday-parallel-research/SKILL.md";

describe("masday-parallel-research skill", () => {
  it("defines the parallel research orchestration flow", () => {
    expect(existsSync(skillPath)).toBe(true);
    const content = readFileSync(skillPath, "utf8");
    expect(content).toContain("workflow.createParallelBranches");
    expect(content).toContain("masday-researcher");
    expect(content).toContain("masday-synthesizer");
    expect(content).toContain("local.save_artifact");
  });
});
