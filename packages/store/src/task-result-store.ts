import type { Task } from '@mcp-rebuild/core';
import type { StorageBackend, ITaskResultStore, TaskRow } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('TaskResultStore');

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

function deserializeTask(row: TaskRow): Task {
  return {
    id: row.id,
    name: row.name,
    agent: row.agent,
    skill: row.skill,
    dependencies: JSON.parse(row.dependencies),
    state: row.state as Task['state'],
    input: row.input !== null ? JSON.parse(row.input) : undefined,
    output: row.output !== null ? JSON.parse(row.output) : undefined,
    error: row.error ?? undefined,
    createdAt: new Date(row.created_at),
    startedAt: row.started_at ? new Date(row.started_at) : undefined,
    completedAt: row.completed_at ? new Date(row.completed_at) : undefined,
  };
}

export class TaskResultStore implements ITaskResultStore {
  private backend: StorageBackend;

  constructor(backend: StorageBackend) {
    this.backend = backend;
  }

  saveTask(workflowId: string, task: Task): void {
    const row = serializeTask(task, workflowId);
    const existing = this.backend.queryOne('SELECT id FROM tasks WHERE id = ?', [task.id]);
    if (existing) {
      this.backend.run(
        'UPDATE tasks SET name = ?, agent = ?, skill = ?, dependencies = ?, state = ?, input = ?, output = ?, error = ?, started_at = ?, completed_at = ? WHERE id = ?',
        [
          row.name, row.agent, row.skill, row.dependencies, row.state,
          row.input, row.output, row.error, row.started_at, row.completed_at, row.id,
        ]
      );
    } else {
      this.backend.run(
        'INSERT INTO tasks (id, workflow_id, name, agent, skill, dependencies, state, input, output, error, created_at, started_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
        [
          row.id, row.workflow_id, row.name, row.agent, row.skill,
          row.dependencies, row.state, row.input, row.output, row.error,
          row.created_at, row.started_at, row.completed_at,
        ]
      );
    }
    logger.debug(`Saved task ${task.id} for workflow ${workflowId}`);
  }

  loadTasks(workflowId: string): Task[] {
    const rows = this.backend.query<TaskRow>(
      'SELECT * FROM tasks WHERE workflow_id = ? ORDER BY created_at',
      [workflowId]
    );
    return rows.map(deserializeTask);
  }

  loadTask(taskId: string): Task | undefined {
    const row = this.backend.queryOne<TaskRow>('SELECT * FROM tasks WHERE id = ?', [taskId]);
    return row ? deserializeTask(row) : undefined;
  }

  deleteTasks(workflowId: string): void {
    this.backend.run('DELETE FROM tasks WHERE workflow_id = ?', [workflowId]);
    logger.debug(`Deleted tasks for workflow ${workflowId}`);
  }
}
