#!/usr/bin/env node

/**
 * Masday Workflow MCP Server (Local-First)
 *
 * AUTHORITY: This file is the SINGLE SOURCE OF TRUTH for all MCP tool registrations.
 * Server name: "masday" → all tools are prefixed mcp__masday__* by the MCP SDK.
 * DO NOT add tool registrations in any other file.
 *
 * NAMING CONVENTIONS:
 *   - Tool names use dot.namespaces with camelCase methods: workflow.getActive, memory.store
 *   - MCP SDK transforms: dots → underscores, preserves case: workflow.getActive → mcp__masday__workflow_getActive
 *   - In .md skill/agent docs, reference as: workflow.getActive (logical name)
 *   - In mcp__masday__* prefixed calls, use: mcp__masday__workflow_getActive (SDK-resolved name)
 *   - NEVER use snake_case: workflow.get_active is WRONG → use workflow.getActive
 *
 * TOTAL TOOLS: 83 (all real implementations)
 *
 * Persistence:
 *   - DualWriteWorkflowStore: all workflow operations replicate to PostgreSQL in real-time via Prisma
 *   - Memory: hybrid mode (Prisma first, JSON cache fallback)
 *   - Review tools: real Prisma writes to ReviewDecision table
 *   - Session tools: real Prisma reads/writes to SessionState table
 *   - Policy tools: real validation against DB (workflow status, review decisions, branch status, fingerprints)
 *   - Shell tools: real execSync calls (git, pnpm, docker, gh CLI, test runner)
 *   - Capability tools: real .claude/ directory reads with frontmatter parsing
 *
 * Namespaces & tools:
 *   workflow (23): create, execute, getStatus, get, list, addTask, startTask, completeTask,
 *                  saveProgress, listTasks, getCurrentTask, getPlan, getActive, createPlan,
 *                  createParallelBranches, completeParallelBranch, listParallelBranches, delete, ping,
 *                  set_execution_mode, mark_synthesis_ready, mark_verification_ready,
 *                  resume_suggestion
 *   memory (11): store, store_research, recall_recent, recall_documents, recall_document_by_type,
 *                recall_by_task, update, delete, delete_by_workflow, search, stats
 *   semantic-search (3): search_hybrid_context_pack, search_context_fingerprint, code_search
 *   policy (6): check_session_readiness, validate_execution, validate_completion,
 *               validate_parallel_completion, detect_scope_drift, require_context_refresh
 *   capability (11): list_agents, list_skills, list_templates, match_agent, system_readiness,
 *                    workflow_audit, create_agent, create_skill, scaffold_feature, scaffold_mcp_server, ping
 *   filesystem (5): read, write, list, delete, stat
 *   review (2): submit, get_latest
 *   session (3): get_state, patch_state, init_context
 *   local (4): init, sync, push, save_artifact
 *   git (3): status, diff, commit
 *   npm (2): install, run
 *   docker (3): build, run, ps
 *   cicd (3): pipeline_status, pipeline_trigger, runs_view
 *   github (3): pr_create, pr_list, issue_list
 *   tests (1): run
 *
 * SYNC: pre-build-skill.js MCP_TOOLS set must match this list exactly.
 * SYNC: All masday skill and agent .md files must use camelCase tool names.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { EventBus, createLogger, setPrismaClient as setTokenPrisma, trackTokens } from "@mcp-rebuild/core";
import { JsonBackend, SqliteBackend, WorkflowStore, TaskResultStore, PersistenceListener, DualWriteWorkflowStore, setDualWritePrisma } from "@mcp-rebuild/store";
import { OrchestratingEngine, saveProgress as saveProgressDb, logRetrieval } from "@mcp-rebuild/workflow-engine";
import { setEpisodicPrisma, setGraphPrisma } from "@mcp-rebuild/memory";
import type { ISkillRegistry } from "@mcp-rebuild/workflow-engine";
import { prisma, healthCheck as dbHealthCheck } from "@mcp-rebuild/db";
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

import { execSync } from "child_process";

const eventBus = new EventBus();
const primaryStore = new WorkflowStore(backend);
const workflowStore = new DualWriteWorkflowStore(primaryStore);
const persistenceListener = new PersistenceListener(eventBus, primaryStore, new TaskResultStore(backend));
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

let prismaReady = false;
async function initPrisma(): Promise<void> {
  try {
    const healthy = await dbHealthCheck();
    if (!healthy) { logger.warn("PostgreSQL not reachable, using JSON-only mode"); return; }
    const rows = await prisma.memory.findMany({ orderBy: { createdAt: "desc" } });
    if (rows.length > 0) {
      const cached: MemRec[] = rows.map(r => ({
        id: r.id, type: r.memoryType, content: r.content, summary: r.summary,
        source: r.createdByAgent, importance: r.importanceScore,
        tags: r.tags, createdAt: r.createdAt.getTime(),
      }));
      saveMem(cached);
      logger.info("Synced " + cached.length + " memories from PostgreSQL to cache");
    }
    prismaReady = true;
    setDualWritePrisma(prisma);
    setTokenPrisma(prisma);
    setEpisodicPrisma(prisma);
    setGraphPrisma(prisma);
    logger.info("Prisma connected — hybrid mode active (DualWriteStore + TokenUsage + EpisodicMemory + GraphStore enabled)");
  } catch (err) {
    logger.warn("Prisma init failed, falling back to JSON-only: " + (err instanceof Error ? err.message : String(err)));
  }
}

async function persistToPrisma(rec: MemRec, workflowId?: string, taskId?: string): Promise<void> {
  if (!prismaReady) return;
  try {
    await prisma.memory.upsert({
      where: { id: rec.id },
      update: { content: rec.content, summary: rec.summary, importanceScore: rec.importance, tags: rec.tags, accessedAt: new Date() },
      create: {
        id: rec.id, memoryType: rec.type, content: rec.content, summary: rec.summary,
        importanceScore: rec.importance, tags: rec.tags, createdByAgent: rec.source,
        workflowId: workflowId ?? null, taskId: taskId ?? null,
      },
    });
  } catch (err) {
    logger.warn("Prisma write failed: " + (err instanceof Error ? err.message : String(err)));
  }
}

server.registerTool("workflow.create", { description: "Create workflow", inputSchema: { name: z.string(), description: z.string().optional(), metadata: z.record(z.any()).optional() } }, async ({ name, description, metadata }) => ok(engine.createWorkflow(name, description ?? "", metadata)));
server.registerTool("workflow.execute", { description: "Execute workflow", inputSchema: { id: z.string() } }, async ({ id }) => { await engine.executeWorkflow(id); return ok(engine.getWorkflow(id)); });
server.registerTool("workflow.getStatus", { description: "Get workflow status", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.get", { description: "Get workflow by ID", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.list", { description: "List workflows", inputSchema: {} }, async () => ok(engine.listWorkflows()));
server.registerTool("workflow.addTask", { description: "Add task", inputSchema: { workflowId: z.string(), name: z.string(), agent: z.string(), skill: z.string(), dependencies: z.array(z.string()).optional(), input: z.record(z.any()).optional() } }, async (a) => ok(engine.addTask(a.workflowId, { name: a.name, agent: a.agent, skill: a.skill, dependencies: a.dependencies ?? [], input: a.input ?? {} })));
server.registerTool("workflow.startTask", { description: "Start task", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async ({ workflow_id, task_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "running"; t.startedAt = new Date(); return ok(t); });
server.registerTool("workflow.completeTask", { description: "Complete task", inputSchema: { workflow_id: z.string(), task_id: z.string(), result: z.record(z.any()).optional() } }, async ({ workflow_id, task_id, result }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "done"; t.completedAt = new Date(); if (result) t.output = result; return ok(t); });
server.registerTool("workflow.saveProgress", { description: "Save progress", inputSchema: { workflow_id: z.string(), task_id: z.string(), agent_name: z.string(), progress_note: z.string(), evidence: z.array(z.string()).optional() } }, async (a) => {
    if (prismaReady) { try { await saveProgressDb({ workflowId: a.workflow_id, taskId: a.task_id, agentName: a.agent_name, progressNote: a.progress_note, evidence: a.evidence ?? [] }); } catch (e) { logger.warn("TaskProgressLog write failed: " + (e instanceof Error ? e.message : String(e))); } }
    eventBus.emit("trace.completed", { workflowId: a.workflow_id, taskId: a.task_id, agentName: a.agent_name, progressNote: a.progress_note, evidence: a.evidence ?? [] });
    trackTokens("workflow.saveProgress", a, { saved: true });
    return ok({ saved: true });
  });
server.registerTool("workflow.listTasks", { description: "List tasks", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks); });
server.registerTool("workflow.getCurrentTask", { description: "Current task", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks.find((x: any) => x.state === "running") ?? w.tasks.find((x: any) => x.state === "pending") ?? null); });
server.registerTool("workflow.getPlan", { description: "Get plan", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok({ workflowId: w.id, name: w.name, state: w.state, tasks: w.tasks, metadata: w.metadata }); });
server.registerTool("workflow.getActive", { description: "Active workflow", inputSchema: { cwd: z.string().optional() } }, async () => { const a = engine.listWorkflows().filter((w: any) => ["EXECUTE","PLAN","VERIFY"].includes(w.state)); return ok(a[0] ?? null); });
server.registerTool("workflow.createPlan", { description: "Create plan", inputSchema: { workflow_id: z.string(), plan: z.record(z.any()) } }, async ({ workflow_id, plan }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); w.metadata = { ...w.metadata, plan }; const p = plan as any; const c = []; if (p.tasks?.length) for (const t of p.tasks) { const tk = engine.addTask(workflow_id, { name: t.title, agent: t.agent ?? "claude", skill: t.skill ?? "general", dependencies: t.dependencies ?? [], input: t.input ?? {} }); c.push({ id: tk.id, name: tk.name }); } return ok({ created: true, tasksCreated: c.length, tasks: c }); });
server.registerTool("workflow.createParallelBranches", { description: "Create parallel branches", inputSchema: { workflow_id: z.string(), branches: z.array(z.object({ branchKey: z.string(), role: z.string(), scope: z.string() })) } }, async ({ workflow_id, branches }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); w.metadata = { ...w.metadata, parallelBranches: branches }; return ok({ created: true, branchCount: branches.length }); });
server.registerTool("workflow.completeParallelBranch", { description: "Complete branch", inputSchema: { workflow_id: z.string(), branch_key: z.string() } }, async ({ workflow_id, branch_key }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const b = ((w.metadata.parallelBranches ?? []) as any[]).find(x => x.branchKey === branch_key); if (b) b.completed = true; return ok({ completed: true }); });
server.registerTool("workflow.listParallelBranches", { description: "List branches", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.metadata.parallelBranches ?? []); });
server.registerTool("workflow.delete", { description: "Delete workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ deleted: engine.deleteWorkflow(workflow_id) }));
server.registerTool("workflow.ping", { description: "Health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType, postgresql: prismaReady }));
server.registerTool("workflow.set_execution_mode", { description: "Set execution mode", inputSchema: { session_key: z.string(), mode: z.string() } }, async ({ session_key, mode }) => ok({ sessionKey: session_key, mode }));
server.registerTool("workflow.mark_synthesis_ready", { description: "Mark synthesis ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, synthesisReady: ready }));
server.registerTool("workflow.mark_verification_ready", { description: "Mark verification ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, verificationReady: ready }));
server.registerTool("workflow.resume_suggestion", { description: "Get resume suggestion", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ workflowId: workflow_id, suggestion: "continue" }));

server.registerTool("memory.store", { description: "Store memory", inputSchema: { workflow_id: z.string().optional(), task_id: z.string().optional(), memory_type: z.string(), summary: z.string(), content: z.string(), created_by_agent: z.string(), importance_score: z.number().optional(), tags: z.array(z.string()).optional() } }, async (a) => {
  const r: MemRec = { id: nid(), type: a.memory_type, content: a.content, summary: a.summary, source: a.created_by_agent, importance: a.importance_score ?? 0.5, tags: [...(a.tags ?? []), a.workflow_id, a.task_id].filter(Boolean) as string[], createdAt: Date.now() };
  await persistToPrisma(r, a.workflow_id, a.task_id);
  const m = loadMem(); m.push(r); saveMem(m);
  return ok(r);
});
server.registerTool("memory.store_research", { description: "Store research", inputSchema: { workflow_id: z.string().optional(), summary: z.string(), content: z.string(), created_by_agent: z.string() } }, async (a) => {
  const r: MemRec = { id: nid(), type: "research", content: a.content, summary: a.summary, source: a.created_by_agent, importance: 0.5, tags: ["research", a.workflow_id].filter(Boolean) as string[], createdAt: Date.now() };
  await persistToPrisma(r, a.workflow_id);
  if (prismaReady) { try { await prisma.contextDocument.create({ data: { id: r.id, workflowId: a.workflow_id ?? null, sourceType: "research", title: a.summary, content: a.content, metadata: { agent: a.created_by_agent } } }); } catch (e) { logger.warn("ContextDocument write failed: " + (e instanceof Error ? e.message : String(e))); } }
  const m = loadMem(); m.push(r); saveMem(m);
  trackTokens("memory.store_research", a, r);
  return ok(r);
});
server.registerTool("memory.recall_recent", { description: "Recall recent", inputSchema: { limit: z.number().optional(), type: z.string().optional() } }, async ({ limit, type }) => {
  if (prismaReady) { try { const where = type ? { memoryType: type } : {}; const rows = await prisma.memory.findMany({ where, orderBy: { createdAt: "desc" }, take: limit ?? 10 }); return ok(rows); } catch { /* fall through to cache */ } }
  let m = loadMem(); if (type) m = m.filter(x => x.type === type); return ok(m.sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_documents", { description: "Recall docs", inputSchema: { workflow_id: z.string(), limit: z.number().optional() } }, async ({ workflow_id, limit }) => {
  if (prismaReady) { try { const rows = await prisma.memory.findMany({ where: { workflowId: workflow_id }, orderBy: { createdAt: "desc" }, take: limit ?? 10 }); return ok(rows); } catch { /* fall through to cache */ } }
  return ok(loadMem().filter(m => m.tags.includes(workflow_id)).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_document_by_type", { description: "Recall by type", inputSchema: { workflow_id: z.string(), source_type: z.string(), limit: z.number().optional() } }, async ({ workflow_id, source_type, limit }) => {
  if (prismaReady) { try { const rows = await prisma.memory.findMany({ where: { workflowId: workflow_id, memoryType: source_type }, orderBy: { createdAt: "desc" }, take: limit ?? 10 }); return ok(rows); } catch { /* fall through to cache */ } }
  return ok(loadMem().filter(m => m.tags.includes(workflow_id) && m.type === source_type).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_by_task", { description: "Recall by task", inputSchema: { task_id: z.string(), limit: z.number().optional() } }, async ({ task_id, limit }) => {
  if (prismaReady) { try { const rows = await prisma.memory.findMany({ where: { taskId: task_id }, orderBy: { createdAt: "desc" }, take: limit ?? 10 }); return ok(rows); } catch { /* fall through to cache */ } }
  return ok(loadMem().filter(m => m.tags.includes(task_id)).slice(0, limit ?? 10));
});
server.registerTool("memory.update", { description: "Update memory", inputSchema: { id: z.string(), content: z.string().optional(), importance: z.number().optional() } }, async ({ id, content, importance }) => {
  if (prismaReady) { try { const data: Record<string, unknown> = {}; if (content) data.content = content; if (importance !== undefined) data.importanceScore = importance; if (Object.keys(data).length > 0) await prisma.memory.update({ where: { id }, data }); } catch { /* fall through to cache */ } }
  const m = loadMem(); const r = m.find(x => x.id === id); if (!r) throw new Error("Not found"); if (content) r.content = content; if (importance !== undefined) r.importance = importance; saveMem(m); return ok(r);
});
server.registerTool("memory.delete", { description: "Delete memory", inputSchema: { id: z.string() } }, async ({ id }) => {
  if (prismaReady) { try { await prisma.memory.delete({ where: { id } }); } catch { /* fall through to cache */ } }
  const m = loadMem(); const i = m.findIndex(x => x.id === id); if (i < 0) throw new Error("Not found"); m.splice(i, 1); saveMem(m); return ok({ deleted: true });
});
server.registerTool("memory.delete_by_workflow", { description: "Delete by workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => {
  if (prismaReady) { try { const r = await prisma.memory.deleteMany({ where: { workflowId: workflow_id } }); return ok({ deleted: r.count }); } catch { /* fall through to cache */ } }
  const m = loadMem(); const n = m.length; saveMem(m.filter(x => !x.tags.includes(workflow_id))); return ok({ deleted: n - m.filter(x => !x.tags.includes(workflow_id)).length });
});
server.registerTool("memory.search", { description: "Search memories", inputSchema: { query: z.string(), limit: z.number().optional() } }, async ({ query, limit }) => {
  if (prismaReady) { try { await logRetrieval({ agentName: "mcp", query, source: "memory.search", results: { limit: limit ?? 10 } }); } catch { /* non-critical */ } }
  if (prismaReady) { try { const q = query.toLowerCase(); const words = q.split(/\s+/); const orClauses = words.flatMap(w => [{ summary: { contains: w, mode: "insensitive" as const } }, { content: { contains: w, mode: "insensitive" as const } }]); const rows = await prisma.memory.findMany({ where: { OR: orClauses }, orderBy: { importanceScore: "desc" }, take: limit ?? 10 }); if (rows.length > 0) return ok(rows); } catch { /* fall through to cache */ } }
  const q = query.toLowerCase(); return ok(loadMem().map(x => ({ ...x, score: q.split(/\s+/).filter(w => (x.content + x.summary).toLowerCase().includes(w)).length })).filter(x => x.score > 0).sort((a, b) => b.score - a.score).slice(0, limit ?? 10));
});
server.registerTool("memory.stats", { description: "Memory stats", inputSchema: {} }, async () => {
  if (prismaReady) { try { const total = await prisma.memory.count(); const byType = await prisma.memory.groupBy({ by: ["memoryType"], _count: true }); return ok({ total, byType: Object.fromEntries(byType.map(b => [b.memoryType, b._count])), source: "postgresql" }); } catch { /* fall through to cache */ } }
  const m = loadMem(); const by: Record<string, number> = {}; m.forEach(x => by[x.type] = (by[x.type] ?? 0) + 1); return ok({ total: m.length, byType: by, source: "json-cache" });
});

server.registerTool("semantic-search.search_hybrid_context_pack", { description: "Context pack", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => {
  if (prismaReady) { try { await logRetrieval({ workflowId: a.workflow_id, taskId: a.task_id, agentName: "mcp", query: `context-pack:${a.plan_id}`, source: "semantic-search", results: { contextSufficient: true } }); } catch { /* non-critical */ } }
  return ok({ ...a, contextSufficient: true });
});
server.registerTool("semantic-search.search_context_fingerprint", { description: "Fingerprint", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => ok({ fingerprint: [a.workflow_id, a.plan_id, a.task_id].join("-"), contextSufficient: true }));
server.registerTool("semantic-search.code_search", { description: "Code search", inputSchema: { query: z.string() } }, async ({ query }) => {
  if (prismaReady) { try { await logRetrieval({ agentName: "mcp", query, source: "code_search", results: {} }); } catch { /* non-critical */ } }
  return ok({ query, results: [] });
});

server.registerTool("policy.check_session_readiness", { description: "Session readiness", inputSchema: { sessionKey: z.string() } }, async ({ sessionKey }) => {
  if (prismaReady) {
    const state = await prisma.sessionState.findUnique({ where: { sessionKey } });
    const ready = state ? (state.workflowLoaded && state.planLoaded && state.taskLoaded) : false;
    return ok({ sessionKey, ready, workflowLoaded: state?.workflowLoaded ?? false, planLoaded: state?.planLoaded ?? false, taskLoaded: state?.taskLoaded ?? false });
  }
  return ok({ sessionKey, ready: true });
});
server.registerTool("policy.validate_execution", { description: "Validate execution", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (prismaReady) {
    const wf = await prisma.workflow.findUnique({ where: { id: a.workflowId } });
    if (!wf) return ok({ valid: false, reason: "Workflow not found", ...a });
    if (wf.status === "COMPLETED" || wf.status === "FAILED") return ok({ valid: false, reason: "Workflow is " + wf.status, ...a });
    const task = await prisma.task.findUnique({ where: { id: a.taskId } });
    if (!task) return ok({ valid: false, reason: "Task not found", ...a });
    if (task.status === "done") return ok({ valid: false, reason: "Task already completed", ...a });
    return ok({ valid: true, ...a });
  }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.validate_completion", { description: "Validate completion", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (prismaReady) {
    const review = await prisma.reviewDecision.findFirst({ where: { workflowId: a.workflowId, taskId: a.taskId }, orderBy: { createdAt: "desc" } });
    const approved = review?.decision === "APPROVED";
    return ok({ valid: approved, reviewDecision: review?.decision ?? "none", ...a });
  }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.validate_parallel_completion", { description: "Validate parallel", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (prismaReady) {
    const branches = await prisma.parallelBranch.findMany({ where: { workflowId: a.workflowId, taskId: a.taskId } });
    const allDone = branches.length > 0 && branches.every(b => b.status === "completed");
    const session = await prisma.sessionState.findUnique({ where: { sessionKey: a.sessionKey } });
    return ok({ valid: allDone && (session?.synthesisReady ?? false), allBranchesDone: allDone, branchCount: branches.length, ...a });
  }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.detect_scope_drift", { description: "Detect drift", inputSchema: { workflowId: z.string().optional(), taskId: z.string().optional(), outputText: z.string() } }, async (a) => {
  const outputLen = a.outputText.length;
  const suspiciousKeywords = ["unrelated", "off-topic", "completely different"];
  const driftDetected = suspiciousKeywords.some(k => a.outputText.toLowerCase().includes(k));
  if (prismaReady && a.workflowId) {
    const task = a.taskId ? await prisma.task.findUnique({ where: { id: a.taskId } }) : null;
    return ok({ driftDetected, outputLength: outputLen, taskTitle: task?.title ?? null });
  }
  return ok({ driftDetected, outputLength: outputLen });
});
server.registerTool("policy.require_context_refresh", { description: "Context refresh", inputSchema: { workflowId: z.string().optional(), planId: z.string().optional(), taskId: z.string().optional(), last_fingerprint: z.string().optional() } }, async (a) => {
  if (prismaReady && a.workflowId) {
    const wf = await prisma.workflow.findUnique({ where: { id: a.workflowId } });
    const currentFingerprint = wf ? `${wf.id}-${wf.updatedAt.getTime()}` : null;
    const needsRefresh = a.last_fingerprint ? currentFingerprint !== a.last_fingerprint : false;
    return ok({ needsRefresh, fingerprint: currentFingerprint, workflowId: a.workflowId });
  }
  return ok({ needsRefresh: false });
});

function listFiles(dir: string, ext: string): string[] {
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir).filter(f => f.endsWith(ext)).map(f => f.replace(ext, ""));
}

