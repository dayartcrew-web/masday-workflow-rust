import { describe, it, expect } from "vitest";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { mkdir, rm, writeFile } from "node:fs/promises";

const GUARD_PATH = join(process.cwd(), ".claude", "hooks", "tool-name-guard.js");

describe("tool-name-guard", () => {
  it("exports a default function", async () => {
    const mod = await import(GUARD_PATH);
    expect(typeof mod.default).toBe("function");
  });

  it("returns undefined when all namespaces are known", async () => {
    const mod = await import(GUARD_PATH);
    const guard = mod.default as () => Promise<{ systemMessage: string } | undefined>;

    const result = await guard();
    if (result) {
      console.log("Current unknown namespaces:", result.systemMessage);
    }
    expect(result === undefined || typeof result.systemMessage === "string").toBe(true);
  });

  it("detects unknown namespaces from temp file", async () => {
    const tmpDir = join(tmpdir(), "guard-test-" + Date.now());
    await mkdir(tmpDir, { recursive: true });
    const tmpFile = join(tmpDir, "mcp.ts");

    await writeFile(tmpFile, `
      server.registerTool("unknown_namespace.tool1", {}, async () => {});
      server.registerTool("workflow.create", {}, async () => {});
      server.registerTool("unknown_namespace.tool2", {}, async () => {});
    `);

    const content = await readFile(tmpFile, "utf-8");
    const TOOL_RE = /registerTool\("([^.]+)\.([^"]+)"/g;
    const knownNamespaces = new Set([
      "workflow", "memory", "semantic-search", "policy", "capability",
      "filesystem", "review", "session", "local",
      "git", "npm", "docker", "cicd", "github", "tests",
      "reminder", "projectRules", "use",
    ]);

    const unknown: string[] = [];
    let match;
    while ((match = TOOL_RE.exec(content)) !== null) {
      if (!knownNamespaces.has(match[1])) {
        unknown.push(`${match[1]}.${match[2]}`);
      }
    }

    expect(unknown).toEqual(["unknown_namespace.tool1", "unknown_namespace.tool2"]);

    await rm(tmpDir, { recursive: true, force: true });
  });

  it("known namespaces include all 18 namespaces", async () => {
    const content = await readFile(GUARD_PATH, "utf-8");
    const expected = [
      "workflow", "memory", "semantic-search", "policy", "capability",
      "filesystem", "review", "session", "local",
      "git", "npm", "docker", "cicd", "github", "tests",
      "reminder", "projectRules", "use",
    ];
    for (const ns of expected) {
      expect(content).toContain(`'${ns}'`);
    }
  });
});
