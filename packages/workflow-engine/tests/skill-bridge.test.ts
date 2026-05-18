import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { MCPToolBridge } from "../src/mcp-tool-bridge.js";
import { parseSkillFrontmatter, SkillLoader } from "../src/skill-loader.js";
import { LiveSkillRegistry } from "../src/live-skill-registry.js";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

describe("MCPToolBridge", () => {
  let bridge: MCPToolBridge;

  beforeEach(() => {
    bridge = new MCPToolBridge();
  });

  it("registers and executes a tool handler", async () => {
    bridge.register("test.tool", async (input) => ({ echoed: input }));
    const result = await bridge.execute("test.tool", { name: "hello" });
    expect(result).toEqual({ echoed: { name: "hello" } });
  });

  it("has() returns true for registered tools", () => {
    bridge.register("workflow.create", async () => null);
    expect(bridge.has("workflow.create")).toBe(true);
    expect(bridge.has("nonexistent")).toBe(false);
  });

  it("execute throws for unregistered tools", async () => {
    await expect(bridge.execute("missing", {})).rejects.toThrow("Tool not found: missing");
  });

  it("getAll returns all registered tool names", () => {
    bridge.register("a", async () => null);
    bridge.register("b", async () => null);
    expect(bridge.getAll()).toEqual(["a", "b"]);
  });

  it("size returns correct count", () => {
    expect(bridge.size).toBe(0);
    bridge.register("x", async () => null);
    expect(bridge.size).toBe(1);
  });
});

describe("parseSkillFrontmatter", () => {
  it("parses frontmatter with allowed-tools array", () => {
    const content = `---
name: test-skill
description: A test skill
allowed-tools:
  - workflow.create
  - memory.store
  - Bash
---

# Steps
1. Do something
2. Do another thing`;

    const { meta, body } = parseSkillFrontmatter(content);
    expect(meta.name).toBe("test-skill");
    expect(meta.description).toBe("A test skill");
    expect(meta["allowed-tools"]).toEqual(["workflow.create", "memory.store", "Bash"]);
    expect(body).toContain("# Steps");
    expect(body).toContain("Do something");
  });

  it("returns empty meta for content without frontmatter", () => {
    const { meta, body } = parseSkillFrontmatter("Just plain text");
    expect(Object.keys(meta)).toHaveLength(0);
    expect(body).toBe("Just plain text");
  });

  it("handles multi-line description with >", () => {
    const content = `---
name: complex-skill
description: >
  A long description
  spanning multiple lines
allowed-tools:
  - tool.one
---
Body here`;

    const { meta, body } = parseSkillFrontmatter(content);
    expect(meta.name).toBe("complex-skill");
    expect(meta["allowed-tools"]).toEqual(["tool.one"]);
    expect(body).toContain("Body here");
  });

  it("handles empty allowed-tools", () => {
    const content = `---
name: no-tools
description: No tools needed
---
No tools body`;

    const { meta } = parseSkillFrontmatter(content);
    expect(meta.name).toBe("no-tools");
    expect(meta["allowed-tools"]).toBeUndefined();
  });
});

describe("SkillLoader", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "skill-loader-test-"));
    const skillDir = path.join(tmpDir, "test-skill");
    fs.mkdirSync(skillDir, { recursive: true });
    fs.writeFileSync(path.join(skillDir, "SKILL.md"), `---
name: test-skill
description: A test skill for unit tests
allowed-tools:
  - workflow.create
  - memory.store
---

# Test Skill Steps
1. Create a workflow
2. Store a memory`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("loads skills from directory", () => {
    const loader = new SkillLoader(tmpDir);
    const skills = loader.loadAll();
    expect(skills).toHaveLength(1);
    expect(skills[0].name).toBe("test-skill");
    expect(skills[0].allowedTools).toEqual(["workflow.create", "memory.store"]);
    expect(skills[0].steps).toContain("Create a workflow");
  });

  it("load() returns skill by name", () => {
    const loader = new SkillLoader(tmpDir);
    const skill = loader.load("test-skill");
    expect(skill).toBeDefined();
    expect(skill!.name).toBe("test-skill");
  });

  it("load() returns undefined for missing skill", () => {
    const loader = new SkillLoader(tmpDir);
    expect(loader.load("nonexistent")).toBeUndefined();
  });

  it("has() returns correct boolean", () => {
    const loader = new SkillLoader(tmpDir);
    expect(loader.has("test-skill")).toBe(true);
    expect(loader.has("missing")).toBe(false);
  });

  it("getAll() returns name and description arrays", () => {
    const loader = new SkillLoader(tmpDir);
    const all = loader.getAll();
    expect(all).toEqual([{ name: "test-skill", description: "A test skill for unit tests" }]);
  });

  it("handles missing directory gracefully", () => {
    const loader = new SkillLoader("/nonexistent/path");
    expect(loader.loadAll()).toEqual([]);
    expect(loader.size).toBe(0);
  });
});

