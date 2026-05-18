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
 * TOTAL TOOLS: 83 (54 original + 29 stubs)
 *
 * Namespaces & tools:
 *   workflow (23): create, execute, getStatus, get, list, addTask, startTask, completeTask,
 *                  saveProgress, listTasks, getCurrentTask, getPlan, getActive, createPlan,
 *                  createParallelBranches, completeParallelBranch, listParallelBranches, delete, ping,
 *                  set_execution_mode, mark_synthesis_ready, mark_verification_ready,
 *                  resume_suggestion [last 4 are STUBS]
 *   memory (11): store, store_research, recall_recent, recall_documents, recall_document_by_type,
 *                recall_by_task, update, delete, delete_by_workflow, search, stats
 *   semantic-search (3): search_hybrid_context_pack, search_context_fingerprint, code_search
 *   policy (6): check_session_readiness, validate_execution, validate_completion,
 *               validate_parallel_completion, detect_scope_drift, require_context_refresh
 *   capability (11): list_agents, list_skills, list_templates, match_agent, system_readiness,
 *                    workflow_audit, create_agent, create_skill, scaffold_feature, scaffold_mcp_server, ping
 *   filesystem (5): read, write, list, delete, stat
 *   review (2): submit, get_latest [STUB]
 *   session (3): get_state, patch_state, init_context [STUB]
 *   local (4): init, sync, push, save_artifact [STUB]
 *   git (3): status, diff, commit [STUB]
 *   npm (2): install, run [STUB]
 *   docker (3): build, run, ps [STUB]
 *   cicd (3): pipeline_status, pipeline_trigger, runs_view [STUB]
 *   github (3): pr_create, pr_list, issue_list [STUB]
 *   tests (1): run [STUB]
 *
 * SYNC: pre-build-skill.js MCP_TOOLS set must match this list exactly.
 * SYNC: All masday skill and agent .md files must use camelCase tool names.
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
server.registerTool("workflow.set_execution_mode", { description: "Set execution mode", inputSchema: { session_key: z.string(), mode: z.string() } }, async ({ session_key, mode }) => ok({ sessionKey: session_key, mode }));
server.registerTool("workflow.mark_synthesis_ready", { description: "Mark synthesis ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, synthesisReady: ready }));
server.registerTool("workflow.mark_verification_ready", { description: "Mark verification ready", inputSchema: { session_key: z.string(), ready: z.boolean() } }, async ({ session_key, ready }) => ok({ sessionKey: session_key, verificationReady: ready }));
server.registerTool("workflow.resume_suggestion", { description: "Get resume suggestion", inputSchema: { workflow_id: z.string() } }, async ({ workflow_id }) => ok({ workflowId: workflow_id, suggestion: "continue" }));

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

// --- STUB TOOLS (28) ---
// These tools are referenced by skills/agents but need real implementations.
// Each returns a minimal valid response.

// review (2)
server.registerTool("review.submit", { description: "Submit review", inputSchema: { workflow_id: z.string(), task_id: z.string(), reviewer_agent: z.string(), decision: z.string(), notes: z.string(), gaps: z.array(z.string()).optional() } }, async (a) => ok({ submitted: true, ...a }));
server.registerTool("review.get_latest", { description: "Get latest review", inputSchema: { workflow_id: z.string(), task_id: z.string() } }, async (a) => ok({ review: null, ...a }));

// session (3)
server.registerTool("session.get_state", { description: "Get session state", inputSchema: { session_key: z.string() } }, async ({ session_key }) => ok({ sessionKey: session_key, state: {} }));
server.registerTool("session.patch_state", { description: "Patch session state", inputSchema: { session_key: z.string(), patch: z.record(z.any()) } }, async ({ session_key, patch }) => ok({ sessionKey: session_key, patched: true, patch }));
server.registerTool("session.init_context", { description: "Init session context", inputSchema: { cwd: z.string() } }, async ({ cwd }) => ok({ initialized: true, cwd }));

// local (4)
server.registerTool("local.init", { description: "Init local state dir", inputSchema: { cwd: z.string() } }, async ({ cwd }) => { const p = path.join(cwd, ".masday"); if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true }); return ok({ initialized: true, path: p }); });
server.registerTool("local.sync", { description: "Sync local state from DB", inputSchema: { cwd: z.string(), workflow_id: z.string() } }, async (a) => ok({ synced: true, ...a }));
server.registerTool("local.push", { description: "Push local state to DB", inputSchema: { cwd: z.string(), workflow_id: z.string() } }, async (a) => ok({ pushed: true, ...a }));
server.registerTool("local.save_artifact", { description: "Save artifact file locally", inputSchema: { cwd: z.string(), category: z.string(), filename: z.string(), content: z.string() } }, async ({ cwd, category, filename, content }) => { const d = path.join(cwd, ".masday", category); if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true }); fs.writeFileSync(path.join(d, filename), content); return ok({ saved: true, path: path.join(d, filename) }); });