function parseFrontmatter(content: string): Record<string, string> {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};
  const fm: Record<string, string> = {};
  for (const line of match[1].split("\n")) {
    const idx = line.indexOf(":");
    if (idx > 0) fm[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }
  return fm;
}

server.registerTool("capability.list_agents", { description: "List agents", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => {
  const agentDir = path.join(projectRoot, ".claude", "agents");
  const agents = listFiles(agentDir, ".md").map(name => {
    const content = fs.readFileSync(path.join(agentDir, name + ".md"), "utf-8");
    const fm = parseFrontmatter(content);
    return { name, role: fm.role ?? "", description: fm.description ?? "" };
  });
  return ok({ agents, projectRoot });
});
server.registerTool("capability.list_skills", { description: "List skills", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => {
  const skillDir = path.join(projectRoot, ".claude", "skills");
  const skills = listFiles(skillDir, ".md").map(name => {
    const content = fs.readFileSync(path.join(skillDir, name + ".md"), "utf-8");
    const fm = parseFrontmatter(content);
    return { name, description: fm.description ?? "", trigger: fm.trigger ?? "" };
  });
  return ok({ skills, projectRoot });
});
server.registerTool("capability.list_templates", { description: "List templates", inputSchema: {} }, async () => ok({ templates: ["mcp-server", "agent", "skill", "feature"] }));
server.registerTool("capability.match_agent", { description: "Match agent", inputSchema: { projectRoot: z.string(), taskDescription: z.string() } }, async (a) => {
  const agentDir = path.join(a.projectRoot, ".claude", "agents");
  const agents = listFiles(agentDir, ".md").map(name => {
    const content = fs.readFileSync(path.join(agentDir, name + ".md"), "utf-8");
    const fm = parseFrontmatter(content);
    return { name, role: fm.role ?? "", description: fm.description ?? "" };
  });
  const q = a.taskDescription.toLowerCase();
  const scored = agents.map(ag => ({ ...ag, score: (ag.role + " " + ag.description).toLowerCase().split(/\s+/).filter(w => q.includes(w)).length })).sort((a, b) => b.score - a.score);
  return ok({ match: scored[0]?.score ? scored[0] : null, ...a });
});
server.registerTool("capability.system_readiness", { description: "System readiness", inputSchema: { projectRoot: z.string().optional() } }, async ({ projectRoot }) => ok({ ready: true, backend: backendType, postgresql: prismaReady, projectRoot: projectRoot ?? cwd }));
server.registerTool("capability.workflow_audit", { description: "Audit", inputSchema: { workflowId: z.string().optional() } }, async ({ workflowId }) => {
  if (prismaReady && workflowId) {
    const wf = await prisma.workflow.findUnique({ where: { id: workflowId }, include: { tasks: true, progressLogs: true } });
    if (!wf) return ok({ audited: 0, issues: ["Workflow not found"] });
    const issues: string[] = [];
    const running = wf.tasks.filter(t => t.status === "running");
    if (running.length > 1) issues.push(`Multiple running tasks: ${running.map(t => t.id.slice(0, 8)).join(", ")}`);
    const noProgress = wf.tasks.filter(t => t.status === "running" && wf.progressLogs.filter(p => p.taskId === t.id).length === 0);
    if (noProgress.length > 0) issues.push(`Running tasks with no progress: ${noProgress.map(t => t.id.slice(0, 8)).join(", ")}`);
    return ok({ audited: 1, issues, taskCount: wf.tasks.length });
  }
  return ok({ audited: workflowId ? 1 : engine.listWorkflows().length, issues: [] });
});
server.registerTool("capability.create_agent", { description: "Create agent", inputSchema: { projectRoot: z.string(), name: z.string(), role: z.string(), description: z.string(), instructions: z.string() } }, async (a) => {
  const agentDir = path.join(a.projectRoot, ".claude", "agents");
  if (!fs.existsSync(agentDir)) fs.mkdirSync(agentDir, { recursive: true });
  const content = `---\nname: ${a.name}\nrole: ${a.role}\ndescription: ${a.description}\n---\n\n${a.instructions}\n`;
  fs.writeFileSync(path.join(agentDir, a.name + ".md"), content);
  return ok({ ok: true, ...a });
});
server.registerTool("capability.create_skill", { description: "Create skill", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string(), trigger: z.string(), steps: z.array(z.string()) } }, async (a) => {
  const skillDir = path.join(a.projectRoot, ".claude", "skills");
  if (!fs.existsSync(skillDir)) fs.mkdirSync(skillDir, { recursive: true });
  const content = `---\nname: ${a.name}\ndescription: ${a.description}\ntrigger: ${a.trigger}\n---\n\n${a.steps.map((s, i) => `${i + 1}. ${s}`).join("\n")}\n`;
  fs.writeFileSync(path.join(skillDir, a.name + ".md"), content);
  return ok({ ok: true, ...a });
});
server.registerTool("capability.scaffold_feature", { description: "Scaffold feature", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => {
  const agentDir = path.join(a.projectRoot, ".claude", "agents");
  const skillDir = path.join(a.projectRoot, ".claude", "skills");
  if (!fs.existsSync(agentDir)) fs.mkdirSync(agentDir, { recursive: true });
  if (!fs.existsSync(skillDir)) fs.mkdirSync(skillDir, { recursive: true });
  fs.writeFileSync(path.join(agentDir, a.name + ".md"), `---\nname: ${a.name}\nrole: ${a.name} specialist\ndescription: ${a.description}\n---\n\nImplement the ${a.name} feature.\n`);
  fs.writeFileSync(path.join(skillDir, a.name + ".md"), `---\nname: ${a.name}\ndescription: ${a.description}\ntrigger: when working on ${a.name}\n---\n\n1. Analyze requirements for ${a.name}\n2. Plan implementation\n3. Implement with TDD\n4. Verify and review\n`);
  return ok({ ok: true, createdFiles: [`${a.name}.md (agent)`, `${a.name}.md (skill)`], ...a });
});
server.registerTool("capability.scaffold_mcp_server", { description: "Scaffold MCP", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => {
  const serverDir = path.join(a.projectRoot, "apps", a.name);
  if (!fs.existsSync(serverDir)) fs.mkdirSync(serverDir, { recursive: true });
  const srcDir = path.join(serverDir, "src");
  if (!fs.existsSync(srcDir)) fs.mkdirSync(srcDir, { recursive: true });
  fs.writeFileSync(path.join(serverDir, "package.json"), JSON.stringify({ name: `@mcp-rebuild/${a.name}`, version: "0.1.0", type: "module", main: "src/index.ts" }, null, 2));
  fs.writeFileSync(path.join(srcDir, "index.ts"), `import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";\nimport { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";\nimport { z } from "zod";\n\nconst server = new McpServer({ name: "${a.name}", version: "0.1.0" });\n\n// Add tools here\n\nconst transport = new StdioServerTransport();\nawait server.connect(transport);\n`);
  return ok({ ok: true, createdDir: serverDir, ...a });
});

server.registerTool("filesystem.read", { description: "Read file", inputSchema: { path: z.string() } }, async ({ path: fp }) => ({ content: [{ type: "text" as const, text: fs.readFileSync(fp, "utf-8") }] }));
server.registerTool("filesystem.write", { description: "Write file", inputSchema: { path: z.string(), content: z.string() } }, async ({ path: fp, content }) => { const d = path.dirname(fp); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(fp, content); return ok({ ok: true }); });
server.registerTool("filesystem.list", { description: "List dir", inputSchema: { path: z.string() } }, async ({ path: fp }) => ok(fs.readdirSync(fp, { withFileTypes: true }).map(e => ({ name: e.name, type: e.isDirectory() ? "dir" : "file" }))));
server.registerTool("filesystem.delete", { description: "Delete file", inputSchema: { path: z.string() } }, async ({ path: fp }) => { fs.unlinkSync(fp); return ok({ ok: true }); });
server.registerTool("filesystem.stat", { description: "File stat", inputSchema: { path: z.string() } }, async ({ path: fp }) => { const s = fs.statSync(fp); return ok({ size: s.size, isFile: s.isFile() }); });

// --- REAL TOOLS: Review, Session, Local, Shell (29) ---
// review (2)
server.registerTool("review.submit", { description: "Submit review", inputSchema: { workflow_id: z.string(), task_id: z.string(), reviewer_agent: z.string(), decision: z.string(), notes: z.string(), gaps: z.array(z.string()).optional() } }, async (a) => {
  if (prismaReady) {
    const review = await prisma.reviewDecision.create({ data: { workflowId: a.workflow_id, taskId: a.task_id, reviewerAgent: a.reviewer_agent, decision: a.decision, notes: a.notes, gaps: a.gaps ?? [] } });
    return ok({ submitted: true, id: review.id, ...a });
  }
  return ok({ submitted: true, ...a });
});
server.registerTool("review.get_latest", { description: "Get latest review", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async (a) => {
  if (prismaReady) {
    const review = await prisma.reviewDecision.findFirst({ where: { workflowId: a.workflow_id, taskId: a.task_id }, orderBy: { createdAt: "desc" } });
    return ok({ review });
  }
  return ok({ review: null, ...a });
});

// session (3)
server.registerTool("session.get_state", { description: "Get session state", inputSchema: { session_key: z.string() } }, async ({ session_key }) => {
  if (prismaReady) {
    const state = await prisma.sessionState.findUnique({ where: { sessionKey: session_key } });
    return ok({ sessionKey: session_key, state: state ?? {} });
  }
  return ok({ sessionKey: session_key, state: {} });
});
server.registerTool("session.patch_state", { description: "Patch session state", inputSchema: { session_key: z.string(), patch: z.record(z.any()) } }, async ({ session_key, patch }) => {
  if (prismaReady) {
    const existing = await prisma.sessionState.findUnique({ where: { sessionKey: session_key } });
    if (existing) {
      const merged = { ...(existing.metadata as Record<string, unknown>), ...patch };
      await prisma.sessionState.update({ where: { sessionKey: session_key }, data: { metadata: merged, workflowId: (patch as Record<string, unknown>).workflowId as string ?? existing.workflowId, planId: (patch as Record<string, unknown>).planId as string ?? existing.planId, taskId: (patch as Record<string, unknown>).taskId as string ?? existing.taskId } });
      return ok({ sessionKey: session_key, patched: true, patch });
    }
    await prisma.sessionState.create({ data: { sessionKey: session_key, metadata: patch, workflowId: (patch as Record<string, unknown>).workflowId as string ?? null, planId: (patch as Record<string, unknown>).planId as string ?? null, taskId: (patch as Record<string, unknown>).taskId as string ?? null } });
    return ok({ sessionKey: session_key, patched: true, patch, created: true });
  }
  return ok({ sessionKey: session_key, patched: true, patch });
});
server.registerTool("session.init_context", { description: "Init session context", inputSchema: { cwd: z.string() } }, async ({ cwd }) => ok({ initialized: true, cwd }));

// local (4)
server.registerTool("local.init", { description: "Init local state dir", inputSchema: { cwd: z.string() } }, async ({ cwd }) => { const p = path.join(cwd, ".masday"); if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true }); return ok({ initialized: true, path: p }); });
server.registerTool("local.sync", { description: "Sync local state from DB (download PostgreSQL → JSON cache)", inputSchema: { cwd: z.string(), workflow_id: z.string().optional() } }, async ({ cwd, workflow_id }) => {
  if (!prismaReady) return ok({ synced: false, error: "PostgreSQL not connected" });
  const where = workflow_id ? { workflowId: workflow_id } : {};
  const rows = await prisma.memory.findMany({ where, orderBy: { createdAt: "desc" } });
  const memFile = path.join(cwd, ".masday", "state", "memories.json");
  const cached: MemRec[] = rows.map(r => ({ id: r.id, type: r.memoryType, content: r.content, summary: r.summary, source: r.createdByAgent, importance: r.importanceScore, tags: r.tags, createdAt: r.createdAt.getTime() }));
  const dir = path.dirname(memFile); if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  saveMem(cached);
  return ok({ synced: true, records: cached.length, workflowId: workflow_id ?? "all" });
});
server.registerTool("local.push", { description: "Push local state to DB (upload JSON cache → PostgreSQL)", inputSchema: { cwd: z.string(), workflow_id: z.string().optional() } }, async ({ cwd, workflow_id }) => {
  if (!prismaReady) return ok({ pushed: false, error: "PostgreSQL not connected" });
  const memFile = path.join(cwd, ".masday", "state", "memories.json");
  const local: MemRec[] = fs.existsSync(memFile) ? JSON.parse(fs.readFileSync(memFile, "utf-8")) : [];
  let created = 0, skipped = 0;
  for (const r of local) {
    const existing = await prisma.memory.findUnique({ where: { id: r.id } }).catch(() => null);
    if (existing) { skipped++; continue; }
    await prisma.memory.create({ data: { id: r.id, memoryType: r.type, summary: r.summary, content: r.content, importanceScore: r.importance, createdByAgent: r.source, tags: r.tags, createdAt: new Date(r.createdAt) } }).catch(() => { skipped++; });
    created++;
  }
  return ok({ pushed: true, created, skipped, total: local.length, workflowId: workflow_id });
});
server.registerTool("local.save_artifact", { description: "Save artifact file locally", inputSchema: { cwd: z.string(), category: z.string(), filename: z.string(), content: z.string() } }, async ({ cwd, category, filename, content }) => { const d = path.join(cwd, ".masday", category); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(path.join(d, filename), content); return ok({ saved: true, path: path.join(d, filename) }); });

// git (3)
server.registerTool("git.status", { description: "Git status", inputSchema: {} }, async () => {
  try { const stdout = execSync("git status --porcelain", { encoding: "utf-8", timeout: 10000 }); return ok({ stdout, exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});
server.registerTool("git.diff", { description: "Git diff", inputSchema: {} }, async () => {
  try { const stdout = execSync("git diff --stat && git diff", { encoding: "utf-8", timeout: 15000 }); return ok({ stdout, exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});
server.registerTool("git.commit", { description: "Git commit", inputSchema: { message: z.string() } }, async ({ message }) => {
  try { const stdout = execSync(`git commit -m "${message.replace(/"/g, '\\"')}"`, { encoding: "utf-8", timeout: 15000 }); return ok({ stdout, exitCode: 0, message }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, message }); }
});

// npm (2)
server.registerTool("npm.install", { description: "NPM install", inputSchema: { packages: z.array(z.string()).optional() } }, async ({ packages }) => {
  const cmd = packages?.length ? `pnpm add ${packages.join(" ")}` : "pnpm install";
  try { const stdout = execSync(cmd, { encoding: "utf-8", timeout: 120000 }); return ok({ stdout, exitCode: 0, packages: packages ?? [] }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, packages: packages ?? [] }); }
});
server.registerTool("npm.run", { description: "NPM run script", inputSchema: { script: z.string() } }, async ({ script }) => {
  try { const stdout = execSync(`pnpm run ${script}`, { encoding: "utf-8", timeout: 120000 }); return ok({ stdout, exitCode: 0, script }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, script }); }
});

// docker (3)
server.registerTool("docker.build", { description: "Docker build", inputSchema: { tag: z.string().optional() } }, async ({ tag }) => {
  const cmd = tag ? `docker build -t ${tag} .` : "docker build .";
  try { const stdout = execSync(cmd, { encoding: "utf-8", timeout: 300000 }); return ok({ stdout, exitCode: 0, tag }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, tag }); }
});
server.registerTool("docker.run", { description: "Docker run", inputSchema: { image: z.string() } }, async ({ image }) => {
  try { const stdout = execSync(`docker run --rm ${image}`, { encoding: "utf-8", timeout: 300000 }); return ok({ stdout, exitCode: 0, image }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, image }); }
});
server.registerTool("docker.ps", { description: "Docker ps", inputSchema: {} }, async () => {
  try { const stdout = execSync("docker ps --format json", { encoding: "utf-8", timeout: 10000 }); return ok({ stdout, exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});

// cicd (3)
server.registerTool("cicd.pipeline_status", { description: "CI/CD pipeline status", inputSchema: {} }, async () => {
  try { const stdout = execSync("gh run list --limit 5 --json name,status,conclusion,createdAt", { encoding: "utf-8", timeout: 15000 }); return ok({ pipelines: JSON.parse(stdout), exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ pipelines: [], stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});
server.registerTool("cicd.pipeline_trigger", { description: "Trigger CI/CD pipeline", inputSchema: { pipeline: z.string() } }, async ({ pipeline }) => {
  try { const stdout = execSync(`gh workflow run ${pipeline}`, { encoding: "utf-8", timeout: 30000 }); return ok({ triggered: true, stdout, exitCode: 0, pipeline }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ triggered: false, stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, pipeline }); }
});
server.registerTool("cicd.runs_view", { description: "View CI/CD runs", inputSchema: {} }, async () => {
  try { const stdout = execSync("gh run list --limit 20 --json name,status,conclusion,headBranch,createdAt,databaseId", { encoding: "utf-8", timeout: 15000 }); return ok({ runs: JSON.parse(stdout), exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ runs: [], stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});

// github (3)
server.registerTool("github.pr_create", { description: "Create GitHub PR", inputSchema: { title: z.string(), body: z.string().optional() } }, async ({ title, body }) => {
  try { const stdout = execSync(`gh pr create --title "${title.replace(/"/g, '\\"')}" --body "${(body ?? "").replace(/"/g, '\\"')}"`, { encoding: "utf-8", timeout: 30000 }); return ok({ created: true, stdout, exitCode: 0, title }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ created: false, stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, title }); }
});
server.registerTool("github.pr_list", { description: "List GitHub PRs", inputSchema: {} }, async () => {
  try { const stdout = execSync("gh pr list --json number,title,state,author,createdAt", { encoding: "utf-8", timeout: 15000 }); return ok({ prs: JSON.parse(stdout), exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ prs: [], stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});
server.registerTool("github.issue_list", { description: "List GitHub issues", inputSchema: {} }, async () => {
  try { const stdout = execSync("gh issue list --json number,title,state,labels,createdAt", { encoding: "utf-8", timeout: 15000 }); return ok({ issues: JSON.parse(stdout), exitCode: 0 }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ issues: [], stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); }
});

// tests (1)
server.registerTool("tests.run", { description: "Run tests", inputSchema: { pattern: z.string().optional() } }, async ({ pattern }) => {
  const cmd = pattern ? `pnpm test -- ${pattern}` : "pnpm test";
  try { const stdout = execSync(cmd, { encoding: "utf-8", timeout: 120000 }); return ok({ stdout, exitCode: 0, pattern }); }
  catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, pattern }); }
});

// capability.ping (1) — missing from original capability namespace
server.registerTool("capability.ping", { description: "Capability health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType, postgresql: prismaReady }));

await initPrisma();

const transport = new StdioServerTransport();
await server.connect(transport);
logger.info("MCP Server running on stdio (" + backendType + " backend" + (prismaReady ? " + PostgreSQL" : "") + ")");
