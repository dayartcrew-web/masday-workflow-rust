import type { Workflow, Task } from '@mcp-rebuild/core';
import type { StorageBackend, IWorkflowStore, WorkflowRow, TaskRow } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('WorkflowStore');

function serializeWorkflow(w: Workflow): WorkflowRow {
  return {
    id: w.id,
    name: w.name,
    description: w.description,
    state: w.state,
    metadata: JSON.stringify({
      ...w.metadata,
      ...(w.traceId ? { traceId: w.traceId } : {}),
    }),
    created_at: w.createdAt.toISOString(),
    updated_at: w.updatedAt.toISOString(),
  };
}

function serializeTask(task: Task, workflowId: string): TaskRow {
  return {
    id: task.id,
    workflow_id: workflowId,
    name: task.name,
    agent: task.agent,
    skill: task.skill,
    dependencies: JSON.stringify(task.dependencies),
    state: task.state,
    input: task.input !== undefined ? JSON.stringify(task.input) : null,
    output: task.output !== undefined ? JSON.stringify(task.output) : null,
    error: task.error ?? null,
    created_at: task.createdAt.toISOString(),
    started_at: task.startedAt?.toISOString() ?? null,
    completed_at: task.completedAt?.toISOString() ?? null,
  };
}

function deserializeWorkflow(row: WorkflowRow, tasks: Task[]): Workflow {
  const parsed = JSON.parse(row.metadata) as Record<string, unknown>;
  const { traceId, ...metadata } = parsed;
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    state: row.state as Workflow['state'],
    tasks,
    metadata,
    traceId: traceId as string | undefined,
    createdAt: new Date(row.created_at),
    updatedAt: new Date(row.updated_at),
  };
}

function deserializeTask(row: TaskRow): Task {
  return {
    id: row.id,
    name: row.name,
    agent: row.agent,
    skill: row.skill,
    dependencies: row.dependencies ? JSON.parse(row.dependencies) : [],
    state: row.state as Task['state'],
    input: row.input !== null ? JSON.parse(row.input) : undefined,
    output: row.output !== null ? JSON.parse(row.output) : undefined,
    error: row.error ?? undefined,
    createdAt: new Date(row.created_at),
    startedAt: row.started_at ? new Date(row.started_at) : undefined,
    completedAt: row.completed_at ? new Date(row.completed_at) : undefined,
  };
}

function loadTasksForWorkflow(backend: StorageBackend, workflowId: string): Task[] {
  const rows = backend.query<TaskRow>(
    'SELECT * FROM tasks WHERE workflow_id = ? ORDER BY created_at',
    [workflowId]
  );
  return rows.map(deserializeTask);
}

export class WorkflowStore implements IWorkflowStore {
  private backend: StorageBackend;

  constructor(backend: StorageBackend) {
    this.backend = backend;
  }

  save(workflow: Workflow): void {
    const wfRow = serializeWorkflow(workflow);

    // Upsert workflow
    const existing = this.backend.queryOne('SELECT id FROM workflows WHERE id = ?', [workflow.id]);
    if (existing) {
      this.backend.run(
        'UPDATE workflows SET name = ?, description = ?, state = ?, metadata = ?, updated_at = ? WHERE id = ?',
        [wfRow.name, wfRow.description, wfRow.state, wfRow.metadata, wfRow.updated_at, wfRow.id]
      );
    } else {
      this.backend.run(
        'INSERT INTO workflows (id, name, description, state, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
        [wfRow.id, wfRow.name, wfRow.description, wfRow.state, wfRow.metadata, wfRow.created_at, wfRow.updated_at]
      );
    }

    // Delete old tasks and re-insert
    this.backend.run('DELETE FROM tasks WHERE workflow_id = ?', [workflow.id]);
    for (const task of workflow.tasks) {
      const taskRow = serializeTask(task, workflow.id);
      this.backend.run(
        'INSERT INTO tasks (id, workflow_id, name, agent, skill, dependencies, state, input, output, error, created_at, started_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
        [
          taskRow.id, taskRow.workflow_id, taskRow.name, taskRow.agent, taskRow.skill,
          taskRow.dependencies, taskRow.state, taskRow.input, taskRow.output, taskRow.error,
          taskRow.created_at, taskRow.started_at, taskRow.completed_at,
        ]
      );
    }

    logger.debug(`Saved workflow ${workflow.id} with ${workflow.tasks.length} tasks`);
  }

  load(id: string): Workflow | undefined {
    const row = this.backend.queryOne<WorkflowRow>('SELECT * FROM workflows WHERE id = ?', [id]);
    if (!row) return undefined;
    const tasks = loadTasksForWorkflow(this.backend, id);
    return deserializeWorkflow(row, tasks);
  }

  loadAll(): Workflow[] {
    const rows = this.backend.query<WorkflowRow>('SELECT * FROM workflows ORDER BY created_at');
    return rows.map(row => {
      const tasks = loadTasksForWorkflow(this.backend, row.id);
      return deserializeWorkflow(row, tasks);
    });
  }

  loadByState(state: string): Workflow[] {
    const rows = this.backend.query<WorkflowRow>('SELECT * FROM workflows WHERE state = ?', [state]);
    return rows.map(row => {
      const tasks = loadTasksForWorkflow(this.backend, row.id);
      return deserializeWorkflow(row, tasks);
    });
  }

  delete(id: string): void {
    this.backend.run('DELETE FROM tasks WHERE workflow_id = ?', [id]);
    this.backend.run('DELETE FROM workflows WHERE id = ?', [id]);
    logger.debug(`Deleted workflow ${id}`);
  }
}