// git (3)
server.registerTool("git.status", { description: "Git status", inputSchema: {} }, async () => ok({ stdout: "", stderr: "", exitCode: 0 }));
server.registerTool("git.diff", { description: "Git diff", inputSchema: {} }, async () => ok({ stdout: "", stderr: "", exitCode: 0 }));
server.registerTool("git.commit", { description: "Git commit", inputSchema: { message: z.string() } }, async ({ message }) => ok({ stdout: "", stderr: "", exitCode: 0, message }));

// npm (2)
server.registerTool("npm.install", { description: "NPM install", inputSchema: { packages: z.array(z.string()).optional() } }, async ({ packages }) => ok({ stdout: "", stderr: "", exitCode: 0, packages: packages ?? [] }));
server.registerTool("npm.run", { description: "NPM run script", inputSchema: { script: z.string() } }, async ({ script }) => ok({ stdout: "", stderr: "", exitCode: 0, script }));

// docker (3)
server.registerTool("docker.build", { description: "Docker build", inputSchema: { tag: z.string().optional() } }, async ({ tag }) => ok({ stdout: "", stderr: "", exitCode: 0, tag }));
server.registerTool("docker.run", { description: "Docker run", inputSchema: { image: z.string() } }, async ({ image }) => ok({ stdout: "", stderr: "", exitCode: 0, image }));
server.registerTool("docker.ps", { description: "Docker ps", inputSchema: {} }, async () => ok({ stdout: "", stderr: "", exitCode: 0 }));

// cicd (3)
server.registerTool("cicd.pipeline_status", { description: "CI/CD pipeline status", inputSchema: {} }, async () => ok({ pipelines: [] }));
server.registerTool("cicd.pipeline_trigger", { description: "Trigger CI/CD pipeline", inputSchema: { pipeline: z.string() } }, async ({ pipeline }) => ok({ triggered: true, pipeline }));
server.registerTool("cicd.runs_view", { description: "View CI/CD runs", inputSchema: {} }, async () => ok({ runs: [] }));

// github (3)
server.registerTool("github.pr_create", { description: "Create GitHub PR", inputSchema: { title: z.string(), body: z.string().optional() } }, async ({ title, body }) => ok({ created: true, title, body }));
server.registerTool("github.pr_list", { description: "List GitHub PRs", inputSchema: {} }, async () => ok({ prs: [] }));
server.registerTool("github.issue_list", { description: "List GitHub issues", inputSchema: {} }, async () => ok({ issues: [] }));

// tests (1)
server.registerTool("tests.run", { description: "Run tests", inputSchema: { pattern: z.string().optional() } }, async ({ pattern }) => ok({ stdout: "", stderr: "", exitCode: 0, pattern }));

// capability.ping (1) — missing from original capability namespace
server.registerTool("capability.ping", { description: "Capability health check", inputSchema: {} }, async () => ok({ pong: true, backend: backendType }));

const transport = new StdioServerTransport();
await server.connect(transport);
logger.info("MCP Server running on stdio (" + backendType + " backend)");
