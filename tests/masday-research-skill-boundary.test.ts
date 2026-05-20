import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-research skill boundaries", () => {
  it("directs multi-branch research to masday-parallel-research", () => {
    const content = readFileSync(".claude/skills/masday-research/SKILL.md", "utf8");
    expect(content).toContain("Use masday-parallel-research");
    expect(content).toContain("2+ independent research questions");
  });
});
