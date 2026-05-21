import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-researcher branch contract", () => {
  it("documents branch-only memory persistence and synthesis metadata", () => {
    const content = readFileSync(".claude/agents/masday-researcher.md", "utf8");
    expect(content).toContain("branch worker");
    expect(content).toContain("memory_store_research");
    expect(content).toContain("Do not write local artifacts");
    expect(content).toContain("branch_scope");
    expect(content).toContain("confidence");
    expect(content).toContain("gaps");
  });
});