describe("LiveSkillRegistry", () => {
  let bridge: MCPToolBridge;
  let loader: SkillLoader;
  let tmpDir: string;
  let registry: LiveSkillRegistry;

  beforeEach(() => {
    bridge = new MCPToolBridge();
    bridge.register("workflow.create", async (input) => ({ created: true, ...(input as Record<string, unknown>) }));
    bridge.register("memory.store", async (input) => ({ stored: true, ...(input as Record<string, unknown>) }));
    bridge.register("policy.validate", async (input) => ({ valid: true, ...(input as Record<string, unknown>) }));

    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "registry-test-"));
    const skillDir = path.join(tmpDir, "test-skill");
    fs.mkdirSync(skillDir, { recursive: true });
    fs.writeFileSync(path.join(skillDir, "SKILL.md"), `---
name: test-skill
description: Test skill
allowed-tools:
  - workflow.create
  - memory.store
---
Steps here`);

    loader = new SkillLoader(tmpDir);
    registry = new LiveSkillRegistry(loader, bridge);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("execute routes direct tool calls through the bridge", async () => {
    const result = await registry.execute("test-skill", {
      tool: "workflow.create",
      args: { name: "my-workflow" },
    });
    expect(result).toEqual({ created: true, name: "my-workflow" });
  });

  it("execute rejects disallowed tools", async () => {
    await expect(
      registry.execute("test-skill", {
        tool: "policy.validate",
        args: {},
      }),
    ).rejects.toThrow('Tool "policy.validate" not allowed for skill "test-skill"');
  });

  it("execute throws for missing skill", async () => {
    await expect(
      registry.execute("missing-skill", {}),
    ).rejects.toThrow("Skill not found: missing-skill");
  });

  it("execute uses default tool when no tool specified", async () => {
    const result = await registry.execute("test-skill", { name: "default-test" });
    expect(result).toEqual({ created: true, name: "default-test" });
  });

  it("has() delegates to loader", () => {
    expect(registry.has("test-skill")).toBe(true);
    expect(registry.has("missing")).toBe(false);
  });

  it("getAll() delegates to loader", () => {
    const all = registry.getAll();
    expect(all).toEqual([{ name: "test-skill", description: "Test skill" }]);
  });
});

// ─── Integration Tests ───

describe("Integration: Load real project skills", () => {
  const projectRoot = path.resolve(__dirname, "../../..");
  const skillsDir = path.join(projectRoot, ".claude", "skills");

  it("loads all SKILL.md files from the real project", () => {
    const loader = new SkillLoader(skillsDir);
    const skills = loader.loadAll();
    expect(skills.length).toBeGreaterThanOrEqual(30);
    for (const skill of skills) {
      expect(skill.name).toBeTruthy();
      expect(skill.filePath).toContain("SKILL.md");
    }
  });

  it("each skill has valid structure", () => {
    const loader = new SkillLoader(skillsDir);
    const skills = loader.loadAll();
    for (const skill of skills) {
      expect(skill.name).toMatch(/^masday-/);
      expect(typeof skill.description).toBe("string");
      expect(skill.steps.length).toBeGreaterThan(0);
    }
  });
});

describe("Integration: Full flow — skill load → bridge → handler", () => {
  let bridge: MCPToolBridge;
  let loader: SkillLoader;
  let registry: LiveSkillRegistry;
  let tmpDir: string;

  beforeEach(() => {
    bridge = new MCPToolBridge();

    bridge.register("workflow.create", async (input) => {
      return { id: "wf-1", ...(input as Record<string, unknown>) };
    });
    bridge.register("memory.store", async (input) => {
      return { id: "mem-1", ...(input as Record<string, unknown>) };
    });

    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "integration-test-"));
    const skillDir = path.join(tmpDir, "run-workflow");
    fs.mkdirSync(skillDir, { recursive: true });
    fs.writeFileSync(path.join(skillDir, "SKILL.md"), `---
name: run-workflow
description: Runs a workflow
allowed-tools:
  - workflow.create
  - memory.store
---
1. Create workflow
2. Store memory`);

    loader = new SkillLoader(tmpDir);
    registry = new LiveSkillRegistry(loader, bridge);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("full flow: load skill → route tool call → execute handler", async () => {
    const skill = loader.load("run-workflow");
    expect(skill).toBeDefined();
    expect(skill!.allowedTools).toContain("workflow.create");

    const result = await registry.execute("run-workflow", {
      tool: "workflow.create",
      args: { name: "test-wf" },
    });
    expect(result).toEqual({ id: "wf-1", name: "test-wf" });
  });

  it("full flow: disallowed tool is rejected", async () => {
    await expect(
      registry.execute("run-workflow", {
        tool: "memory.delete",
        args: { id: "x" },
      }),
    ).rejects.toThrow('Tool "memory.delete" not allowed for skill "run-workflow"');
  });

  it("full flow: default action routes to first allowed tool", async () => {
    const result = await registry.execute("run-workflow", { name: "auto" });
    expect(result).toEqual({ id: "wf-1", name: "auto" });
  });
});
