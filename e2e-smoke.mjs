#!/usr/bin/env node
/**
 * E2E Smoke Test: All API routes + All MCP tools
 * Usage: node e2e-smoke.mjs
 */
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const API = "http://localhost:3000";
const results = { api: [], mcp: [] };
let pass = 0, fail = 0;

function log(cat, name, ok, detail = "") {
  const status = ok ? "PASS" : "FAIL";
  if (ok) pass++; else fail++;
  results[cat].push({ name, status, detail });
  console.log(`[${status}] ${cat}/${name} ${detail ? "- " + detail : ""}`);
}

async function testApiRoutes() {
  console.log("\n=== API ROUTE TESTS ===\n");

  const routes = [
    ["GET", "/health"],
    ["GET", "/api/workflows"],
    ["GET", "/api/memory?workflowId=test"],
    ["POST", "/api/search/context-pack", { workflowId: "test", planId: "test", taskId: "test" }],
    ["POST", "/api/search/fingerprint", { workflowId: "test", planId: "test", taskId: "test" }],
    ["POST", "/api/search/code", { query: "test" }],
    ["POST", "/api/policy/check-readiness", { sessionKey: "test" }],
    ["POST", "/api/policy/validate-execution", { sessionKey: "test", workflowId: "test", taskId: "test" }],
    ["POST", "/api/policy/validate-completion", { workflowId: "test", taskId: "test" }],
    ["POST", "/api/policy/validate-parallel", { workflowId: "test", branchResults: [] }],
    ["POST", "/api/policy/detect-drift", { workflowId: "test", originalScope: "test", currentInput: "test" }],
    ["POST", "/api/policy/require-context-refresh", { workflowId: "test", planId: "test", taskId: "test" }],
    ["GET", "/api/capability/agents?projectRoot=."],
    ["GET", "/api/capability/skills?projectRoot=."],
    ["GET", "/api/capability/templates"],
    ["GET", "/api/capability/readiness?projectRoot=."],
    ["GET", "/api/monitoring/health"],
    ["GET", "/api/monitoring/metrics"],
    ["GET", "/api/monitoring/stats"],
    ["GET", "/api/providers"],
    ["POST", "/api/chat", { message: "hello" }],
  ];

  for (const [method, path, body] of routes) {
    try {
      const r = await fetch(`${API}${path}`, {
        method,
        headers: { "Content-Type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      });
      log("api", `${method} ${path}`, r.ok || r.status === 400 || r.status === 401, `status=${r.status}`);
    } catch (e) {
      log("api", `${method} ${path}`, false, e.message);
    }
  }
}

async function testMcpTools() {
  console.log("\n=== MCP TOOL SMOKE TESTS ===\n");

  const transport = new StdioClientTransport({
    command: "npx",
    args: ["tsx", "apps/agent-runner/src/runtime/mcp.ts"],
    env: { ...process.env, MASDAY_STORE_PATH: ".masday/state/masday.json" },
  });

  const client = new Client({ name: "e2e-smoke", version: "1.0.0" });
  await client.connect(transport);
  const { tools } = await client.listTools();
  console.log(`Discovered ${tools.length} MCP tools\n`);

  const toolTests = [
    ["workflow.create", { name: "e2e-test" }],
    ["workflow.list", {}],
    ["workflow.getActive", {}],
    ["workflow.ping", {}],
    ["workflow.createPlan", { workflow_id: "fake", summary: "t", created_by_agent: "e2e", content: { tasks: [{ title: "t" }] } }],
    ["workflow.listTasks", { workflow_id: "fake" }],
    ["workflow.getCurrentTask", { workflow_id: "fake" }],
    ["workflow.getPlan", { workflow_id: "fake" }],
    ["workflow.getStatus", { id: "fake" }],
    ["workflow.get", { id: "fake" }],
    ["workflow.resume_suggestion", { workflow_id: "fake" }],
    ["memory.store", { memoryType: "fact", summary: "e2e", content: "test", created_by_agent: "e2e" }],
    ["memory.recall_recent", { limit: 5 }],
    ["memory.search", { query: "test" }],
    ["memory.stats", {}],
    ["memory.store_research", { summary: "t", content: "t", created_by_agent: "e2e" }],
    ["memory.recall_documents", { workflow_id: "fake" }],
    ["memory.recall_document_by_type", { workflow_id: "fake", source_type: "research" }],
    ["memory.recall_by_task", { task_id: "fake" }],
    ["semantic-search.code_search", { query: "workflow" }],
    ["semantic-search.search_hybrid_context_pack", { workflow_id: "f", plan_id: "f", task_id: "f" }],
    ["semantic-search.search_context_fingerprint", { workflow_id: "f", plan_id: "f", task_id: "f" }],
    ["policy.check_session_readiness", { sessionKey: "e2e" }],
    ["policy.validate_execution", { workflowId: "f", taskId: "f", sessionKey: "e2e" }],
    ["policy.validate_completion", { workflowId: "f", taskId: "f" }],
    ["policy.validate_parallel_completion", { workflowId: "f", taskId: "f", sessionKey: "e2e" }],
    ["policy.detect_scope_drift", { workflowId: "f", taskId: "f", outputText: "test" }],
    ["policy.require_context_refresh", { workflowId: "f", planId: "f", taskId: "f" }],
    ["capability.list_agents", { projectRoot: "." }],
    ["capability.list_skills", { projectRoot: "." }],
    ["capability.list_templates", {}],
    ["capability.match_agent", { taskDescription: "code" }],
    ["capability.system_readiness", { projectRoot: "." }],
    ["capability.ping", {}],
    ["filesystem.list", { path: "." }],
    ["filesystem.read", { path: "package.json" }],
    ["filesystem.stat", { path: "package.json" }],
    ["review.get_latest", { workflow_id: "f", task_id: "f" }],
    ["session.get_state", { session_key: "e2e" }],
    ["session.init_context", { cwd: "." }],
    ["session.patch_state", { session_key: "e2e", patch: { test: true } }],
    ["local.init", { cwd: "." }],
    ["local.sync", { cwd: ".", workflow_id: "f" }],
    ["local.save_artifact", { cwd: ".", category: "reports", filename: "t.md", content: "# t" }],
    ["git.status", {}],
    ["git.diff", {}],
    ["tests.run", {}],
    ["reminder.list", {}],
    ["reminder.check", {}],
  ];

  for (const [toolName, args] of toolTests) {
    try {
      const result = await client.callTool({ name: toolName, arguments: args }, undefined, { timeout: 30000 });
      const text = result?.content?.[0]?.text || "";
      log("mcp", toolName, true, text.slice(0, 80));
    } catch (e) {
      log("mcp", toolName, false, e.message?.slice(0, 100));
    }
  }

  await client.close();
}

async function main() {
  console.log("=".repeat(60));
  console.log("  E2E SMOKE TEST: API Routes + MCP Tools");
  console.log("=".repeat(60));

  await testApiRoutes();
  await testMcpTools();

  console.log("\n" + "=".repeat(60));
  console.log(`  RESULTS: ${pass} PASS, ${fail} FAIL (total ${pass + fail})`);
  console.log("=".repeat(60));

  const fs = await import("fs");
  fs.writeFileSync("e2e-results.json", JSON.stringify(results, null, 2));
  console.log("\nResults saved to e2e-results.json");
  process.exit(fail > 0 ? 1 : 0);
}

main().catch(e => { console.error("FATAL:", e); process.exit(1); });
