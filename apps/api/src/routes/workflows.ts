// ============================================================
// Workflow routes — CRUD, execution, task management
// Uses Drizzle for direct DB reads when available, falls back to engine
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';
import type { OrchestratingEngine } from '@mcp-rebuild/workflow-engine';

// Drizzle ORM 0.45 has ESM/CJS dual-package type resolution conflicts in monorepos.
// Use lazy init to avoid top-level await in CommonJS contexts.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let db: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let wfTable: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let taskTable: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let progressTable: any = null;
let dbInitialized = false;

async function ensureDb(): Promise<void> {
  if (dbInitialized) return;
  dbInitialized = true;
  try {
    const mod = await import('@mcp-rebuild/db');
    db = mod.db;
    wfTable = mod.workflows;
    taskTable = mod.tasks;
    progressTable = mod.taskProgressLogs;
  } catch {
    // DB not available — routes will fall back to engine
  }
}

interface EngineWorkflow {
  id: string;
  name: string;
  state: string;
  description?: string;
  metadata?: Record<string, unknown>;
  tasks: EngineTask[];
  createdAt?: Date;
  updatedAt?: Date;
}

interface EngineTask {
  id: string;
  name: string;
  state: string;
  agent: string;
  skill: string;
  dependencies: string[];
  input?: unknown;
  output?: unknown;
  startedAt?: Date;
  completedAt?: Date;
}

async function syncWorkflowToDb(id: string, workflow: EngineWorkflow): Promise<void> {
  await ensureDb();
  if (!db || !wfTable || !taskTable) return;
  try {
    const { eq } = await import('drizzle-orm');
    const wfState = workflow.state?.toUpperCase() ?? 'INIT';
    await db.update(wfTable)
      .set({ status: wfState, updatedAt: new Date() })
      .where(eq(wfTable.id, id));
    for (const t of workflow.tasks ?? []) {
      await db.update(taskTable)
        .set({ status: t.state?.toUpperCase() ?? 'PENDING', updatedAt: new Date() })
        .where(eq(taskTable.id, t.id));
    }
  } catch {
    // non-critical: engine state is authoritative
  }
}

