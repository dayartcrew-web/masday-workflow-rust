import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("masday-synthesizer research output", () => {
  it("documents final-only local artifact output for parallel research", () => {
    const content = readFileSync(".claude/agents/masday-synthesizer.md", "utf8");
    expect(content).toContain("research synthesis");
    expect(content).toContain("local.save_artifact");
    expect(content).toContain("final-only local artifact");
    expect(content).toContain("memory.recall_document_by_type");
  });
});
