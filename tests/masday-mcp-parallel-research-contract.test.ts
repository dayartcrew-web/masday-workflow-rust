import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const runtime = readFileSync("apps/agent-runner/src/runtime/mcp.ts", "utf8");

describe("masday MCP runtime contract", () => {
  it("exposes the tools required by parallel research orchestration", () => {
    expect(runtime).toContain('server.registerTool("workflow.createParallelBranches"');
    expect(runtime).toContain('server.registerTool("workflow.completeParallelBranch"');
    expect(runtime).toContain('server.registerTool("workflow.listParallelBranches"');
    expect(runtime).toContain('server.registerTool("memory.store_research"');
    expect(runtime).toContain('server.registerTool("local.save_artifact"');
  });
});