export function createWorkflowRoutes(engine: OrchestratingEngine): RouteDefinition[] {
  return [
    // GET /api/workflows — List workflows (Drizzle first, engine fallback)
    {
      method: 'GET',
      pattern: '/api/workflows',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        await ensureDb();
        if (db && wfTable && taskTable) {
          try {
            const { desc, sql } = await import('drizzle-orm');
            const rows = await db.select().from(wfTable).orderBy(desc(wfTable.updatedAt));
            const wfIds = rows.map((r: Record<string, unknown>) => r.id as string);
            // Batch-fetch all tasks for these workflows
            const allTasks = wfIds.length > 0
              ? await db.select().from(taskTable).where(sql`${taskTable.workflowId} IN (${sql.join(wfIds.map((id: string) => sql`${id}`), sql`, `)})`)
              : [];
            const tasksByWf = new Map<string, Record<string, unknown>[]>();
            for (const t of allTasks) {
              const arr = tasksByWf.get(t.workflowId as string) ?? [];
              arr.push(t);
              tasksByWf.set(t.workflowId as string, arr);
            }
            const workflows = rows.map((r: Record<string, unknown>) => ({
              id: r.id,
              name: r.name,
              state: r.status,
              projectPath: r.projectPath,
              currentPlanId: r.currentPlanId,
              currentTaskId: r.currentTaskId,
              metadata: r.metadata ?? {},
              description: (r.metadata as Record<string, unknown>)?.description ?? '',
              tasks: (tasksByWf.get(r.id as string) ?? []).map((t: Record<string, unknown>) => ({
                id: t.id,
                name: t.title,
                state: (t.status as string)?.toLowerCase() ?? 'pending',
                agent: t.ownerAgent ?? '',
                skill: '',
                dependencies: [],
                output: null,
              })),
              createdAt: r.createdAt,
              updatedAt: r.updatedAt,
            }));
            sendJson(res, 200, { workflows });
            return;
          } catch {
            // fall through to engine fallback
          }
        }
        const workflows = engine.listWorkflows();
        sendJson(res, 200, { workflows });
      },
    },
    // POST /api/workflows — Create workflow
    {
      method: 'POST',
      pattern: '/api/workflows',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
        const input = body!;
        const workflow = engine.createWorkflow(
          input.name as string,
          input.description as string,
          (input.metadata as Record<string, unknown>) || {},
        );

        // Persist to Drizzle so GET reads it
        await ensureDb();
        if (db && wfTable) {
          try {
            await db.insert(wfTable).values({
              id: workflow.id,
              name: workflow.name,
              status: workflow.state?.toUpperCase() ?? 'INIT',
              metadata: { description: workflow.description ?? '', ...((input.metadata as Record<string, unknown>) || {}) },
              updatedAt: new Date(),
            });
          } catch (insertErr: unknown) {
            const msg = insertErr instanceof Error ? insertErr.message : String(insertErr);
            console.error('[POST /workflows] Drizzle insert failed:', msg);
          }
        }

        sendJson(res, 201, { workflow });
      },
    },
    // GET /api/workflows/active — Get active workflow (Drizzle first, engine fallback)
    {
      method: 'GET',
      pattern: '/api/workflows/active',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        await ensureDb();
        if (db && wfTable && taskTable) {
          try {
            const { sql, desc, eq } = await import('drizzle-orm');
            const rows = await db.select().from(wfTable)
              .where(sql`${wfTable.status} NOT IN ('DONE', 'FAILED')`)
              .orderBy(desc(wfTable.updatedAt))
              .limit(1);
            if (rows.length > 0) {
              const r = rows[0] as Record<string, unknown>;
              const taskRows = await db.select().from(taskTable).where(eq(taskTable.workflowId, r.id as string));
              sendJson(res, 200, {
                workflow: {
                  id: r.id,
                  name: r.name,
                  state: r.status,
                  projectPath: r.projectPath,
                  currentPlanId: r.currentPlanId,
                  currentTaskId: r.currentTaskId,
                  metadata: r.metadata ?? {},
                  description: (r.metadata as Record<string, unknown>)?.description ?? '',
                  tasks: taskRows.map((t: Record<string, unknown>) => ({
                    id: t.id,
                    name: t.title,
                    state: (t.status as string)?.toLowerCase() ?? 'pending',
                    agent: t.ownerAgent ?? '',
                    skill: '',
                    dependencies: [],
                    output: null,
                  })),
                  createdAt: r.createdAt,
                  updatedAt: r.updatedAt,
                },
              });
              return;
            }
            sendJson(res, 200, { workflow: null });
            return;
          } catch {
            // fall through to engine fallback
          }
        }
        const workflows = engine.listWorkflows();
        const active = workflows.find(
          (w: EngineWorkflow) => w.state !== 'DONE' && w.state !== 'FAILED',
        );
        sendJson(res, 200, { workflow: active || null });
      },
    },
    // GET /api/workflows/:id/status — Get workflow status
    {
      method: 'GET',
      pattern: '/api/workflows/:id/status',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        await ensureDb();
        if (db && wfTable && taskTable) {
          try {
            const { eq } = await import('drizzle-orm');
            const [wf] = await db.select().from(wfTable).where(eq(wfTable.id, params.id)).limit(1);
            if (!wf) {
              sendJson(res, 404, { error: 'Workflow not found' });
              return;
            }
            const taskRows = await db.select().from(taskTable).where(eq(taskTable.workflowId, params.id));
            sendJson(res, 200, {
              status: {
                id: wf.id,
                name: wf.name,
                state: wf.status,
                taskCount: taskRows.length,
                tasksCompleted: taskRows.filter((t: Record<string, unknown>) => t.status === 'DONE').length,
                tasksRunning: taskRows.filter((t: Record<string, unknown>) => t.status === 'RUNNING').length,
                updatedAt: wf.updatedAt,
              },
            });
            return;
          } catch {
            // fall through to engine fallback
          }
        }
        try {
          const status = engine.getStatus(params.id);
          sendJson(res, 200, { status });
        } catch {
          sendJson(res, 404, { error: 'Workflow not found' });
        }
      },
    },
    // GET /api/workflows/:id — Get workflow (Drizzle first, engine fallback)
    {
      method: 'GET',
      pattern: '/api/workflows/:id',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        await ensureDb();
        if (db && wfTable && taskTable) {
          try {
            const { eq } = await import('drizzle-orm');
            const [wf] = await db.select().from(wfTable).where(eq(wfTable.id, params.id)).limit(1);
            if (!wf) {
              sendJson(res, 404, { error: 'Workflow not found' });
              return;
            }
            const taskRows = await db.select().from(taskTable).where(eq(taskTable.workflowId, params.id));
            sendJson(res, 200, {
              workflow: {
                id: wf.id,
                name: wf.name,
                state: wf.status,
                projectPath: wf.projectPath,
                currentPlanId: wf.currentPlanId,
                currentTaskId: wf.currentTaskId,
                metadata: wf.metadata ?? {},
                tasks: taskRows.map((t: Record<string, unknown>) => ({
                  id: t.id,
                  name: t.title,
                  state: t.status,
                  agent: t.ownerAgent ?? '',
                  skill: '',
                  dependencies: [],
                  priority: t.priority,
                  progressPercent: t.progressPercent,
                  acceptanceCriteria: t.acceptanceCriteria,
                  requiresTdd: t.requiresTdd,
                  testEvidence: t.testEvidence,
                })),
                createdAt: wf.createdAt,
                updatedAt: wf.updatedAt,
              },
            });
            return;
          } catch {
            // fall through to engine fallback
          }
        }
        const workflow = engine.getWorkflow(params.id);
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        sendJson(res, 200, { workflow });
      },
    },
    // POST /api/workflows/:id/execute — Execute workflow
    {
      method: 'POST',
      pattern: '/api/workflows/:id/execute',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        try {
          const result = await engine.executeWorkflow(params.id);
          await syncWorkflowToDb(params.id, result as EngineWorkflow);
          sendJson(res, 200, { workflow: result });
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : 'Execution failed';

          // If engine rejects due to state mismatch, sync engine→DB and return current state
          if (message.includes('Invalid state transition')) {
            const workflow = engine.getWorkflow(params.id) as EngineWorkflow | undefined;
            if (workflow) {
              await syncWorkflowToDb(params.id, workflow);
              sendJson(res, 200, { workflow });
              return;
            }
          }

          // If engine doesn't know the workflow (server restart), check Drizzle
          if (message.includes('not found')) {
            await ensureDb();
            if (db && wfTable && taskTable) {
              try {
                const { eq } = await import('drizzle-orm');
                const [wf] = await db.select().from(wfTable).where(eq(wfTable.id, params.id)).limit(1);
                if (wf) {
                  const taskRows = await db.select().from(taskTable).where(eq(taskTable.workflowId, params.id));
                  // If already terminal state, return it
                  if (wf.status === 'DONE' || wf.status === 'FAILED') {
                    sendJson(res, 200, {
                      workflow: {
                        id: wf.id, name: wf.name, state: wf.status, description: (wf.metadata as Record<string, unknown>)?.description ?? '',
                        metadata: wf.metadata ?? {}, tasks: taskRows.map((t: Record<string, unknown>) => ({
                          id: t.id, name: t.title, state: (t.status as string)?.toLowerCase() ?? 'pending',
                          agent: t.ownerAgent ?? '', skill: '', dependencies: [],
                        })),
                        createdAt: wf.createdAt, updatedAt: wf.updatedAt,
                      },
                    });
                    return;
                  }
                  // Non-terminal: cannot execute without engine state
                  sendJson(res, 422, { error: 'Workflow state lost on server restart. Create a new workflow to continue.', workflowId: params.id, dbState: wf.status });
                  return;
                }
              } catch { /* fall through to 404 */ }
            }
            sendJson(res, 404, { error: 'Workflow not found' });
            return;
          }

          const isClientError = message.includes('stuck') || message.includes('no ready tasks');
          sendJson(res, isClientError ? 422 : 500, { error: message });
        }
      },
    },
    // POST /api/workflows/:id/plan — Create plan (adds tasks to workflow)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/plan',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body!;
        const taskSpecs = (input.tasks as Array<Record<string, unknown>>) || [];
        const workflow = engine.getWorkflow(params.id);

        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }

        const createdTasks = [];
        for (const spec of taskSpecs) {
          const task = engine.addTask(params.id, {
            name: spec.name as string,
            agent: (spec.ownerAgent as string) || 'default',
            skill: (spec.title as string) || 'default',
            dependencies: [],
            input: spec,
          });
          createdTasks.push(task);
        }

        sendJson(res, 200, { plan: { tasks: createdTasks, taskCount: createdTasks.length } });
      },
    },
    // GET /api/workflows/:id/tasks — List tasks (Drizzle first, engine fallback)
    {
      method: 'GET',
      pattern: '/api/workflows/:id/tasks',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        await ensureDb();
        if (db && taskTable) {
          try {
            const { eq, desc } = await import('drizzle-orm');
            const rows = await db.select().from(taskTable)
              .where(eq(taskTable.workflowId, params.id))
              .orderBy(desc(taskTable.updatedAt));
            sendJson(res, 200, {
              tasks: rows.map((t: Record<string, unknown>) => ({
                id: t.id,
                name: t.title,
                state: t.status,
                agent: t.ownerAgent ?? '',
                skill: '',
                priority: t.priority,
                progressPercent: t.progressPercent,
                acceptanceCriteria: t.acceptanceCriteria,
                requiresTdd: t.requiresTdd,
                testEvidence: t.testEvidence,
                contextFingerprint: t.contextFingerprint,
                createdAt: t.createdAt,
                updatedAt: t.updatedAt,
              })),
            });
            return;
          } catch {
            // fall through to engine fallback
          }
        }
        const workflow = engine.getWorkflow(params.id);
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        sendJson(res, 200, { tasks: workflow.tasks });
      },
    },
    // POST /api/workflows/:id/tasks — Add task
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body!;
        try {
          const task = engine.addTask(params.id, {
            name: input.name as string,
            agent: input.agent as string,
            skill: input.skill as string,
            dependencies: (input.dependencies as string[]) || [],
            input: input.input,
          });

          // Persist to Drizzle so GET reads it
          await ensureDb();
          if (db && taskTable && wfTable) {
            try {
              const { eq } = await import('drizzle-orm');
              // Ensure a default plan exists for this workflow
              const [wfRow] = await db.select({ currentPlanId: wfTable.currentPlanId })
                .from(wfTable).where(eq(wfTable.id, params.id)).limit(1);
              let planId = wfRow?.currentPlanId as string | null;
              if (!planId) {
                planId = `plan_${params.id}_default`;
                const { plans } = await import('@mcp-rebuild/db');
                await db.insert(plans).values({
                  id: planId,
                  workflowId: params.id,
                  version: 1,
                  status: 'ACTIVE',
                  summary: 'Auto-created plan',
                  content: {},
                  createdByAgent: 'api',
                }).onConflictDoNothing();
                await db.update(wfTable)
                  .set({ currentPlanId: planId, updatedAt: new Date() })
                  .where(eq(wfTable.id, params.id));
              }
              await db.insert(taskTable).values({
                id: task.id,
                workflowId: params.id,
                planId,
                title: task.name,
                status: 'PENDING',
                ownerAgent: task.agent,
                acceptanceCriteria: [],
                requiredContext: [],
                verificationSteps: [],
                progressPercent: 0,
                requiresTdd: false,
                testEvidence: {},
                updatedAt: new Date(),
              });
            } catch (taskErr: unknown) {
              const msg = taskErr instanceof Error ? taskErr.message : String(taskErr);
              console.error('[POST /tasks] Drizzle insert failed:', msg);
            }
          }

          sendJson(res, 201, { task });
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : 'Failed to add task';
          sendJson(res, 400, { error: message });
        }
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/start — Start task (update state + Drizzle)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/start',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        await ensureDb();
        const workflow = engine.getWorkflow(params.id) as EngineWorkflow | undefined;
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        const task = workflow.tasks.find((t: EngineTask) => t.id === params.taskId);
        if (!task) {
          sendJson(res, 404, { error: 'Task not found' });
          return;
        }
        task.state = 'running';
        task.startedAt = new Date();

        // Persist to Drizzle
        if (db && taskTable) {
          try {
            const { eq } = await import('drizzle-orm');
            await db.update(taskTable)
              .set({ status: 'RUNNING', updatedAt: new Date() })
              .where(eq(taskTable.id, params.taskId));
          } catch {
            // non-critical: engine state is authoritative
          }
        }

        sendJson(res, 200, { task });
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/complete — Complete task (update state + Drizzle)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/complete',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        await ensureDb();
        const input = body || {};
        const workflow = engine.getWorkflow(params.id) as EngineWorkflow | undefined;
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        const task = workflow.tasks.find((t: EngineTask) => t.id === params.taskId);
        if (!task) {
          sendJson(res, 404, { error: 'Task not found' });
          return;
        }
        task.state = 'done';
        task.output = input.output;
        task.completedAt = new Date();

        // Persist to Drizzle
        if (db && taskTable && wfTable) {
          try {
            const { eq, and, sql } = await import('drizzle-orm');
            await db.update(taskTable)
              .set({ status: 'DONE', progressPercent: 100, updatedAt: new Date() })
              .where(eq(taskTable.id, params.taskId));
            // Check if all tasks done -> mark workflow DONE
            const remaining = await db.select({ id: taskTable.id, status: taskTable.status })
              .from(taskTable)
              .where(and(
                eq(taskTable.workflowId, params.id),
                sql`${taskTable.status} != 'DONE'`,
              ));
            if (remaining.length === 0) {
              await db.update(wfTable)
                .set({ status: 'DONE', updatedAt: new Date() })
                .where(eq(wfTable.id, params.id));
            }
          } catch {
            // non-critical: engine state is authoritative
          }
        }

        sendJson(res, 200, { task });
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/progress — Save progress (Drizzle + engine)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/progress',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        await ensureDb();
        const input = body || {};
        const progressNote = (input as Record<string, unknown>).progressNote as string ?? '';

        // Persist progress log to Drizzle
        if (db && progressTable && taskTable) {
          try {
            const { eq } = await import('drizzle-orm');
            await db.insert(progressTable).values({
              workflowId: params.id,
              taskId: params.taskId,
              agentName: (input as Record<string, unknown>).agentName as string ?? 'api',
              statusBefore: (input as Record<string, unknown>).statusBefore as string ?? null,
              statusAfter: (input as Record<string, unknown>).statusAfter as string ?? 'RUNNING',
              progressNote,
              evidence: ((input as Record<string, unknown>).evidence as string[]) ?? [],
            });
            // Update task progress percent if provided
            const pct = (input as Record<string, unknown>).progressPercent as number | undefined;
            if (pct !== undefined) {
              await db.update(taskTable)
                .set({ progressPercent: pct, updatedAt: new Date() })
                .where(eq(taskTable.id, params.taskId));
            }
          } catch {
            // non-critical
          }
        }

        sendJson(res, 200, {
          saved: true,
          workflowId: params.id,
          taskId: params.taskId,
          note: progressNote,
        });
      },
    },
  ];
}
