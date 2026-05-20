import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import {
  createAgent, validateAgentName, buildAgentMarkdown,
  createSkill, validateSkillName, buildSkillMarkdown,
} from "@mcp-rebuild/shared-utils";

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "masday-test-"));
});
afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

// --- createAgent ---

describe("validateAgentName", () => {
  it("accepts valid kebab-case names", () => {
    expect(validateAgentName("security-reviewer")).toBeNull();
    expect(validateAgentName("api-designer")).toBeNull();
    expect(validateAgentName("a1")).toBeNull();
  });

  it("rejects invalid names", () => {
    expect(validateAgentName("ab")).toBeNull();
    expect(validateAgentName("A")).not.toBeNull();
    expect(validateAgentName("has_space")).not.toBeNull();
    expect(validateAgentName("1start")).not.toBeNull();
    expect(validateAgentName("a")).not.toBeNull();
  });
});

describe("buildAgentMarkdown", () => {
  it("generates minimal frontmatter", () => {
    const md = buildAgentMarkdown({
      projectRoot: tmpDir, name: "test-agent", role: "tester",
      description: "A test agent", instructions: "Do the thing.",
    });
    expect(md).toContain("name: test-agent");
    expect(md).toContain("role: tester");
    expect(md).toContain("description: A test agent");
    expect(md).toContain("Do the thing.");
  });

  it("includes optional model and tools", () => {
    const md = buildAgentMarkdown({
      projectRoot: tmpDir, name: "test-agent", role: "tester",
      description: "desc", instructions: "inst",
      model: "sonnet", tools: ["filesystem.read", "filesystem.write"],
    });
    expect(md).toContain("model: sonnet");
    expect(md).toContain("  - filesystem.read");
    expect(md).toContain("  - filesystem.write");
  });
});

describe("createAgent", () => {
  it("creates agent file on disk", () => {
    const result = createAgent({
      projectRoot: tmpDir, name: "my-agent", role: "dev",
      description: "dev agent", instructions: "Write code.",
    });
    expect(result.ok).toBe(true);
    expect(result.name).toBe("my-agent");
    expect(result.alreadyExists).toBe(false);
    expect(existsSync(result.filePath)).toBe(true);

    const content = readFileSync(result.filePath, "utf-8");
    expect(content).toContain("name: my-agent");
    expect(content).toContain("Write code.");
  });

  it("detects existing file", () => {
    const r1 = createAgent({
      projectRoot: tmpDir, name: "dup", role: "r",
      description: "d", instructions: "i",
    });
    const r2 = createAgent({
      projectRoot: tmpDir, name: "dup", role: "r",
      description: "d", instructions: "i",
    });
    expect(r1.alreadyExists).toBe(false);
    expect(r2.alreadyExists).toBe(true);
  });

  it("throws on invalid name", () => {
    expect(() => createAgent({
      projectRoot: tmpDir, name: "INVALID", role: "r",
      description: "d", instructions: "i",
    })).toThrow();
  });
});

// --- createSkill ---

describe("validateSkillName", () => {
  it("accepts valid kebab-case names", () => {
    expect(validateSkillName("masday-deploy-check")).toBeNull();
    expect(validateSkillName("my-skill")).toBeNull();
  });

  it("rejects invalid names", () => {
    expect(validateSkillName("UPPER")).not.toBeNull();
    expect(validateSkillName("has space")).not.toBeNull();
    expect(validateSkillName("x")).not.toBeNull();
  });
});

describe("buildSkillMarkdown", () => {
  it("generates frontmatter with steps", () => {
    const md = buildSkillMarkdown({
      projectRoot: tmpDir, name: "test-skill",
      description: "A test skill", trigger: "test trigger",
      steps: ["Step one", "Step two"],
    });
    expect(md).toContain("name: test-skill");
    expect(md).toContain("A test skill");
    expect(md).toContain("test trigger");
    expect(md).toContain("1. Step one");
    expect(md).toContain("2. Step two");
  });

  it("includes allowed-tools when provided", () => {
    const md = buildSkillMarkdown({
      projectRoot: tmpDir, name: "test-skill",
      description: "desc", trigger: "trig",
      steps: ["Step"], allowedTools: ["filesystem.read", "workflow.create"],
    });
    expect(md).toContain("  - filesystem.read");
    expect(md).toContain("  - workflow.create");
  });
});

describe("createSkill", () => {
  it("creates skill directory and SKILL.md", () => {
    const result = createSkill({
      projectRoot: tmpDir, name: "my-skill",
      description: "test skill", trigger: "trigger",
      steps: ["Do A", "Do B"],
    });
    expect(result.ok).toBe(true);
    expect(result.name).toBe("my-skill");
    expect(result.alreadyExists).toBe(false);
    expect(existsSync(result.filePath)).toBe(true);
    expect(result.filePath).toContain("my-skill");
    expect(result.filePath).toContain("SKILL.md");

    const content = readFileSync(result.filePath, "utf-8");
    expect(content).toContain("name: my-skill");
    expect(content).toContain("1. Do A");
    expect(content).toContain("2. Do B");
  });

  it("detects existing skill", () => {
    createSkill({
      projectRoot: tmpDir, name: "dup-skill",
      description: "d", trigger: "t", steps: ["s"],
    });
    const r2 = createSkill({
      projectRoot: tmpDir, name: "dup-skill",
      description: "d", trigger: "t", steps: ["s"],
    });
    expect(r2.alreadyExists).toBe(true);
  });

  it("throws on invalid name", () => {
    expect(() => createSkill({
      projectRoot: tmpDir, name: "BAD",
      description: "d", trigger: "t", steps: ["s"],
    })).toThrow();
  });
});
