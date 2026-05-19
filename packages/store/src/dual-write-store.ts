import type { Workflow, Task } from '@mcp-rebuild/core';
import type { IWorkflowStore, ITaskResultStore } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('DualWriteStore');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let prismaClient: any = null;

export function setDualWritePrisma(client: unknown): void {
  prismaClient = client;
}

/**
 * DualWriteWorkflowStore wraps a primary sync store and replicates
 * writes to PostgreSQL via Prisma (async, fire-and-forget).
 *
 * Reads always go to the primary store (JsonBackend) for speed.
 * Writes go to both: primary (sync, for engine) + PostgreSQL (async, for MCP/dashboard).
 */
export class DualWriteWorkflowStore implements IWorkflowStore {
  private primary: IWorkflowStore;

  constructor(primary: IWorkflowStore) {
    this.primary = primary;
  }

  save(workflow: Workflow): void {
    this.primary.save(workflow);
    this.replicateWorkflow(workflow);
  }

  load(id: string): Workflow | undefined {
    return this.primary.load(id);
  }

  loadAll(): Workflow[] {
    return this.primary.loadAll();
  }

  loadByState(state: string): Workflow[] {
    return this.primary.loadByState(state);
  }

  delete(id: string): void {
    this.primary.delete(id);
    this.replicateDelete(id);
  }

  private async replicateWorkflow(workflow: Workflow): Promise<void> {
    if (!prismaClient) return;

    const status = workflow.state ?? 'INIT';
    const rawMeta = workflow.metadata ?? {};
    let metaObj: Record<string, unknown> = {};
    if (typeof rawMeta === 'string') {
      try { metaObj = JSON.parse(rawMeta); } catch { metaObj = {}; }
    } else if (typeof rawMeta === 'object' && rawMeta !== null) {
      metaObj = rawMeta as Record<string, unknown>;
    }

    try {
      await prismaClient.workflow.upsert({
        where: { id: workflow.id },
        update: {
          name: workflow.name,
          status,
          metadata: { description: workflow.description, traceId: workflow.traceId, ...metaObj },
          updatedAt: new Date(),
        },
        create: {
          id: workflow.id,
          name: workflow.name,
          status,
          metadata: {
            description: workflow.description,
            traceId: workflow.traceId,
            ...metaObj,
          },
          createdAt: typeof workflow.createdAt === 'string' ? new Date(workflow.createdAt) : workflow.createdAt,
          updatedAt: typeof workflow.updatedAt === 'string' ? new Date(workflow.updatedAt) : workflow.updatedAt,
        },
      });
    } catch (err: unknown) {
      logger.warn({ err: String(err), workflowId: workflow.id }, 'Failed to replicate workflow to PostgreSQL');
      return;
    }

    for (const task of workflow.tasks) {
      this.replicateTask(workflow.id, task);
    }
  }

  private async replicateTask(workflowId: string, task: Task): Promise<void> {
    if (!prismaClient) return;

    const planId = `plan-default-${workflowId}`;

    try {
      await prismaClient.plan.upsert({
        where: { id: planId },
        update: {},
        create: {
          id: planId,
          workflowId,
          version: 1,
          status: 'active',
          summary: 'Default auto-created plan',
          content: { tasks: [] },
          createdByAgent: 'system',
        },
      });
    } catch { /* plan may already exist */ }

    const status = task.state ?? 'pending';
    const title = task.name || `Task ${task.id.slice(0, 8)}`;

    prismaClient.task.upsert({
      where: { id: task.id },
      update: {
        title,
        status,
        ownerAgent: task.agent ?? null,
        updatedAt: new Date(),
      },
      create: {
        id: task.id,
        workflowId,
        planId,
        title,
        status,
        ownerAgent: task.agent ?? null,
        acceptanceCriteria: [],
        requiredContext: [],
        verificationSteps: [],
        createdAt: typeof task.createdAt === 'string' ? new Date(task.createdAt) : (task.createdAt ?? new Date()),
        updatedAt: new Date(),
      },
    }).catch((err: unknown) => {
      logger.warn({ err: String(err), taskId: task.id }, 'Failed to replicate task to PostgreSQL');
    });
  }

  private replicateDelete(workflowId: string): void {
    if (!prismaClient) return;

    prismaClient.task.deleteMany({ where: { workflowId } })
      .then(() => prismaClient.plan.deleteMany({ where: { workflowId } }))
      .then(() => prismaClient.workflow.deleteMany({ where: { id: workflowId } }))
      .catch((err: unknown) => {
        logger.warn({ err: String(err), workflowId }, 'Failed to replicate workflow deletion to PostgreSQL');
      });
  }
}

/**
 * DualWriteTaskResultStore wraps a primary sync store and replicates
 * task writes to PostgreSQL.
 */
export class DualWriteTaskResultStore implements ITaskResultStore {
  private primary: ITaskResultStore;

  constructor(primary: ITaskResultStore) {
    this.primary = primary;
  }

  saveTask(workflowId: string, task: Task): void {
    this.primary.saveTask(workflowId, task);
    this.replicateTask(workflowId, task);
  }

  loadTasks(workflowId: string): Task[] {
    return this.primary.loadTasks(workflowId);
  }

  loadTask(taskId: string): Task | undefined {
    return this.primary.loadTask(taskId);
  }

  deleteTasks(workflowId: string): void {
    this.primary.deleteTasks(workflowId);
    if (prismaClient) {
      prismaClient.task.deleteMany({ where: { workflowId } }).catch((err: unknown) => {
        logger.warn({ err: String(err), workflowId }, 'Failed to replicate task deletion to PostgreSQL');
      });
    }
  }

  private replicateTask(workflowId: string, task: Task): void {
    if (!prismaClient) return;

    const planId = `plan-default-${workflowId}`;
    const status = task.state ?? 'pending';
    const title = task.name || `Task ${task.id.slice(0, 8)}`;

    prismaClient.task.upsert({
      where: { id: task.id },
      update: {
        title,
        status,
        ownerAgent: task.agent ?? null,
        updatedAt: new Date(),
      },
      create: {
        id: task.id,
        workflowId,
        planId,
        title,
        status,
        ownerAgent: task.agent ?? null,
        acceptanceCriteria: [],
        requiredContext: [],
        verificationSteps: [],
        createdAt: typeof task.createdAt === 'string' ? new Date(task.createdAt) : (task.createdAt ?? new Date()),
        updatedAt: new Date(),
      },
    }).catch((err: unknown) => {
      logger.warn({ err: String(err), taskId: task.id }, 'Failed to replicate task to PostgreSQL');
    });
  }
}
