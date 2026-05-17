import type { Workflow, Task } from '@mcp-rebuild/core';

// --- Storage Backend ---

export interface RunResult {
  changes: number;
  lastInsertRowid: number | bigint;
}

export interface StorageBackend {
  initialize(): void;
  close(): void;
  run(sql: string, params?: unknown[]): RunResult;
  query<T = Record<string, unknown>>(sql: string, params?: unknown[]): T[];
  queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): T | undefined;
}

// --- Store Interfaces ---

export interface IWorkflowStore {
  save(workflow: Workflow): void;
  load(id: string): Workflow | undefined;
  loadAll(): Workflow[];
  loadByState(state: string): Workflow[];
  delete(id: string): void;
}

export interface ITaskResultStore {
  saveTask(workflowId: string, task: Task): void;
  loadTasks(workflowId: string): Task[];
  loadTask(taskId: string): Task | undefined;
  deleteTasks(workflowId: string): void;
}

export interface IConfigStore {
  get(key: string): string | undefined;
  set(key: string, value: string): void;
  delete(key: string): void;
  getAll(): Map<string, string>;
}

// --- Serialized Row Types ---

export interface WorkflowRow {
  id: string;
  name: string;
  description: string;
  state: string;
  metadata: string;
  created_at: string;
  updated_at: string;
}

export interface TaskRow {
  id: string;
  workflow_id: string;
  name: string;
  agent: string;
  skill: string;
  dependencies: string;
  state: string;
  input: string | null;
  output: string | null;
  error: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export interface ConfigRow {
  key: string;
  value: string;
  updated_at: string;
}
