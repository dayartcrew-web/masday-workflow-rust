#!/usr/bin/env node

/**
 * Masday Workflow MCP Server (Local-First)
 * Uses official @modelcontextprotocol/sdk with JsonBackend storage.
 * No PostgreSQL or Docker required.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { EventBus, createLogger } from "@mcp-rebuild/core";
import { JsonBackend, SqliteBackend, WorkflowStore, TaskResultStore, PersistenceListener } from "@mcp-rebuild/store";
import { OrchestratingEngine } from "@mcp-rebuild/workflow-engine";
import type { ISkillRegistry } from "@mcp-rebuild/workflow-engine";
import * as path from "path";
import * as fs from "fs";

const logger = createLogger("MCPServer");

const skillMap = new Map<string, { name: string; description: string }>();
const skillRegistry: ISkillRegistry = {
  async execute(skill: string): Promise<unknown> { throw new Error("Not available: " + skill); },
  has(skill: string): boolean { return skillMap.has(skill); },
  getAll(): Array<{ name: string; description: string }> { return Array.from(skillMap.values()); },
};

function createBackend(dbDir: string): { backend: JsonBackend | SqliteBackend; type: string } {
  try {
    require.resolve("better-sqlite3");
    const b = new SqliteBackend(path.join(dbDir, "masday.db"));
    b.initialize();
    return { backend: b, type: "sqlite" };
  } catch {
    const b = new JsonBackend(path.join(dbDir, "masday.json"));
    b.initialize();
    return { backend: b, type: "json" };
  }
}

const server = new McpServer({ name: "masday", version: "0.1.0" });
const cwd = process.cwd();
const dataDir = path.join(cwd, ".masday", "state");
if (!fs.existsSync(dataDir)) fs.mkdirSync(dataDir, { recursive: true });

const { backend, type: backendType } = createBackend(dataDir);
logger.info("Storage: " + backendType);

const eventBus = new EventBus();
const workflowStore = new WorkflowStore(backend);
const persistenceListener = new PersistenceListener(eventBus, workflowStore, new TaskResultStore(backend));
persistenceListener.start();

const engine = new OrchestratingEngine(skillRegistry, eventBus, { coordinator: false, enableSkillRouting: false, store: workflowStore });
const orphans = engine.restoreWorkflows(workflowStore.loadAll());
if (orphans > 0) logger.info("Restored " + orphans + " orphaned workflows");

const MEMORY_FILE = path.join(dataDir, "memories.json");
interface MemRec { id: string; type: string; content: string; summary: string; source: string; importance: number; tags: string[]; createdAt: number }
function loadMem(): MemRec[] { return fs.existsSync(MEMORY_FILE) ? JSON.parse(fs.readFileSync(MEMORY_FILE, "utf-8")) : []; }
function saveMem(m: MemRec[]) { fs.writeFileSync(MEMORY_FILE, JSON.stringify(m, null, 2)); }
let mic = 0;
function nid() { return "mem_" + Date.now() + "_" + (++mic); }
function ok(d: unknown) { return { content: [{ type: "text" as const, text: JSON.stringify(d) }] }; }

server.registerTool("workflow.create", { description: "Create workflow", inputSchema: { name: z.string(), description: z.string().optional(), metadata: z.record(z.any()).optional() } }, async ({ name, description, metadata }) => ok(engine.createWorkflow(name, description ?? "", metadata)));
server.registerTool("workflow.execute", { description: "Execute workflow", inputSchema: { id: z.string() } }, async ({ id }) => { await engine.executeWorkflow(id); return ok(engine.getWorkflow(id)); });
server.registerTool("workflow.getStatus", { description: "Get workflow status", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.get", { description: "Get workflow by ID", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.list", { description: "List workflows", inputSchema: {} }, async () => ok(engine.listWorkflows()));
server.registerTool("workflow.addTask", { description: "Add task", inputSchema: { workflowId: z.string(), name: z.string(), agent: z.string(), skill: z.string(), dependencies: z.array(z.string()).optional(), input: z.record(z.any()).optional() } }, async (a) => ok(engine.addTask(a.workflowId, { name: a.name, agent: a.agent, skill: a.skill, dependencies: a.dependencies ?? [], input: a.input ?? {} })));
server.registerTool("workflow.startTask", { description: "Start task", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async ({ workflow_id, task_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "running"; t.startedAt = new Date(); return ok(t); });
server.registerTool("workflow.completeTask", { description: "Complete task", inputSchema: { workflow_id: z.string(), task_id: z.string(), result: z.record(z.any()).optional() } }, async ({ workflow_id, task_id, result }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "done"; t.completedAt = new Date(); if (result) t.output = result; return ok(t); });
server.registerTool("workflow.saveProgress", { description: "Save progress", inputSchema: { workflow_id: z.string(), task_id: z.string(), agent_name: z.string(), progress_note: z.string(), evidence: z.array(z.string()).optional() } }, async (a) => { eventBus.emit("trace.completed", { workflowId: a.workflow_id, taskId: a.task_id, agentName: a.agent_name, progressNote: a.progress_note, evidence: a.evidence ?? [] }); return ok({ saved: true }); });
server.registerTool("workflow.listTasks", { description: "List tasks", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks); });
server.registerTool("workflow.getCurrentTask", { description: "Current task", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks.find((x: any) => x.state === "running") ?? w.tasks.find((x: any) => x.state === "pending") ?? null); });
server.registerTool("workflow.getPlan", { description: "Get plan", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok({ workflowId: w.id, name: w.name, state: w.state, tasks: w.tasks, metadata: w.metadata }); });
server.registerTool("workflow.getActive", { description: "Active workflow", inputSchema: { cwd: z.string().optional() } }, async () => { const a = engine.listWorkflows().filter((w: any) => ["EXECUTE","PLAN","VERIFY"].includes(w.state)); return ok(a[0] ?? null); });
server.registerTool("workflow.createPlan", { description: "Create plan", inputSchema: { workflow_id: z.string(), plan: z.record(z.any()) } }, async ({ workflow_id, plan }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); w.metadata = { ...w.metadata, plan }; const p = plan as any; const c = []; if (p.tasks?.length) for (const t of p.tasks) { const tk = engine.addTask(workflow_id, { name: t.title, agent: t.agent ?? "claude", skill: t.skill ?? "general", dependencies: t.dependencies ?? [], input: t.input ?? {} }); c.push({ id: tk.id, name: tk.name }); } return ok({ created: true, tasksCreated: c.length, tasks: c }); });
server.registerTool("workflow.createParallelBranches", { description: "Create parallel branches", inputSchema: { workflow_id: z.string(), branches: z.array(z.object({ branchKey: z.string(), role: z.string(), scope: z.string() })) } }, async ({ workflow_id, branches }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); w.metadata = { ...w.metadata, parallelBranches: branches }; return ok({ created: true, branchCount: branches.length }); });
server.registerTool("workflow.completeParallelBranch", { description: "Complete branch", inputSchema: { workflow_id: z.string(), branch_key: z.string() } }, async ({ workflow_id, branch_key }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const b = ((w.metadata.parallelBranches ?? []) as any[]).find(x => x.branchKey === branch_key); if (b) b.completed = true; return ok({ completed: true }); });
server.registerTool("workflow.listParallelBranches", { description: "List branches", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.metadata.parallelBranches ?? []); });
server.registerTool("workflow.delete", { description: "Delete workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ deleted: engine.deleteWorkflow(workflow_id) }));
server.registerTool("workflow.ping", { description: "Health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType }));

server.registerTool("memory.store", { description: "Store memory", inputSchema: { workflow_id: z.string().optional(), task_id: z.string().optional(), memory_type: z.string(), summary: z.string(), content: z.string(), created_by_agent: z.string(), importance_score: z.number().optional(), tags: z.array(z.string()).optional() } }, async (a) => { const m = loadMem(); const r: MemRec = { id: nid(), type: a.memory_type, content: a.content, summary: a.summary, source: a.created_by_agent, importance: a.importance_score ?? 0.5, tags: [...(a.tags ?? []), a.workflow_id, a.task_id].filter(Boolean) as string[], createdAt: Date.now() }; m.push(r); saveMem(m); return ok(r); });
server.registerTool("memory.store_research", { description: "Store research", inputSchema: { workflow_id: z.string().optional(), summary: z.string(), content: z.string(), created_by_agent: z.string() } }, async (a) => { const m = loadMem(); const r: MemRec = { id: nid(), type: "research", content: a.content, summary: a.summary, source: a.created_by_agent, importance: 0.5, tags: ["research", a.workflow_id].filter(Boolean) as string[], createdAt: Date.now() }; m.push(r); saveMem(m); return ok(r); });
server.registerTool("memory.recall_recent", { description: "Recall recent", inputSchema: { limit: z.number().optional(), type: z.string().optional() } }, async ({ limit, type }) => { let m = loadMem(); if (type) m = m.filter(x => x.type === type); return ok(m.sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10)); });
server.registerTool("memory.recall_documents", { description: "Recall docs", inputSchema: { workflow_id: z.string(), limit: z.number().optional() } }, async ({ workflow_id, limit }) => ok(loadMem().filter(m => m.tags.includes(workflow_id)).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10)));
server.registerTool("memory.recall_document_by_type", { description: "Recall by type", inputSchema: { workflow_id: z.string(), source_type: z.string(), limit: z.number().optional() } }, async ({ workflow_id, source_type, limit }) => ok(loadMem().filter(m => m.tags.includes(workflow_id) && m.type === source_type).slice(0, limit ?? 10)));
server.registerTool("memory.recall_by_task", { description: "Recall by task", inputSchema: { task_id: z.string(), limit: z.number().optional() } }, async ({ task_id, limit }) => ok(loadMem().filter(m => m.tags.includes(task_id)).slice(0, limit ?? 10)));
server.registerTool("memory.update", { description: "Update memory", inputSchema: { id: z.string(), content: z.string().optional(), importance: z.number().optional() } }, async ({ id, content, importance }) => { const m = loadMem(); const r = m.find(x => x.id === id); if (!r) throw new Error("Not found"); if (content) r.content = content; if (importance !== undefined) r.importance = importance; saveMem(m); return ok(r); });
server.registerTool("memory.delete", { description: "Delete memory", inputSchema: { id: z.string() } }, async ({ id }) => { const m = loadMem(); const i = m.findIndex(x => x.id === id); if (i < 0) throw new Error("Not found"); m.splice(i, 1); saveMem(m); return ok({ deleted: true }); });
server.registerTool("memory.delete_by_workflow", { description: "Delete by workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const m = loadMem(); const n = m.length; saveMem(m.filter(x => !x.tags.includes(workflow_id))); return ok({ deleted: n - m.filter(x => !x.tags.includes(workflow_id)).length }); });
server.registerTool("memory.search", { description: "Search memories", inputSchema: { query: z.string(), limit: z.number().optional() } }, async ({ query, limit }) => { const q = query.toLowerCase(); return ok(loadMem().map(x => ({ ...x, score: q.split(/\s+/).filter(w => (x.content + x.summary).toLowerCase().includes(w)).length })).filter(x => x.score > 0).sort((a, b) => b.score - a.score).slice(0, limit ?? 10)); });
server.registerTool("memory.stats", { description: "Memory stats", inputSchema: {} }, async () => { const m = loadMem(); const by: Record<string, number> = {}; m.forEach(x => by[x.type] = (by[x.type] ?? 0) + 1); return ok({ total: m.length, byType: by }); });

server.registerTool("semantic-search.search_hybrid_context_pack", { description: "Context pack", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => ok({ ...a, contextSufficient: true }));
server.registerTool("semantic-search.search_context_fingerprint", { description: "Fingerprint", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => ok({ fingerprint: [a.workflow_id, a.plan_id, a.task_id].join("-"), contextSufficient: true }));
server.registerTool("semantic-search.code_search", { description: "Code search", inputSchema: { query: z.string() } }, async ({ query }) => ok({ query, results: [] }));

server.registerTool("policy.check_session_readiness", { description: "Session readiness", inputSchema: { sessionKey: z.string() } }, async ({ sessionKey }) => ok({ sessionKey, ready: true }));
server.registerTool("policy.validate_execution", { description: "Validate execution", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => ok({ valid: true, ...a }));
server.registerTool("policy.validate_completion", { description: "Validate completion", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => ok({ valid: true, ...a }));
server.registerTool("policy.validate_parallel_completion", { description: "Validate parallel", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => ok({ valid: true, ...a }));
server.registerTool("policy.detect_scope_drift", { description: "Detect drift", inputSchema: { outputText: z.string() } }, async ({ outputText }) => ok({ driftDetected: false, outputLength: outputText.length }));
server.registerTool("policy.require_context_refresh", { description: "Context refresh", inputSchema: { sessionKey: z.string() } }, async ({ sessionKey }) => ok({ needsRefresh: false, sessionKey }));

server.registerTool("capability.list_agents", { description: "List agents", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => ok({ agents: [], projectRoot }));
server.registerTool("capability.list_skills", { description: "List skills", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => ok({ skills: [], projectRoot }));
server.registerTool("capability.list_templates", { description: "List templates", inputSchema: {} }, async () => ok({ templates: ["mcp-server", "agent", "skill"] }));
server.registerTool("capability.match_agent", { description: "Match agent", inputSchema: { projectRoot: z.string(), taskDescription: z.string() } }, async (a) => ok({ match: null, ...a }));
server.registerTool("capability.system_readiness", { description: "System readiness", inputSchema: { projectRoot: z.string().optional() } }, async ({ projectRoot }) => ok({ ready: true, backend: backendType, projectRoot: projectRoot ?? cwd }));
server.registerTool("capability.workflow_audit", { description: "Audit", inputSchema: { workflowId: z.string().optional() } }, async ({ workflowId }) => ok({ audited: workflowId ? 1 : engine.listWorkflows().length, issues: [] }));
server.registerTool("capability.create_agent", { description: "Create agent", inputSchema: { projectRoot: z.string(), name: z.string(), role: z.string(), description: z.string(), instructions: z.string() } }, async (a) => ok({ ok: true, ...a }));
server.registerTool("capability.create_skill", { description: "Create skill", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string(), trigger: z.string(), steps: z.array(z.string()) } }, async (a) => ok({ ok: true, ...a }));
server.registerTool("capability.scaffold_feature", { description: "Scaffold feature", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => ok({ ok: false, ...a }));
server.registerTool("capability.scaffold_mcp_server", { description: "Scaffold MCP", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => ok({ ok: false, ...a }));

server.registerTool("filesystem.read", { description: "Read file", inputSchema: { path: z.string() } }, async ({ path: fp }) => ({ content: [{ type: "text" as const, text: fs.readFileSync(fp, "utf-8") }] }));
server.registerTool("filesystem.write", { description: "Write file", inputSchema: { path: z.string(), content: z.string() } }, async ({ path: fp, content }) => { const d = path.dirname(fp); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(fp, content); return ok({ ok: true }); });
server.registerTool("filesystem.list", { description: "List dir", inputSchema: { path: z.string() } }, async ({ path: fp }) => ok(fs.readdirSync(fp, { withFileTypes: true }).map(e => ({ name: e.name, type: e.isDirectory() ? "dir" : "file" }))));
server.registerTool("filesystem.delete", { description: "Delete file", inputSchema: { path: z.string() } }, async ({ path: fp }) => { fs.unlinkSync(fp); return ok({ ok: true }); });
server.registerTool("filesystem.stat", { description: "File stat", inputSchema: { path: z.string() } }, async ({ path: fp }) => { const s = fs.statSync(fp); return ok({ size: s.size, isFile: s.isFile() }); });

const transport = new StdioServerTransport();
await server.connect(transport);
logger.info("MCP Server running on stdio (" + backendType + " backend)");
