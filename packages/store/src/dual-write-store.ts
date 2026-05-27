import type { Workflow, Task } from '@mcp-rebuild/core';
import type { IWorkflowStore, ITaskResultStore } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('DualWriteStore');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let drizzleDb: any = null;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let schemaTables: any = null;

export function setDualWriteDb(client: unknown): void {
  drizzleDb = client;
}

export function setDualWriteSchema(tables: unknown): void {
  schemaTables = tables;
}

/**
 * DualWriteWorkflowStore wraps a primary sync store and replicates
 * writes to PostgreSQL via Drizzle (async, fire-and-forget).
 */
export class DualWriteWorkflowStore implements IWorkflowStore {
  private primary: IWorkflowStore;
  private pendingReplication: Promise<void> = Promise.resolve();

  constructor(primary: IWorkflowStore) {
    this.primary = primary;
  }

  save(workflow: Workflow): void {
    this.primary.save(workflow);
    this.pendingReplication = this.pendingReplication.then(() => this.replicateWorkflow(workflow)).catch((err: unknown) => {
      logger.warn({ err: String(err), workflowId: workflow.id }, 'Replication failed in save queue');
    });
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
    if (!drizzleDb || !schemaTables) return;

    const status = (workflow.state ?? 'INIT').toUpperCase();
    const rawMeta = workflow.metadata ?? {};
    let metaObj: Record<string, unknown> = {};
    if (typeof rawMeta === 'string') {
      try { metaObj = JSON.parse(rawMeta); } catch { metaObj = {}; }
    } else if (typeof rawMeta === 'object' && rawMeta !== null) {
      metaObj = rawMeta as Record<string, unknown>;
    }

    try {
      await drizzleDb.insert(schemaTables.workflows).values({
        id: workflow.id,
        name: workflow.name,
        status,
        metadata: { description: workflow.description, traceId: workflow.traceId, ...metaObj },
        updatedAt: new Date(),
      }).onConflictDoUpdate({
        target: schemaTables.workflows.id,
        set: {
          name: workflow.name,
          status,
          metadata: { description: workflow.description, traceId: workflow.traceId, ...metaObj },
          updatedAt: new Date(),
        },
      });
    } catch (err: unknown) {
      logger.warn({ err: String(err), workflowId: workflow.id }, 'Failed to replicate workflow to PostgreSQL');
      return;
    }

    for (const task of workflow.tasks) {
      await this.replicateTask(workflow.id, task);
    }
  }

  private async replicateTask(workflowId: string, task: Task): Promise<void> {
    if (!drizzleDb || !schemaTables) return;

    const planId = `plan-default-${workflowId}`;

    try {
      await drizzleDb.insert(schemaTables.plans).values({
        id: planId,
        workflowId,
        version: 1,
        status: 'ACTIVE',
        summary: 'Default auto-created plan',
        content: { tasks: [] },
        createdByAgent: 'system',
      }).onConflictDoNothing();
    } catch { /* plan may already exist */ }

    const rawStatus = task.state ?? 'pending';
    const status = rawStatus.toUpperCase();
    const title = task.name || `Task ${task.id.slice(0, 8)}`;

    try {
      await drizzleDb.insert(schemaTables.tasks).values({
        id: task.id,
        workflowId,
        planId,
        title,
        status,
        ownerAgent: task.agent ?? null,
        acceptanceCriteria: [],
        requiredContext: [],
        verificationSteps: [],
        updatedAt: new Date(),
      }).onConflictDoUpdate({
        target: schemaTables.tasks.id,
        set: {
          title,
          status,
          ownerAgent: task.agent ?? null,
          updatedAt: new Date(),
        },
      });
    } catch (err: unknown) {
      logger.warn({ err: String(err), taskId: task.id }, 'Failed to replicate task to PostgreSQL');
    }
  }

  private replicateDelete(workflowId: string): void {
    if (!drizzleDb || !schemaTables) return;

    import('drizzle-orm').then(({ eq }) => {
      drizzleDb.delete(schemaTables.tasks).where(eq(schemaTables.tasks.workflowId, workflowId))
        .then(() => drizzleDb.delete(schemaTables.plans).where(eq(schemaTables.plans.workflowId, workflowId)))
        .then(() => drizzleDb.delete(schemaTables.workflows).where(eq(schemaTables.workflows.id, workflowId)))
        .catch((err: unknown) => {
          logger.warn({ err: String(err), workflowId }, 'Failed to replicate workflow deletion to PostgreSQL');
        });
    }).catch(() => {});
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
    if (drizzleDb && schemaTables) {
      import('drizzle-orm').then(({ eq }) => {
        drizzleDb.delete(schemaTables.tasks).where(eq(schemaTables.tasks.workflowId, workflowId)).catch((err: unknown) => {
          logger.warn({ err: String(err), workflowId }, 'Failed to replicate task deletion to PostgreSQL');
        });
      }).catch(() => {});
    }
  }

  private async replicateTask(workflowId: string, task: Task): Promise<void> {
    if (!drizzleDb || !schemaTables) return;

    const planId = `plan-default-${workflowId}`;
    const rawStatus = task.state ?? 'pending';
    const status = rawStatus.toUpperCase();
    const title = task.name || `Task ${task.id.slice(0, 8)}`;

    try {
      await drizzleDb.insert(schemaTables.tasks).values({
        id: task.id,
        workflowId,
        planId,
        title,
        status,
        ownerAgent: task.agent ?? null,
        acceptanceCriteria: [],
        requiredContext: [],
        verificationSteps: [],
        updatedAt: new Date(),
      }).onConflictDoUpdate({
        target: schemaTables.tasks.id,
        set: {
          title,
          status,
          ownerAgent: task.agent ?? null,
          updatedAt: new Date(),
        },
      });
    } catch (err: unknown) {
      logger.warn({ err: String(err), taskId: task.id }, 'Failed to replicate task to PostgreSQL');
    }
  }
}
