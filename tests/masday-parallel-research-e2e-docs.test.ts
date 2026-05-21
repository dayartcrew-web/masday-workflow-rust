import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const skill = readFileSync(".claude/skills/masday-parallel-research/SKILL.md", "utf8");
const researcher = readFileSync(".claude/agents/masday-researcher.md", "utf8");
const synthesizer = readFileSync(".claude/agents/masday-synthesizer.md", "utf8");

describe("parallel research documentation integration", () => {
  it("aligns orchestrator, branch worker, and synthesizer responsibilities", () => {
    expect(skill).toContain("memory_store_research");
    expect(skill).toContain("local_save_artifact");
    expect(researcher).toContain("Do not write local artifacts");
    expect(synthesizer).toContain("final-only local artifact");
  });
});
