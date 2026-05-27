#!/usr/bin/env node
// Suppress MaxListenersExceededWarning — DualWriteStore + PersistenceListener
// register multiple exit handlers which triggers the default limit of 10.
process.setMaxListeners(20);

import { config } from "dotenv";
import { fileURLToPath } from "url";
import * as path from "path";
config({ path: path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", ".env") });

/**
 * Masday Workflow MCP Server — HTTP/SSE + STDIO dual-mode entry point
 *
 * Modes (via MCP_TRANSPORT env var):
 *   "stdio"  — traditional STDIO transport (default for CLI usage)
 *   "http"   — HTTP Streamable transport with SSE support (for remote/dashboard)
 *
 * HTTP mode endpoints:
 *   POST /mcp          — JSON-RPC requests (tool calls)
 *   GET  /mcp          — SSE stream for server-initiated notifications
 *   DELETE /mcp        — Close session
 *   GET  /health       — Health check
 *
 * This file contains the same tools as mcp.ts but with dual transport support.
 * Shell tools (git, docker, gh) intentionally use execSync for CLI integration.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { createServer } from "node:http";
import { randomUUID } from "node:crypto";
import { z } from "zod";
import { eq, and, or, desc, asc, count, sql, inArray } from "drizzle-orm";
import { EventBus, createLogger, setDrizzleDb as setTokenDb, trackTokens } from "@mcp-rebuild/core";
import { JsonBackend, SqliteBackend, WorkflowStore, TaskResultStore, PersistenceListener, DualWriteWorkflowStore, setDualWriteDb } from "@mcp-rebuild/store";
import { OrchestratingEngine, saveProgress as saveProgressDb, logRetrieval, setReminderDb, checkReminders, listReminders as listRemindersDb, acknowledgeReminder, dismissWorkflowReminders, reminderStats, makeFingerprint } from "@mcp-rebuild/workflow-engine";
import { buildHybridContextPack, computeFingerprint } from "@mcp-rebuild/intelligence";
import { setEpisodicDb, setGraphDb, EpisodicMemory, GraphStore } from "@mcp-rebuild/memory";
import type { ISkillRegistry } from "@mcp-rebuild/workflow-engine";
import { db as drizzleDb, healthCheck as dbHealthCheck, memories as memoriesTable, contextDocuments as contextDocsTable, tasks as tasksTable, workflows as workflowsTable, plans as plansTable, taskProgressLogs, reviewDecisions as reviewDecisionsTable, sessionStates, parallelBranches as parallelBranchesTable, graphNodes as graphNodesTable, graphEdges as graphEdgesTable, workflowReminders as workflowRemindersTable } from "@mcp-rebuild/db";
import * as fs from "fs";
import { dotToUnderscore, ToolNameRegistry, createAgent, createSkill, runDoctor } from "@mcp-rebuild/shared-utils";

import { execFileSync } from "child_process";

const SAFE_SCRIPT_RE = /^[a-zA-Z0-9_-]+$/;
const SAFE_IMAGE_RE = /^[a-zA-Z0-9._:/-]+$/;
const SAFE_PIPELINE_RE = /^[a-zA-Z0-9_.-]+$/;
const SAFE_PACKAGE_RE = /^(@[a-zA-Z0-9_-]+\/)?[a-zA-Z0-9_.-]+$/;
const SAFE_PATTERN_RE = /^[a-zA-Z0-9_./\\*-]+$/;

const logger = createLogger("MCPHTTPServer");
process.setMaxListeners(process.getMaxListeners() + 20);
const toolNameRegistry = new ToolNameRegistry();
const TRANSPORT_MODE = (process.env.MCP_TRANSPORT ?? "stdio").toLowerCase();
const HTTP_PORT = parseInt(process.env.MCP_HTTP_PORT ?? process.env.API_PORT ?? "3100", 10);

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
const episodicMemory = new EpisodicMemory(100);
const graphStore = new GraphStore({ autoLinkThreshold: 0.3 });

// Wrap registerTool: register ONLY underscore names (universal provider compatibility)
const origRegister = server.registerTool.bind(server);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(server as any).registerTool = function(name: string, schema: any, handler: (args: any) => Promise<any>) {
  const canonicalName = dotToUnderscore(name);
  const wrappedHandler = async (args: any) => {
    episodicMemory.add("user", `[${canonicalName}] ${JSON.stringify(args).substring(0, 500)}`);
    const result = await handler(args);
    const resultText = typeof result === "object" && result !== null && "content" in result
      ? JSON.stringify((result as { content: Array<{ text: string }> }).content).substring(0, 500)
      : String(result).substring(0, 500);
    episodicMemory.add("assistant", `[${canonicalName}] ${resultText}`);
    return result;
  };
  toolNameRegistry.register(canonicalName);
  return origRegister(canonicalName, schema, wrappedHandler);
};

const cwd = process.cwd();
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDir, "..", "..", "..");

function safePath(input: string | undefined, fallback: string = cwd): string {
  if (!input || typeof input !== "string" || input.trim() === "") return fallback;
  const resolved = path.resolve(input);
  if (!fs.existsSync(resolved)) return fallback;
  if (!resolved.includes(".masday") && !resolved.includes(".claude") && !fs.existsSync(path.join(resolved, "package.json"))) return fallback;
  return resolved;
}

const dataDir = path.join(projectRoot, ".masday", "state");
if (!fs.existsSync(dataDir)) fs.mkdirSync(dataDir, { recursive: true });
const { backend, type: backendType } = createBackend(dataDir);
logger.info("Storage: " + backendType);

const eventBus = new EventBus();
const primaryStore = new WorkflowStore(backend);
const workflowStore = new DualWriteWorkflowStore(primaryStore);
const persistenceListener = new PersistenceListener(eventBus, primaryStore, new TaskResultStore(backend));
persistenceListener.start();

const engine = new OrchestratingEngine(skillRegistry, eventBus, { coordinator: false, enableSkillRouting: false, store: workflowStore });
const orphans = engine.restoreWorkflows(workflowStore.loadAll());
if (orphans > 0) logger.info("Restored " + orphans + " orphaned workflows");

// Memory file cache
const MEMORY_FILE = path.join(dataDir, "memories.json");
interface MemRec { id: string; type: string; content: string; summary: string; source: string; importance: number; tags: string[]; createdAt: number }
function loadMem(): MemRec[] { return fs.existsSync(MEMORY_FILE) ? JSON.parse(fs.readFileSync(MEMORY_FILE, "utf-8")) : []; }
function saveMem(m: MemRec[]) { fs.writeFileSync(MEMORY_FILE, JSON.stringify(m, null, 2)); }
let mic = 0;
function nid() { return "mem_" + Date.now() + "_" + (++mic); }
function ok(d: unknown) { return { content: [{ type: "text" as const, text: JSON.stringify(d) }] }; }

// Embedding config
const MCP_API_KEY = process.env.MCP_API_KEY ?? "";
const EMBEDDING_PROVIDER = (process.env.EMBEDDING_PROVIDER ?? "fastembed").toLowerCase();
const EMBEDDING_MODEL = process.env.EMBEDDING_MODEL ?? "";
const OLLAMA_BASE_URL = (process.env.OLLAMA_BASE_URL ?? "http://localhost:11434").replace(/\/$/, "");
const OPENAI_API_KEY = process.env.OPENAI_API_KEY ?? "";
const OPENAI_BASE_URL = (process.env.OPENAI_BASE_URL ?? "https://api.openai.com").replace(/\/$/, "");

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let _embedModel: any = null;
async function getFastembedModel() {
  if (!_embedModel) {
    const { createRequire } = await import("node:module");
    const require = createRequire(import.meta.url);
    const { FlagEmbedding, EmbeddingModel } = require("fastembed");
    const modelId = (EMBEDDING_MODEL as any) || EmbeddingModel.BGEBaseENV15;
    const cacheDir = path.resolve(import.meta.dirname, "../../../local_cache");
    _embedModel = await FlagEmbedding.init({ model: modelId, cacheDir });
  }
  return _embedModel;
}

function mockEmbedding(text: string, dims: number): number[] {
  const vec = new Array(dims);
  let hash = 0;
  for (let i = 0; i < text.length; i++) hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
  for (let i = 0; i < dims; i++) vec[i] = Math.sin(hash * (i + 1) * 0.0001);
  const mag = Math.sqrt(vec.reduce((s, v) => s + v * v, 0));
  return vec.map(v => v / mag);
}

async function generateEmbedding(text: string): Promise<number[] | null> {
  try {
    if (EMBEDDING_PROVIDER === "mock") {
      const dims = parseInt(process.env.EMBEDDING_DIMENSIONS ?? "768", 10);
      return mockEmbedding(text, dims);
    }
    if (EMBEDDING_PROVIDER === "ollama") {
      const model = EMBEDDING_MODEL || "nomic-embed-text";
      const res = await fetch(`${OLLAMA_BASE_URL}/api/embeddings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model, prompt: text }),
      });
      if (!res.ok) return null;
      const json = await res.json() as { embedding?: number[] };
      return json.embedding ?? null;
    }
    if (EMBEDDING_PROVIDER === "openai") {
      if (!OPENAI_API_KEY) return null;
      const model = EMBEDDING_MODEL || "text-embedding-3-small";
      const res = await fetch(`${OPENAI_BASE_URL}/v1/embeddings`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${OPENAI_API_KEY}` },
        body: JSON.stringify({ model, input: text }),
      });
      if (!res.ok) return null;
      const json = await res.json() as { data?: Array<{ embedding: number[] }> };
      return json.data?.[0]?.embedding ?? null;
    }
    const model = await getFastembedModel();
    return await model.queryEmbed(text);
  } catch { return null; }
}

let dbReady = false;
async function initDb(): Promise<void> {
  try {
    const healthy = await dbHealthCheck();
    if (!healthy) { logger.warn("PostgreSQL not reachable, using JSON-only mode"); return; }
    const rows = await drizzleDb.select().from(memoriesTable).orderBy(desc(memoriesTable.createdAt));
    if (rows.length > 0) {
      const cached: MemRec[] = rows.map(r => ({
        id: r.id, type: r.memoryType, content: r.content, summary: r.summary,
        source: r.createdByAgent, importance: r.importanceScore ?? 0.5,
        tags: r.tags, createdAt: r.createdAt.getTime(),
      }));
      saveMem(cached);
      logger.info("Synced " + cached.length + " memories from PostgreSQL to cache");
    }
    dbReady = true;
    setDualWriteDb(drizzleDb as never);
    setTokenDb(drizzleDb as never);
    setEpisodicDb(drizzleDb as never);
    setGraphDb(drizzleDb as never);
    setReminderDb(drizzleDb);
    logger.info("Drizzle connected — hybrid mode active (DualWriteStore + TokenUsage + EpisodicMemory + GraphStore + Reminders enabled)");
  } catch (err) {
    logger.warn("Drizzle init failed, falling back to JSON-only: " + (err instanceof Error ? err.message : String(err)));
  }
}

async function persistToDb(rec: MemRec, workflowId?: string, taskId?: string): Promise<void> {
  if (!dbReady) return;
  try {
    const embedding = await generateEmbedding(rec.content);
    const [existing] = await drizzleDb.select({ id: memoriesTable.id }).from(memoriesTable).where(eq(memoriesTable.id, rec.id)).limit(1);
    if (existing) {
      await drizzleDb.update(memoriesTable).set({
        content: rec.content, summary: rec.summary,
        importanceScore: rec.importance, tags: rec.tags,
        accessedAt: new Date(),
      }).where(eq(memoriesTable.id, rec.id));
    } else {
      await drizzleDb.insert(memoriesTable).values({
        id: rec.id, memoryType: rec.type, content: rec.content, summary: rec.summary,
        importanceScore: rec.importance, tags: rec.tags, createdByAgent: rec.source,
        workflowId: workflowId ?? null, taskId: taskId ?? null,
      });
    }
    if (embedding) {
      const vecStr = `[${embedding.join(",")}]`;
      await drizzleDb.execute(sql`UPDATE "Memory" SET embedding = ${vecStr}::vector WHERE id = ${rec.id}`);
    }
  } catch (err) {
    logger.warn("Drizzle write failed: " + (err instanceof Error ? err.message : String(err)));
  }
}

// ── ALL TOOL REGISTRATIONS ──
// Identical tool set to mcp.ts — see that file for full documentation

server.registerTool("workflow.create", { description: "Create workflow", inputSchema: { name: z.string(), description: z.string().optional(), metadata: z.record(z.any()).optional() } }, async ({ name, description, metadata }) => { const w = engine.createWorkflow(name, description ?? "", metadata); workflowStore.save(w); try { graphStore.addNode({ type: "workflow", label: name, properties: { workflowId: w.id, description: description ?? "" } }); } catch { /* non-critical */ } return ok(w); });
server.registerTool("workflow.execute", { description: "Execute workflow", inputSchema: { id: z.string() } }, async ({ id }) => {
  const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id);
  if (w.state === "INIT") { w.state = "ANALYZE"; w.updatedAt = new Date(); }
  if (w.state === "ANALYZE") { w.state = "PLAN"; w.updatedAt = new Date(); }
  if (w.state === "PLAN" || w.state === "PAUSED" || w.state === "FIX") { w.state = "EXECUTE"; w.updatedAt = new Date(); }
  workflowStore.save(w); return ok(w);
});
server.registerTool("workflow.getStatus", { description: "Get workflow status", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.get", { description: "Get workflow by ID", inputSchema: { id: z.string() } }, async ({ id }) => { const w = engine.getWorkflow(id); if (!w) throw new Error("Not found: " + id); return ok(w); });
server.registerTool("workflow.list", { description: "List workflows", inputSchema: {} }, async () => ok(engine.listWorkflows()));
server.registerTool("workflow.addTask", { description: "Add task", inputSchema: { workflowId: z.string(), name: z.string(), agent: z.string(), skill: z.string(), dependencies: z.array(z.string()).optional(), input: z.record(z.any()).optional(), requires_tdd: z.boolean().optional() } }, async (a) => { const t = engine.addTask(a.workflowId, { name: a.name, agent: a.agent, skill: a.skill, dependencies: a.dependencies ?? [], input: a.input ?? {} }); try { const node = graphStore.addNode({ type: "task", label: a.name, properties: { taskId: t.id, workflowId: a.workflowId, agent: a.agent, skill: a.skill } }); const wfNodes = graphStore.findNodes(n => n.properties.workflowId === a.workflowId && n.type === "workflow"); if (wfNodes.length > 0 && node) graphStore.addEdge({ from: wfNodes[0].id, to: node.id, relation: "contains", weight: 1.0 }); } catch { /* non-critical */ } return ok(t); });
server.registerTool("workflow.startTask", { description: "Start task", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async ({ workflow_id, task_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "running"; t.startedAt = new Date(); workflowStore.save(w); return ok(t); });
server.registerTool("workflow.completeTask", { description: "Complete task", inputSchema: { workflow_id: z.string(), task_id: z.string(), result: z.record(z.any()).optional() } }, async ({ workflow_id, task_id, result }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); const t = w.tasks.find((x: any) => x.id === task_id); if (!t) throw new Error("Task not found"); t.state = "done"; t.completedAt = new Date(); if (result) t.output = result; const allDone = w.tasks.length > 0 && w.tasks.every((x: any) => x.state === "done"); if (allDone && w.state !== "DONE") { w.state = "DONE"; w.updatedAt = new Date(); } workflowStore.save(w); return ok(t); });
server.registerTool("workflow.saveProgress", { description: "Save progress", inputSchema: { workflow_id: z.string(), task_id: z.string(), agent_name: z.string(), progress_note: z.string(), evidence: z.array(z.string()).optional(), test_evidence: z.object({ testFiles: z.array(z.string()), testsPassed: z.boolean(), coveragePercent: z.number().optional(), testOutput: z.string().optional() }).optional() } }, async (a) => {
  if (dbReady) { try { await saveProgressDb({ workflowId: a.workflow_id, taskId: a.task_id, agentName: a.agent_name, progressNote: a.progress_note, evidence: a.evidence ?? [] }); } catch (e) { logger.warn("TaskProgressLog write failed: " + (e instanceof Error ? e.message : String(e))); } }
  if (dbReady && a.test_evidence) { try { await drizzleDb.update(tasksTable).set({ testEvidence: a.test_evidence as never }).where(eq(tasksTable.id, a.task_id)); } catch (e) { logger.warn("testEvidence update failed: " + (e instanceof Error ? e.message : String(e))); } }
  eventBus.emit("trace.completed", { workflowId: a.workflow_id, taskId: a.task_id, agentName: a.agent_name, progressNote: a.progress_note, evidence: a.evidence ?? [] });
  trackTokens("workflow.saveProgress", a, { saved: true }); return ok({ saved: true });
});
server.registerTool("workflow.listTasks", { description: "List tasks", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks); });
server.registerTool("workflow.getCurrentTask", { description: "Current task", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok(w.tasks.find((x: any) => x.state === "running") ?? w.tasks.find((x: any) => x.state === "pending") ?? null); });
server.registerTool("workflow.getPlan", { description: "Get plan", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); return ok({ workflowId: w.id, name: w.name, state: w.state, tasks: w.tasks, metadata: w.metadata }); });
server.registerTool("workflow.getActive", { description: "Active workflow", inputSchema: { cwd: z.string().optional() } }, async () => { const a = engine.listWorkflows().filter((w: any) => ["EXECUTE","PLAN","VERIFY"].includes(w.state)); return ok(a[0] ?? null); });
server.registerTool("workflow.createPlan", { description: "Create plan (stores metadata only — use workflow.addTask to create tasks)", inputSchema: { workflow_id: z.string(), plan: z.record(z.any()) } }, async ({ workflow_id, plan }) => { const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found"); w.metadata = { ...w.metadata, plan }; workflowStore.save(w); return ok({ created: true, planStored: true, tasksInPlan: Array.isArray((plan as any)?.tasks) ? (plan as any).tasks.length : 0, note: "Plan stored. Call workflow.addTask for each task to instantiate them." }); });
server.registerTool("workflow.createParallelBranches", { description: "Create parallel branches", inputSchema: { workflow_id: z.string(), branches: z.array(z.object({ branchKey: z.string(), role: z.string(), scope: z.string() })) } }, async ({ workflow_id, branches }) => {
  const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found");
  w.metadata = { ...w.metadata, parallelBranches: branches }; workflowStore.save(w);
  if (dbReady) { try { for (const b of branches) {
    const branchId = `${workflow_id}_${b.branchKey}`;
    const [existing] = await drizzleDb.select({ id: parallelBranchesTable.id }).from(parallelBranchesTable).where(eq(parallelBranchesTable.id, branchId)).limit(1);
    if (existing) { await drizzleDb.update(parallelBranchesTable).set({ role: b.role, status: "ACTIVE", input: { scope: b.scope } as never }).where(eq(parallelBranchesTable.id, branchId)); }
    else { await drizzleDb.insert(parallelBranchesTable).values({ id: branchId, workflowId: workflow_id, taskId: "", branchKey: b.branchKey, role: b.role, status: "ACTIVE", input: { scope: b.scope } as never }); }
  } } catch (e) { logger.warn("ParallelBranch Drizzle write failed: " + (e instanceof Error ? e.message : String(e))); } }
  return ok({ created: true, branchCount: branches.length });
});
server.registerTool("workflow.completeParallelBranch", { description: "Complete branch", inputSchema: { workflow_id: z.string(), branch_key: z.string() } }, async ({ workflow_id, branch_key }) => {
  const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found");
  const b = ((w.metadata.parallelBranches ?? []) as any[]).find(x => x.branchKey === branch_key); if (b) b.completed = true; workflowStore.save(w);
  if (dbReady) { try { await drizzleDb.update(parallelBranchesTable).set({ status: "COMPLETED", output: { completedAt: new Date().toISOString() } as never }).where(and(eq(parallelBranchesTable.workflowId, workflow_id), eq(parallelBranchesTable.branchKey, branch_key))); } catch (e) { logger.warn("ParallelBranch complete Drizzle failed: " + (e instanceof Error ? e.message : String(e))); } }
  return ok({ completed: true });
});
server.registerTool("workflow.listParallelBranches", { description: "List branches", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => {
  const w = engine.getWorkflow(workflow_id); if (!w) throw new Error("Not found");
  if (dbReady) { try { const rows = await drizzleDb.select().from(parallelBranchesTable).where(eq(parallelBranchesTable.workflowId, workflow_id)); if (rows.length > 0) return ok(rows); } catch { /* fall through */ } }
  return ok(w.metadata.parallelBranches ?? []);
});
server.registerTool("workflow.delete", { description: "Delete workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ deleted: engine.deleteWorkflow(workflow_id) }));
server.registerTool("workflow.ping", { description: "Health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType, postgresql: dbReady, transport: TRANSPORT_MODE }));
server.registerTool("workflow.set_execution_mode", { description: "Set execution mode", inputSchema: { session_key: z.string(), mode: z.string() } }, async ({ session_key, mode }) => ok({ sessionKey: session_key, mode }));
server.registerTool("workflow.mark_synthesis_ready", { description: "Mark synthesis ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, synthesisReady: ready }));
server.registerTool("workflow.mark_verification_ready", { description: "Mark verification ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, verificationReady: ready }));
server.registerTool("workflow.resume_suggestion", { description: "Get resume suggestion", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ workflowId: workflow_id, suggestion: "continue" }));

// Memory
server.registerTool("memory.store", { description: "Store memory", inputSchema: { workflow_id: z.string().optional(), task_id: z.string().optional(), memory_type: z.string(), summary: z.string(), content: z.string(), created_by_agent: z.string(), importance_score: z.number().optional(), tags: z.array(z.string()).optional() } }, async (a) => {
  const r: MemRec = { id: nid(), type: a.memory_type, content: a.content, summary: a.summary, source: a.created_by_agent, importance: a.importance_score ?? 0.5, tags: [...(a.tags ?? []), a.workflow_id, a.task_id].filter(Boolean) as string[], createdAt: Date.now() };
  await persistToDb(r, a.workflow_id, a.task_id); const m = loadMem(); m.push(r); saveMem(m);
  try { graphStore.addNode({ type: "memory", label: a.summary, properties: { memoryId: r.id, memoryType: a.memory_type, tags: a.tags ?? [], workflowId: a.workflow_id, taskId: a.task_id, content: a.content.substring(0, 500) } }); } catch { /* non-critical */ }
  return ok(r);
});
server.registerTool("memory.store_research", { description: "Store research", inputSchema: { workflow_id: z.string().optional(), summary: z.string(), content: z.string(), created_by_agent: z.string() } }, async (a) => {
  const r: MemRec = { id: nid(), type: "research", content: a.content, summary: a.summary, source: a.created_by_agent, importance: 0.5, tags: ["research", a.workflow_id].filter(Boolean) as string[], createdAt: Date.now() };
  await persistToDb(r, a.workflow_id);
  if (dbReady) { try {
    const emb = await generateEmbedding(a.content);
    await drizzleDb.insert(contextDocsTable).values({ id: r.id, workflowId: a.workflow_id ?? null, sourceType: "research", title: a.summary, content: a.content, metadata: { agent: a.created_by_agent } as never });
    if (emb) { const vecStr = `[${emb.join(",")}]`; await drizzleDb.execute(sql`UPDATE "ContextDocument" SET embedding = ${vecStr}::vector WHERE id = ${r.id}`); }
  } catch (e) { logger.warn("ContextDocument write failed: " + (e instanceof Error ? e.message : String(e))); } }
  const m = loadMem(); m.push(r); saveMem(m);
  try { graphStore.addNode({ type: "research", label: a.summary, properties: { memoryId: r.id, workflowId: a.workflow_id, content: a.content.substring(0, 500) } }); } catch { /* non-critical */ }
  trackTokens("memory.store_research", a, r); return ok(r);
});
server.registerTool("memory.recall_recent", { description: "Recall recent", inputSchema: { limit: z.number().optional(), type: z.string().optional() } }, async ({ limit, type }) => {
  if (dbReady) { try { const whereCond = type ? eq(memoriesTable.memoryType, type) : undefined; const rows = await drizzleDb.select().from(memoriesTable).where(whereCond).orderBy(desc(memoriesTable.createdAt)).limit(limit ?? 10); const ids = rows.map(r => r.id); if (ids.length > 0) await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`); return ok(rows); } catch { /* fall through to cache */ } }
  let m = loadMem(); if (type) m = m.filter(x => x.type === type); return ok(m.sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_documents", { description: "Recall docs", inputSchema: { workflow_id: z.string(), limit: z.number().optional() } }, async ({ workflow_id, limit }) => {
  if (dbReady) { try { const rows = await drizzleDb.select().from(memoriesTable).where(eq(memoriesTable.workflowId, workflow_id)).orderBy(desc(memoriesTable.createdAt)).limit(limit ?? 10); const ids = rows.map(r => r.id); if (ids.length > 0) await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`); return ok(rows); } catch { /* fall through to cache */ } }
  return ok(loadMem().filter(m => m.tags.includes(workflow_id)).sort((a, b) => b.createdAt - a.createdAt).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_document_by_type", { description: "Recall by type", inputSchema: { workflow_id: z.string(), source_type: z.string(), limit: z.number().optional() } }, async ({ workflow_id, source_type, limit }) => {
  if (dbReady) { try { const rows = await drizzleDb.select().from(memoriesTable).where(and(eq(memoriesTable.workflowId, workflow_id), eq(memoriesTable.memoryType, source_type))).orderBy(desc(memoriesTable.createdAt)).limit(limit ?? 10); const ids = rows.map(r => r.id); if (ids.length > 0) await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`); return ok(rows); } catch { /* fall through */ } }
  return ok(loadMem().filter(m => m.tags.includes(workflow_id) && m.type === source_type).slice(0, limit ?? 10));
});
server.registerTool("memory.recall_by_task", { description: "Recall by task", inputSchema: { task_id: z.string(), limit: z.number().optional() } }, async ({ task_id, limit }) => {
  if (dbReady) { try { const rows = await drizzleDb.select().from(memoriesTable).where(eq(memoriesTable.taskId, task_id)).orderBy(desc(memoriesTable.createdAt)).limit(limit ?? 10); const ids = rows.map(r => r.id); if (ids.length > 0) await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`); return ok(rows); } catch { /* fall through */ } }
  return ok(loadMem().filter(m => m.tags.includes(task_id)).slice(0, limit ?? 10));
});
server.registerTool("memory.update", { description: "Update memory", inputSchema: { id: z.string(), content: z.string().optional(), importance: z.number().optional() } }, async ({ id, content, importance }) => {
  if (dbReady) { try { const data: Record<string, unknown> = {}; if (content) data.content = content; if (importance !== undefined) data.importanceScore = importance; if (Object.keys(data).length > 0) await drizzleDb.update(memoriesTable).set(data).where(eq(memoriesTable.id, id)); } catch { /* fall through */ } }
  const m = loadMem(); const r = m.find(x => x.id === id); if (!r) throw new Error("Not found"); if (content) r.content = content; if (importance !== undefined) r.importance = importance; saveMem(m); return ok(r);
});
server.registerTool("memory.delete", { description: "Delete memory", inputSchema: { id: z.string() } }, async ({ id }) => {
  if (dbReady) { try { await drizzleDb.delete(memoriesTable).where(eq(memoriesTable.id, id)); } catch { /* fall through */ } }
  const m = loadMem(); const i = m.findIndex(x => x.id === id); if (i < 0) throw new Error("Not found"); m.splice(i, 1); saveMem(m); return ok({ deleted: true });
});
server.registerTool("memory.delete_by_workflow", { description: "Delete by workflow", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => {
  if (dbReady) { try { const result = await drizzleDb.delete(memoriesTable).where(eq(memoriesTable.workflowId, workflow_id)).returning({ id: memoriesTable.id }); return ok({ deleted: result.length }); } catch { /* fall through */ } }
  const m = loadMem(); const n = m.length; saveMem(m.filter(x => !x.tags.includes(workflow_id))); return ok({ deleted: n - m.filter(x => !x.tags.includes(workflow_id)).length });
});
server.registerTool("memory.search", { description: "Search memories with composite scoring", inputSchema: { query: z.string(), limit: z.number().optional() } }, async ({ query, limit }) => {
  if (dbReady) { try { await logRetrieval({ agentName: "mcp", query, source: "memory.search", results: { limit: limit ?? 10 } }); } catch { /* non-critical */ } }
  if (dbReady) {
    try {
      const queryEmb = await generateEmbedding(query);
      if (queryEmb) {
        const vecStr = "[" + queryEmb.join(",") + "]";
        const rows = await drizzleDb.execute(sql`
          SELECT id, "memoryType", content, summary, "importanceScore", tags, "createdByAgent", "workflowId", "taskId", "createdAt", "accessCount",
            (1 - (embedding <=> ${vecStr}::vector)) * 0.6 AS sim_score,
            LEAST(1.0, "importanceScore") * 0.2 AS imp_score,
            EXP(-EXTRACT(EPOCH FROM (NOW() - "createdAt")) / 2592000.0) * 0.1 AS recency_score,
            LEAST(1.0, "accessCount"::double precision / GREATEST(1, (SELECT MAX("accessCount") FROM "Memory"))::double precision) * 0.1 AS usage_score,
            (1 - (embedding <=> ${vecStr}::vector)) * 0.6 + LEAST(1.0, "importanceScore") * 0.2 + EXP(-EXTRACT(EPOCH FROM (NOW() - "createdAt")) / 2592000.0) * 0.1 + LEAST(1.0, "accessCount"::double precision / GREATEST(1, (SELECT MAX("accessCount") FROM "Memory"))::double precision) * 0.1 AS composite_score
          FROM "Memory" WHERE embedding IS NOT NULL ORDER BY composite_score DESC LIMIT ${limit ?? 10}
        `) as Array<Record<string, unknown>>;
        if (Array.isArray(rows) && rows.length > 0) {
          const actualRows = (rows as any)?.rows ?? rows;
          if (Array.isArray(actualRows) && actualRows.length > 0) {
            const ids = actualRows.map((r: Record<string, unknown>) => String(r.id));
            if (ids.length > 0) await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`);
            return ok(actualRows);
          }
          const ids = rows.map((r: Record<string, unknown>) => String(r.id));
          await drizzleDb.execute(sql`UPDATE "Memory" SET "accessCount" = "accessCount" + 1, "accessedAt" = NOW() WHERE id = ANY(${ids}::text[])`);
          return ok(rows);
        }
      }
      const q = query.toLowerCase(); const words = q.split(/\s+/);
      const orClauses = words.flatMap(w => [sql`LOWER(${memoriesTable.summary}) LIKE ${"%" + w + "%"}`, sql`LOWER(${memoriesTable.content}) LIKE ${"%" + w + "%"}`]);
      const rows = await drizzleDb.select().from(memoriesTable).where(or(...orClauses)).orderBy(desc(memoriesTable.importanceScore)).limit(limit ?? 10);
      if (rows.length > 0) return ok(rows);
    } catch { /* fall through */ }
  }
  const q = query.toLowerCase(); return ok(loadMem().map(x => ({ ...x, score: q.split(/\s+/).filter(w => (x.content + x.summary).toLowerCase().includes(w)).length })).filter(x => x.score > 0).sort((a, b) => b.score - a.score).slice(0, limit ?? 10));
});
server.registerTool("memory.stats", { description: "Memory stats", inputSchema: {} }, async () => {
  if (dbReady) { try { const [{ value: total }] = await drizzleDb.select({ value: count() }).from(memoriesTable); const byTypeRows = await drizzleDb.select({ memoryType: memoriesTable.memoryType, count: count() }).from(memoriesTable).groupBy(memoriesTable.memoryType); return ok({ total, byType: Object.fromEntries(byTypeRows.map(b => [b.memoryType, b.count])), source: "postgresql" }); } catch { /* fall through */ } }
  const m = loadMem(); const by: Record<string, number> = {}; m.forEach(x => by[x.type] = (by[x.type] ?? 0) + 1); return ok({ total: m.length, byType: by, source: "json-cache" });
});

// Semantic search
server.registerTool("semantic-search.search_hybrid_context_pack", { description: "Build hybrid context pack", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => {
  if (dbReady) { try { return ok(await buildHybridContextPack({ workflowId: a.workflow_id, planId: a.plan_id, taskId: a.task_id, planSummary: "", taskTitle: "", acceptanceCriteria: [], requiredContext: [], recentProgress: [] }, {})); } catch (err) { return ok({ ...a, contextSufficient: true, error: String(err) }); } }
  return ok({ ...a, contextSufficient: true });
});
server.registerTool("semantic-search.search_context_fingerprint", { description: "Compute fingerprint", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string() } }, async (a) => {
  if (dbReady) { try {
    const memRows = await drizzleDb.select({ id: memoriesTable.id }).from(memoriesTable).where(or(eq(memoriesTable.workflowId, a.workflow_id), eq(memoriesTable.taskId, a.task_id))).orderBy(desc(memoriesTable.importanceScore)).limit(20);
    const docRows = await drizzleDb.select({ id: contextDocsTable.id }).from(contextDocsTable).where(eq(contextDocsTable.workflowId, a.workflow_id)).orderBy(desc(contextDocsTable.createdAt)).limit(10);
    const fingerprint = computeFingerprint({ workflowId: a.workflow_id, planId: a.plan_id, taskId: a.task_id, acceptanceCriteria: [], requiredContext: [], memoryIds: memRows.map(r => r.id), docIds: docRows.map(r => r.id) });
    return ok({ fingerprint, contextSufficient: true });
  } catch { /* fall through */ } }
  return ok({ fingerprint: [a.workflow_id, a.plan_id, a.task_id].join("-"), contextSufficient: true });
});
server.registerTool("semantic-search.code_search", { description: "Code search", inputSchema: { query: z.string() } }, async ({ query }) => { if (dbReady) { try { await logRetrieval({ agentName: "mcp", query, source: "code_search", results: {} }); } catch { /* non-critical */ } } return ok({ query, results: [] }); });
server.registerTool("semantic-search.make_fingerprint", { description: "Generate fingerprint", inputSchema: { workflow_id: z.string(), plan_id: z.string(), task_id: z.string(), acceptance_criteria: z.array(z.string()).optional().default([]), required_context: z.array(z.string()).optional().default([]), document_ids: z.array(z.string()).optional().default([]), memory_ids: z.array(z.string()).optional().default([]) } }, async (a) => {
  const fingerprint = makeFingerprint({ workflowId: a.workflow_id, planId: a.plan_id, taskId: a.task_id, acceptanceCriteria: a.acceptance_criteria, requiredContext: a.required_context, documentIds: a.document_ids, memoryIds: a.memory_ids });
  if (dbReady) { try { await drizzleDb.update(tasksTable).set({ contextFingerprint: fingerprint }).where(and(eq(tasksTable.workflowId, a.workflow_id), eq(tasksTable.id, a.task_id))); } catch { /* non-critical */ } }
  return ok({ fingerprint, workflowId: a.workflow_id, planId: a.plan_id, taskId: a.task_id, inputHashLength: JSON.stringify(a).length });
});

// Policy
server.registerTool("policy.check_session_readiness", { description: "Session readiness", inputSchema: { sessionKey: z.string() } }, async ({ sessionKey }) => {
  if (dbReady) { const [state] = await drizzleDb.select().from(sessionStates).where(eq(sessionStates.sessionKey, sessionKey)).limit(1); const ready = state ? (state.workflowLoaded && state.planLoaded && state.taskLoaded) : false; return ok({ sessionKey, ready, workflowLoaded: state?.workflowLoaded ?? false, planLoaded: state?.planLoaded ?? false, taskLoaded: state?.taskLoaded ?? false }); }
  return ok({ sessionKey, ready: true });
});
server.registerTool("policy.validate_execution", { description: "Validate execution", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (dbReady) { const [wf] = await drizzleDb.select().from(workflowsTable).where(eq(workflowsTable.id, a.workflowId)).limit(1); if (!wf) return ok({ valid: false, reason: "Workflow not found", ...a }); if (wf.status === "COMPLETED" || wf.status === "FAILED") return ok({ valid: false, reason: "Workflow is " + wf.status, ...a }); const [task] = await drizzleDb.select().from(tasksTable).where(eq(tasksTable.id, a.taskId)).limit(1); if (!task) return ok({ valid: false, reason: "Task not found", ...a }); if (task.status === "done") return ok({ valid: false, reason: "Task already completed", ...a }); return ok({ valid: true, ...a }); }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.validate_completion", { description: "Validate completion", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (dbReady) { const [review] = await drizzleDb.select().from(reviewDecisionsTable).where(and(eq(reviewDecisionsTable.workflowId, a.workflowId), eq(reviewDecisionsTable.taskId, a.taskId))).orderBy(desc(reviewDecisionsTable.createdAt)).limit(1); const approved = review?.decision === "APPROVED"; return ok({ valid: approved, reviewDecision: review?.decision ?? "none", ...a }); }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.validate_parallel_completion", { description: "Validate parallel", inputSchema: { sessionKey: z.string(), workflowId: z.string(), taskId: z.string() } }, async (a) => {
  if (dbReady) { const branches = await drizzleDb.select().from(parallelBranchesTable).where(and(eq(parallelBranchesTable.workflowId, a.workflowId), eq(parallelBranchesTable.taskId, a.taskId))); const allDone = branches.length > 0 && branches.every(b => b.status === "completed"); const [session] = await drizzleDb.select().from(sessionStates).where(eq(sessionStates.sessionKey, a.sessionKey)).limit(1); return ok({ valid: allDone && (session?.synthesisReady ?? false), allBranchesDone: allDone, branchCount: branches.length, ...a }); }
  return ok({ valid: true, ...a });
});
server.registerTool("policy.detect_scope_drift", { description: "Detect drift", inputSchema: { workflowId: z.string().optional(), taskId: z.string().optional(), outputText: z.string() } }, async (a) => {
  const outputLen = a.outputText.length; const suspiciousKeywords = ["unrelated", "off-topic", "completely different"]; const driftDetected = suspiciousKeywords.some(k => a.outputText.toLowerCase().includes(k));
  if (dbReady && a.workflowId) { const [task] = a.taskId ? await drizzleDb.select().from(tasksTable).where(eq(tasksTable.id, a.taskId)).limit(1) : [null]; return ok({ driftDetected, outputLength: outputLen, taskTitle: task?.title ?? null }); }
  return ok({ driftDetected, outputLength: outputLen });
});
server.registerTool("policy.require_context_refresh", { description: "Context refresh", inputSchema: { workflowId: z.string().optional(), planId: z.string().optional(), taskId: z.string().optional(), last_fingerprint: z.string().optional() } }, async (a) => {
  if (dbReady && a.workflowId) { const [wf] = await drizzleDb.select().from(workflowsTable).where(eq(workflowsTable.id, a.workflowId)).limit(1); const currentFingerprint = wf ? `${wf.id}-${wf.updatedAt.getTime()}` : null; const needsRefresh = a.last_fingerprint ? currentFingerprint !== a.last_fingerprint : false; return ok({ needsRefresh, fingerprint: currentFingerprint, workflowId: a.workflowId }); }
  return ok({ needsRefresh: false });
});

// Capability
function listFiles(dir: string, ext: string): string[] { if (!fs.existsSync(dir)) return []; return fs.readdirSync(dir).filter(f => f.endsWith(ext)).map(f => f.replace(ext, "")); }
function parseFrontmatter(content: string): Record<string, string> { const match = content.match(/^---\n([\s\S]*?)\n---/); if (!match) return {}; const fm: Record<string, string> = {}; for (const line of match[1].split("\n")) { const idx = line.indexOf(":"); if (idx > 0) fm[line.slice(0, idx).trim()] = line.slice(idx + 1).trim(); } return fm; }

server.registerTool("capability.list_agents", { description: "List agents", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => { const root = safePath(projectRoot); const agentDir = path.join(root, ".claude", "agents"); const agents = listFiles(agentDir, ".md").map(name => { const content = fs.readFileSync(path.join(agentDir, name + ".md"), "utf-8"); const fm = parseFrontmatter(content); return { name, role: fm.role ?? "", description: fm.description ?? "" }; }); return ok({ agents, projectRoot: root }); });
server.registerTool("capability.list_skills", { description: "List skills", inputSchema: { projectRoot: z.string() } }, async ({ projectRoot }) => { const root = safePath(projectRoot); const skillDir = path.join(root, ".claude", "skills"); const skills = listFiles(skillDir, ".md").map(name => { const content = fs.readFileSync(path.join(skillDir, name + ".md"), "utf-8"); const fm = parseFrontmatter(content); return { name, description: fm.description ?? "", trigger: fm.trigger ?? "" }; }); return ok({ skills, projectRoot: root }); });
server.registerTool("capability.list_templates", { description: "List templates", inputSchema: {} }, async () => ok({ templates: ["mcp-server", "agent", "skill", "feature"] }));
server.registerTool("capability.match_agent", { description: "Match agent", inputSchema: { projectRoot: z.string(), taskDescription: z.string() } }, async (a) => { const root = safePath(a.projectRoot); const agentDir = path.join(root, ".claude", "agents"); const agents = listFiles(agentDir, ".md").map(name => { const content = fs.readFileSync(path.join(agentDir, name + ".md"), "utf-8"); const fm = parseFrontmatter(content); return { name, role: fm.role ?? "", description: fm.description ?? "" }; }); const q = a.taskDescription.toLowerCase(); const scored = agents.map(ag => ({ ...ag, score: (ag.role + " " + ag.description).toLowerCase().split(/\s+/).filter(w => q.includes(w)).length })).sort((a, b) => b.score - a.score); return ok({ match: scored[0]?.score ? scored[0] : null }); });
server.registerTool("capability.system_readiness", { description: "System readiness", inputSchema: { projectRoot: z.string().optional() } }, async ({ projectRoot }) => ok({ ready: true, backend: backendType, postgresql: dbReady, transport: TRANSPORT_MODE, projectRoot: safePath(projectRoot) }));
server.registerTool("capability.workflow_audit", { description: "Audit", inputSchema: { workflowId: z.string().optional() } }, async ({ workflowId }) => {
  if (dbReady && workflowId) { const [wf] = await drizzleDb.select().from(workflowsTable).where(eq(workflowsTable.id, workflowId)).limit(1); if (!wf) return ok({ audited: 0, issues: ["Workflow not found"] }); const tasksForWf = await drizzleDb.select().from(tasksTable).where(eq(tasksTable.workflowId, workflowId)); const progressForWf = await drizzleDb.select().from(taskProgressLogs).where(eq(taskProgressLogs.workflowId, workflowId)); const issues: string[] = []; const running = tasksForWf.filter(t => t.status === "running"); if (running.length > 1) issues.push(`Multiple running tasks: ${running.map(t => t.id.slice(0, 8)).join(", ")}`); const noProgress = tasksForWf.filter(t => t.status === "running" && progressForWf.filter(p => p.taskId === t.id).length === 0); if (noProgress.length > 0) issues.push(`Running tasks with no progress: ${noProgress.map(t => t.id.slice(0, 8)).join(", ")}`); return ok({ audited: 1, issues, taskCount: tasksForWf.length }); }
  return ok({ audited: workflowId ? 1 : engine.listWorkflows().length, issues: [] });
});
server.registerTool("capability.create_agent", { description: "Create agent", inputSchema: { projectRoot: z.string(), name: z.string(), role: z.string(), description: z.string(), instructions: z.string() } }, async (a) => ok(createAgent({ projectRoot: safePath(a.projectRoot), name: a.name, role: a.role, description: a.description, instructions: a.instructions })));
server.registerTool("capability.create_skill", { description: "Create skill", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string(), trigger: z.string(), steps: z.array(z.string()) } }, async (a) => ok(createSkill({ projectRoot: safePath(a.projectRoot), name: a.name, description: a.description, trigger: a.trigger, steps: a.steps })));
server.registerTool("capability.scaffold_feature", { description: "Scaffold feature", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => { const root = safePath(a.projectRoot); const agentResult = createAgent({ projectRoot: root, name: a.name, role: `${a.name} specialist`, description: a.description, instructions: `Implement the ${a.name} feature.` }); const skillResult = createSkill({ projectRoot: root, name: a.name, description: a.description, trigger: `when working on ${a.name}`, steps: [`Analyze requirements for ${a.name}`, "Plan implementation", "Implement with TDD", "Verify and review"] }); return ok({ ok: true, createdFiles: [agentResult.filePath, skillResult.filePath] }); });
server.registerTool("capability.scaffold_mcp_server", { description: "Scaffold MCP", inputSchema: { projectRoot: z.string(), name: z.string(), description: z.string() } }, async (a) => { const root = safePath(a.projectRoot); const serverDir = path.join(root, "apps", a.name); if (!fs.existsSync(serverDir)) fs.mkdirSync(serverDir, { recursive: true }); const srcDir = path.join(serverDir, "src"); if (!fs.existsSync(srcDir)) fs.mkdirSync(srcDir, { recursive: true }); fs.writeFileSync(path.join(serverDir, "package.json"), JSON.stringify({ name: `@mcp-rebuild/${a.name}`, version: "0.1.0", type: "module", main: "src/index.ts" }, null, 2)); fs.writeFileSync(path.join(srcDir, "index.ts"), `import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";\nimport { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";\nimport { z } from "zod";\n\nconst server = new McpServer({ name: "${a.name}", version: "0.1.0" });\n\n// Add tools here\n\nconst transport = new StdioServerTransport();\nawait server.connect(transport);\n`); return ok({ ok: true, createdDir: serverDir }); });
server.registerTool("capability.ping", { description: "Capability health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType, postgresql: dbReady, transport: TRANSPORT_MODE }));

// Filesystem
server.registerTool("filesystem.read", { description: "Read file", inputSchema: { path: z.string() } }, async ({ path: fp }) => ({ content: [{ type: "text" as const, text: fs.readFileSync(fp, "utf-8") }] }));
server.registerTool("filesystem.write", { description: "Write file", inputSchema: { path: z.string(), content: z.string() } }, async ({ path: fp, content }) => { const d = path.dirname(fp); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(fp, content); return ok({ ok: true }); });
server.registerTool("filesystem.list", { description: "List dir", inputSchema: { path: z.string() } }, async ({ path: fp }) => ok(fs.readdirSync(fp, { withFileTypes: true }).map(e => ({ name: e.name, type: e.isDirectory() ? "dir" : "file" }))));
server.registerTool("filesystem.delete", { description: "Delete file", inputSchema: { path: z.string() } }, async ({ path: fp }) => { fs.unlinkSync(fp); return ok({ ok: true }); });
server.registerTool("filesystem.stat", { description: "File stat", inputSchema: { path: z.string() } }, async ({ path: fp }) => { const s = fs.statSync(fp); return ok({ size: s.size, isFile: s.isFile() }); });

// Review
server.registerTool("review.submit", { description: "Submit review", inputSchema: { workflow_id: z.string(), task_id: z.string(), reviewer_agent: z.string(), decision: z.string(), notes: z.string(), gaps: z.array(z.string()).optional(), tests_verified: z.boolean().optional(), test_summary: z.object({ testFiles: z.array(z.string()), testsPassed: z.boolean(), coveragePercent: z.number().optional() }).optional() } }, async (a) => {
  if (dbReady) { const [review] = await drizzleDb.insert(reviewDecisionsTable).values({ workflowId: a.workflow_id, taskId: a.task_id, reviewerAgent: a.reviewer_agent, decision: a.decision, notes: a.notes, gaps: a.gaps ?? [] as never, testsVerified: a.tests_verified ?? false, testSummary: a.test_summary ?? {} as never }).returning();
    if (a.decision === "APPROVED" && dbReady) { try { const [task] = await drizzleDb.select().from(tasksTable).where(eq(tasksTable.id, a.task_id)).limit(1); if (task && task.requiresTdd && !a.tests_verified) { await drizzleDb.update(tasksTable).set({ status: "RUNNING" }).where(eq(tasksTable.id, a.task_id)); return ok({ submitted: true, id: review.id, _tddWarning: "Task requires TDD but tests_verified=false. Task stays RUNNING." }); } } catch { /* non-critical */ } }
    return ok({ submitted: true, id: review.id }); }
  return ok({ submitted: true });
});
server.registerTool("review.get_latest", { description: "Get latest review", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async (a) => {
  if (dbReady) { const [review] = await drizzleDb.select().from(reviewDecisionsTable).where(and(eq(reviewDecisionsTable.workflowId, a.workflow_id), eq(reviewDecisionsTable.taskId, a.task_id))).orderBy(desc(reviewDecisionsTable.createdAt)).limit(1); return ok({ review }); }
  return ok({ review: null });
});

// Session
server.registerTool("session.get_state", { description: "Get session state", inputSchema: { session_key: z.string() } }, async ({ session_key }) => { if (dbReady) { const [state] = await drizzleDb.select().from(sessionStates).where(eq(sessionStates.sessionKey, session_key)).limit(1); return ok({ sessionKey: session_key, state: state ?? {} }); } return ok({ sessionKey: session_key, state: {} }); });
server.registerTool("session.patch_state", { description: "Patch session state", inputSchema: { session_key: z.string(), patch: z.record(z.any()) } }, async ({ session_key, patch }) => {
  if (dbReady) { const [existing] = await drizzleDb.select().from(sessionStates).where(eq(sessionStates.sessionKey, session_key)).limit(1);
    if (existing) { const merged = { ...(existing.metadata as Record<string, unknown>), ...patch }; await drizzleDb.update(sessionStates).set({ metadata: merged as never, workflowId: (patch as Record<string, unknown>).workflowId as string ?? existing.workflowId, planId: (patch as Record<string, unknown>).planId as string ?? existing.planId, taskId: (patch as Record<string, unknown>).taskId as string ?? existing.taskId }).where(eq(sessionStates.sessionKey, session_key)); return ok({ sessionKey: session_key, patched: true, patch }); }
    await drizzleDb.insert(sessionStates).values({ id: randomUUID(), sessionKey: session_key, metadata: patch as never, workflowId: (patch as Record<string, unknown>).workflowId as string ?? null, planId: (patch as Record<string, unknown>).planId as string ?? null, taskId: (patch as Record<string, unknown>).taskId as string ?? null }); return ok({ sessionKey: session_key, patched: true, patch, created: true }); }
  return ok({ sessionKey: session_key, patched: true, patch });
});
server.registerTool("session.init_context", { description: "Init session context — also checks for stale/stuck/failed workflows and returns reminders", inputSchema: { cwd: z.string() } }, async ({ cwd: rawCwd }) => {
  const dir = safePath(rawCwd); let reminders: unknown[] = []; let stats: { total: number; unacknowledged: number; bySeverity: Record<string, number> } | null = null;
  if (dbReady) { try { reminders = await checkReminders(); stats = await reminderStats(); if (reminders.length > 0) logger.info({ count: reminders.length }, "Session init: reminders detected"); } catch (e) { logger.warn("Session init reminder check failed: " + (e instanceof Error ? e.message : String(e))); } }
  return ok({ initialized: true, cwd: dir, reminders, reminderStats: stats });
});

// Local
server.registerTool("local.init", { description: "Init local state dir", inputSchema: { cwd: z.string() } }, async ({ cwd: rawCwd }) => { const dir = safePath(rawCwd); const p = path.join(dir, ".masday"); if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true }); return ok({ initialized: true, path: p }); });
server.registerTool("local.sync", { description: "Sync local state from DB", inputSchema: { cwd: z.string(), workflow_id: z.string().optional() } }, async ({ cwd: rawCwd, workflow_id }) => { const dir = safePath(rawCwd); if (!dbReady) return ok({ synced: false, error: "PostgreSQL not connected" }); const whereCond = workflow_id ? eq(memoriesTable.workflowId, workflow_id) : undefined; const rows = await drizzleDb.select().from(memoriesTable).where(whereCond).orderBy(desc(memoriesTable.createdAt)); const cached: MemRec[] = rows.map(r => ({ id: r.id, type: r.memoryType, content: r.content, summary: r.summary, source: r.createdByAgent, importance: r.importanceScore ?? 0.5, tags: r.tags, createdAt: r.createdAt.getTime() })); const memDir = path.join(dir, ".masday", "state"); if (!fs.existsSync(memDir)) fs.mkdirSync(memDir, { recursive: true }); saveMem(cached); return ok({ synced: true, records: cached.length, workflowId: workflow_id ?? "all" }); });
server.registerTool("local.push", { description: "Push local state to DB", inputSchema: { cwd: z.string(), workflow_id: z.string().optional() } }, async ({ cwd: rawCwd, workflow_id }) => { const dir = safePath(rawCwd); if (!dbReady) return ok({ pushed: false, error: "PostgreSQL not connected" }); const memFile = path.join(dir, ".masday", "state", "memories.json"); const local: MemRec[] = fs.existsSync(memFile) ? JSON.parse(fs.readFileSync(memFile, "utf-8")) : []; let created = 0, skipped = 0; for (const r of local) { const [existing] = await drizzleDb.select({ id: memoriesTable.id }).from(memoriesTable).where(eq(memoriesTable.id, r.id)).limit(1); if (existing) { skipped++; continue; } try { await drizzleDb.insert(memoriesTable).values({ id: r.id, memoryType: r.type, summary: r.summary, content: r.content, importanceScore: r.importance, createdByAgent: r.source, tags: r.tags, createdAt: new Date(r.createdAt) }); created++; } catch { skipped++; } } return ok({ pushed: true, created, skipped, total: local.length, workflowId: workflow_id }); });
server.registerTool("local.save_artifact", { description: "Save artifact file locally", inputSchema: { cwd: z.string(), category: z.string(), filename: z.string(), content: z.string() } }, async ({ cwd: rawCwd, category, filename, content }) => { const dir = safePath(rawCwd); const d = path.join(dir, ".masday", category); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(path.join(d, filename), content); return ok({ saved: true, path: path.join(d, filename) }); });

// Shell tools — execFileSync with args arrays (no string interpolation)
server.registerTool("git.status", { description: "Git status", inputSchema: {} }, async () => { try { return ok({ stdout: execFileSync("git", ["status", "--porcelain"], { encoding: "utf-8", timeout: 10000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("git.diff", { description: "Git diff", inputSchema: {} }, async () => { try { const stat = execFileSync("git", ["diff", "--stat"], { encoding: "utf-8", timeout: 15000 }); const diff = execFileSync("git", ["diff"], { encoding: "utf-8", timeout: 15000 }); return ok({ stdout: stat + diff, exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("git.commit", { description: "Git commit", inputSchema: { message: z.string() } }, async ({ message }) => { try { return ok({ stdout: execFileSync("git", ["commit", "-m", message], { encoding: "utf-8", timeout: 15000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("npm.install", { description: "NPM install", inputSchema: { packages: z.array(z.string()).optional() } }, async ({ packages }) => { if (packages?.some(p => !SAFE_PACKAGE_RE.test(p))) return ok({ stdout: "", stderr: "Invalid package name", exitCode: 1, packages: packages ?? [] }); const args = packages?.length ? ["add", ...packages] : ["install"]; try { return ok({ stdout: execFileSync("pnpm", args, { encoding: "utf-8", timeout: 120000 }), exitCode: 0, packages: packages ?? [] }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, packages: packages ?? [] }); } });
server.registerTool("npm.run", { description: "NPM run script", inputSchema: { script: z.string() } }, async ({ script }) => { if (!SAFE_SCRIPT_RE.test(script)) return ok({ stdout: "", stderr: "Invalid script name", exitCode: 1 }); try { return ok({ stdout: execFileSync("pnpm", ["run", script], { encoding: "utf-8", timeout: 120000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("docker.build", { description: "Docker build", inputSchema: { tag: z.string().optional() } }, async ({ tag }) => { if (tag && !SAFE_IMAGE_RE.test(tag)) return ok({ stdout: "", stderr: "Invalid image tag", exitCode: 1 }); const args = tag ? ["build", "-t", tag, "."] : ["build", "."]; try { return ok({ stdout: execFileSync("docker", args, { encoding: "utf-8", timeout: 300000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("docker.run", { description: "Docker run", inputSchema: { image: z.string() } }, async ({ image }) => { if (!SAFE_IMAGE_RE.test(image)) return ok({ stdout: "", stderr: "Invalid image name", exitCode: 1 }); try { return ok({ stdout: execFileSync("docker", ["run", "--rm", image], { encoding: "utf-8", timeout: 300000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("docker.ps", { description: "Docker ps", inputSchema: {} }, async () => { try { return ok({ stdout: execFileSync("docker", ["ps", "--format", "json"], { encoding: "utf-8", timeout: 10000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("cicd.pipeline_status", { description: "CI/CD pipeline status", inputSchema: {} }, async () => { try { return ok({ pipelines: JSON.parse(execFileSync("gh", ["run", "list", "--limit", "5", "--json", "name,status,conclusion,createdAt"], { encoding: "utf-8", timeout: 15000 })), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ pipelines: [], exitCode: err.status ?? 1 }); } });
server.registerTool("cicd.pipeline_trigger", { description: "Trigger CI/CD pipeline", inputSchema: { pipeline: z.string() } }, async ({ pipeline }) => { if (!SAFE_PIPELINE_RE.test(pipeline)) return ok({ triggered: false, exitCode: 1, error: "Invalid pipeline name" }); try { return ok({ triggered: true, stdout: execFileSync("gh", ["workflow", "run", pipeline], { encoding: "utf-8", timeout: 30000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ triggered: false, exitCode: err.status ?? 1 }); } });
server.registerTool("cicd.runs_view", { description: "View CI/CD runs", inputSchema: {} }, async () => { try { return ok({ runs: JSON.parse(execFileSync("gh", ["run", "list", "--limit", "20", "--json", "name,status,conclusion,headBranch,createdAt,databaseId"], { encoding: "utf-8", timeout: 15000 })), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ runs: [], exitCode: err.status ?? 1 }); } });
server.registerTool("github.pr_create", { description: "Create GitHub PR", inputSchema: { title: z.string(), body: z.string().optional() } }, async ({ title, body }) => { try { return ok({ created: true, stdout: execFileSync("gh", ["pr", "create", "--title", title, "--body", body ?? ""], { encoding: "utf-8", timeout: 30000 }), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ created: false, stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1 }); } });
server.registerTool("github.pr_list", { description: "List GitHub PRs", inputSchema: {} }, async () => { try { return ok({ prs: JSON.parse(execFileSync("gh", ["pr", "list", "--json", "number,title,state,author,createdAt"], { encoding: "utf-8", timeout: 15000 })), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ prs: [], exitCode: err.status ?? 1 }); } });
server.registerTool("github.issue_list", { description: "List GitHub issues", inputSchema: {} }, async () => { try { return ok({ issues: JSON.parse(execFileSync("gh", ["issue", "list", "--json", "number,title,state,labels,createdAt"], { encoding: "utf-8", timeout: 15000 })), exitCode: 0 }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ issues: [], exitCode: err.status ?? 1 }); } });
server.registerTool("tests.run", { description: "Run tests", inputSchema: { pattern: z.string().optional() } }, async ({ pattern }) => { if (pattern && !SAFE_PATTERN_RE.test(pattern)) return ok({ stdout: "", stderr: "Invalid test pattern", exitCode: 1, pattern }); const args = pattern ? ["test", "--", pattern] : ["test"]; try { return ok({ stdout: execFileSync("pnpm", args, { encoding: "utf-8", timeout: 120000 }), exitCode: 0, pattern }); } catch (e: unknown) { const err = e as { stdout?: string; stderr?: string; status?: number }; return ok({ stdout: err.stdout ?? "", stderr: err.stderr ?? "", exitCode: err.status ?? 1, pattern }); } });

// Reminders
server.registerTool("reminder.check", { description: "Check for stale, stuck, and failed workflows/tasks", inputSchema: { staleExecutionMinutes: z.number().min(1).optional(), stuckTaskMinutes: z.number().min(1).optional(), includeFailed: z.boolean().optional() } }, async (cfg) => { if (!dbReady) return ok({ reminders: [], note: "PostgreSQL not connected" }); try { const reminders = await checkReminders(cfg); return ok({ reminders, count: reminders.length }); } catch (e) { return ok({ error: String(e), reminders: [] }); } });
server.registerTool("reminder.list", { description: "List stored reminders", inputSchema: { workflowId: z.string().optional(), acknowledged: z.boolean().optional(), limit: z.number().min(1).max(200).optional() } }, async ({ workflowId, acknowledged, limit }) => { if (!dbReady) return ok({ reminders: [], note: "PostgreSQL not connected" }); try { const reminders = await listRemindersDb({ workflowId, acknowledged, limit }); const stats = await reminderStats(); return ok({ reminders, stats }); } catch (e) { return ok({ error: String(e), reminders: [] }); } });
server.registerTool("reminder.acknowledge", { description: "Acknowledge a reminder or dismiss all for a workflow", inputSchema: { id: z.string().optional(), workflowId: z.string().optional() } }, async ({ id, workflowId }) => { if (!dbReady) return ok({ error: "PostgreSQL not connected" }); try { if (workflowId) { const result = await dismissWorkflowReminders(workflowId); return ok({ dismissed: true, workflowId, count: result.count }); } if (id) { const result = await acknowledgeReminder(id); return ok({ acknowledged: true, id, result }); } return ok({ error: "Provide either id or workflowId" }); } catch (e) { return ok({ error: String(e) }); } });

// Project rules
import { validateProject, formatReport, getFailedCritical } from "@mcp-rebuild/project-rules";
server.registerTool("projectRules.check", { description: "Validate project against refactor rules", inputSchema: { projectRoot: z.string().optional() } }, async ({ projectRoot }) => { try { const root = safePath(projectRoot); const report = validateProject(root); return ok({ ...report, formatted: formatReport(report), criticalCount: getFailedCritical(report).length }); } catch (e) { return ok({ error: String(e) }); } });

// ── TRANSPORT SELECTION ──────────────────────────────────────

const toolCount = toolNameRegistry.size;

if (TRANSPORT_MODE === "http") {
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: () => randomUUID() });
  await server.connect(transport);
  logger.info("MCP server connected to HTTP transport");

  const httpServer = createServer(async (req, res) => {
    res.setHeader("Access-Control-Allow-Origin", process.env.CORS_ORIGINS ?? "*");
    res.setHeader("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
    res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Mcp-Session-Id");
    res.setHeader("Access-Control-Expose-Headers", "Mcp-Session-Id");
    if (req.method === "OPTIONS") { res.writeHead(204); res.end(); return; }

    const url = new URL(req.url ?? "/", `http://${req.headers.host}`);

    if (url.pathname === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ status: "ok", backend: backendType, postgresql: dbReady, tools: toolCount, transport: "http" }));
      return;
    }

    if (url.pathname === "/mcp") {
      if (MCP_API_KEY) {
        const auth = req.headers.authorization;
        const token = auth?.startsWith("Bearer ") ? auth.slice(7) : null;
        if (token !== MCP_API_KEY) {
          res.writeHead(401, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ error: "Unauthorized" }));
          return;
        }
      }
      let parsedBody: unknown;
      if (req.method === "POST" || req.method === "PUT") {
        const chunks: Buffer[] = [];
        for await (const chunk of req) chunks.push(chunk);
        const body = Buffer.concat(chunks).toString("utf-8");
        try { parsedBody = JSON.parse(body); } catch { parsedBody = body; }
      }
      await transport.handleRequest(req, res, parsedBody);
      return;
    }

    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: "Not found", endpoints: ["/mcp", "/health"] }));
  });

  httpServer.listen(HTTP_PORT, () => {
    logger.info(`MCP HTTP/SSE server listening on port ${HTTP_PORT}`);
    logger.info("Endpoints: POST /mcp, GET /mcp (SSE), DELETE /mcp, GET /health");
    logger.info(toolCount + " tools registered");
  });

  // Background init
  (async () => {
    const doctorReport = runDoctor(cwd);
    if (doctorReport.fixedCount > 0) {
      for (const d of doctorReport.diagnoses.filter((d: { autoFixed?: boolean }) => d.autoFixed)) {
        logger.info(`Doctor [${d.check}]: ${d.message}`);
      }
    }
    await Promise.race([initDb(), new Promise<void>(resolve => setTimeout(() => { logger.warn("initDb() timed out after 2.5s, continuing without DB"); resolve(); }, 2500))]);
    if (dbReady) {
      try {
        const startupReminders = await checkReminders();
        if (startupReminders.length > 0) { logger.info({ count: startupReminders.length }, "Startup: detected stale/stuck/failed items"); }
      } catch (e) { logger.warn("Startup reminder check failed: " + (e instanceof Error ? e.message : String(e))); }
      const REMINDER_INTERVAL_MS = 15 * 60_000;
      const reminderTimer = setInterval(async () => { try { const reminders = await checkReminders(); if (reminders.length > 0) logger.info({ count: reminders.length }, "Periodic reminder check"); } catch (e) { logger.warn("Periodic reminder check failed: " + (e instanceof Error ? e.message : String(e))); } }, REMINDER_INTERVAL_MS);
      reminderTimer.unref();
    }
    logger.info("Background init complete" + (dbReady ? " (PostgreSQL hybrid)" : " (JSON-only)"));
  })();

} else {
  // STDIO mode
  const transport = new StdioServerTransport();
  await server.connect(transport);
  logger.info("MCP Server running on stdio (" + backendType + " backend) — " + toolCount + " tools");

  (async () => {
    const doctorReport = runDoctor(cwd);
    if (doctorReport.fixedCount > 0) {
      for (const d of doctorReport.diagnoses.filter((d: { autoFixed?: boolean }) => d.autoFixed)) {
        logger.info(`Doctor [${d.check}]: ${d.message}`);
      }
    }
    await Promise.race([initDb(), new Promise<void>(resolve => setTimeout(() => { logger.warn("initDb() timed out after 2.5s, continuing without DB"); resolve(); }, 2500))]);
    if (dbReady) {
      try {
        const startupReminders = await checkReminders();
        if (startupReminders.length > 0) { logger.info({ count: startupReminders.length }, "Startup: detected stale/stuck/failed items"); }
      } catch (e) { logger.warn("Startup reminder check failed: " + (e instanceof Error ? e.message : String(e))); }
      const REMINDER_INTERVAL_MS = 15 * 60_000;
      const reminderTimer = setInterval(async () => { try { const reminders = await checkReminders(); if (reminders.length > 0) logger.info({ count: reminders.length }, "Periodic reminder check"); } catch (e) { logger.warn("Periodic reminder check failed: " + (e instanceof Error ? e.message : String(e))); } }, REMINDER_INTERVAL_MS);
      reminderTimer.unref();
    }
    logger.info("Background init complete" + (dbReady ? " (PostgreSQL hybrid)" : " (JSON-only)"));
  })();
}
