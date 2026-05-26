// ============================================================
// Workflow routes — CRUD, execution, task management
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';
import type { OrchestratingEngine } from '@mcp-rebuild/workflow-engine';

export interface DbReader {
  listWorkflows(): Promise<unknown[]>;
  getWorkflow(id: string): Promise<unknown | null>;
  getWorkflowTasks(workflowId: string): Promise<unknown[]>;
}

export function createWorkflowRoutes(engine: OrchestratingEngine, dbReader?: DbReader): RouteDefinition[] {
  return [
    // GET /api/workflows — List workflows
    {
      method: 'GET',
      pattern: '/api/workflows',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        if (dbReader) {
          try {
            const workflows = await dbReader.listWorkflows();
            sendJson(res, 200, { workflows });
            return;
          } catch { /* fall through to engine */ }
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
        sendJson(res, 201, { workflow });
      },
    },
    // GET /api/workflows/active — Get active workflow
    {
      method: 'GET',
      pattern: '/api/workflows/active',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse) => {
        if (dbReader) {
          try {
            const workflows = await dbReader.listWorkflows() as Array<Record<string, unknown>>;
            const active = workflows.find(
              (w) => w.status !== 'DONE' && w.status !== 'FAILED' && w.status !== 'PAUSED',
            );
            sendJson(res, 200, { workflow: active || null });
            return;
          } catch { /* fall through to engine */ }
        }
        const workflows = engine.listWorkflows();
        const active = workflows.find(
          (w) => w.state !== 'DONE' && w.state !== 'FAILED',
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
        try {
          const status = engine.getStatus(params.id);
          sendJson(res, 200, { status });
        } catch {
          sendJson(res, 404, { error: 'Workflow not found' });
        }
      },
    },
    // GET /api/workflows/:id — Get workflow
    {
      method: 'GET',
      pattern: '/api/workflows/:id',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        if (dbReader) {
          try {
            const workflow = await dbReader.getWorkflow(params.id);
            if (workflow) {
              sendJson(res, 200, { workflow });
              return;
            }
          } catch { /* fall through to engine */ }
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
          sendJson(res, 200, { workflow: result });
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : 'Execution failed';
          sendJson(res, 500, { error: message });
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
    // GET /api/workflows/:id/tasks — List tasks
    {
      method: 'GET',
      pattern: '/api/workflows/:id/tasks',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        if (dbReader) {
          try {
            const tasks = await dbReader.getWorkflowTasks(params.id);
            if (tasks.length > 0) {
              sendJson(res, 200, { tasks });
              return;
            }
          } catch { /* fall through to engine */ }
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
          sendJson(res, 201, { task });
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : 'Failed to add task';
          sendJson(res, 400, { error: message });
        }
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/start — Start task (update state)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/start',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>) => {
        const workflow = engine.getWorkflow(params.id);
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        const task = workflow.tasks.find((t) => t.id === params.taskId);
        if (!task) {
          sendJson(res, 404, { error: 'Task not found' });
          return;
        }
        const updatedTask = { ...task, state: 'running' as const, startedAt: new Date() };
        sendJson(res, 200, { task: updatedTask });
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/complete — Complete task (update state)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/complete',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body || {};
        const workflow = engine.getWorkflow(params.id);
        if (!workflow) {
          sendJson(res, 404, { error: 'Workflow not found' });
          return;
        }
        const task = workflow.tasks.find((t) => t.id === params.taskId);
        if (!task) {
          sendJson(res, 404, { error: 'Task not found' });
          return;
        }
        const updatedTask = { ...task, state: 'done' as const, output: input.output, completedAt: new Date() };
        sendJson(res, 200, { task: updatedTask });
      },
    },
    // POST /api/workflows/:id/tasks/:taskId/progress — Save progress (acknowledge)
    {
      method: 'POST',
      pattern: '/api/workflows/:id/tasks/:taskId/progress',
      authRequired: true,
      handler: async (_req: IncomingMessage, res: ServerResponse, params: Record<string, string>, body?: Record<string, unknown>) => {
        const input = body || {};
        sendJson(res, 200, {
          saved: true,
          workflowId: params.id,
          taskId: params.taskId,
          note: (input as Record<string, unknown>).progressNote || '',
        });
      },
    },
  ];
}
