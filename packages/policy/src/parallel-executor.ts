/**
 * Parallel Execution Branches (StorageBackend-backed)
 *
 * Manages parallel execution branches within a task.
 * Each branch has a role, input, and output when completed.
 *
 * Ported from masday-workflow-reborn/packages/orchestrator/src/parallel.ts
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import { createLogger } from '@mcp-rebuild/core';
import { randomUUID } from 'crypto';

const logger = createLogger('ParallelExecutor');

const BRANCHES_TABLE = 'parallel_branches';

const CREATE_TABLE_SQL = `
CREATE TABLE IF NOT EXISTS ${BRANCHES_TABLE} (
  id            TEXT PRIMARY KEY,
  workflow_id   TEXT NOT NULL,
  task_id       TEXT NOT NULL,
  branch_key    TEXT NOT NULL,
  role          TEXT NOT NULL,
  input         TEXT,
  output        TEXT,
  status        TEXT NOT NULL DEFAULT 'pending',
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
)`;

export interface Branch {
  id: string;
  workflowId: string;
  taskId: string;
  branchKey: string;
  role: string;
  input: string | null;
  output: string | null;
  status: 'pending' | 'completed' | 'failed';
}

interface BranchRow {
  id: string;
  workflow_id: string;
  task_id: string;
  branch_key: string;
  role: string;
  input: string | null;
  output: string | null;
  status: string;
  created_at: string;
}

function rowToBranch(row: BranchRow): Branch {
  return {
    id: row.id,
    workflowId: row.workflow_id,
    taskId: row.task_id,
    branchKey: row.branch_key,
    role: row.role,
    input: row.input,
    output: row.output,
    status: row.status as Branch['status'],
  };
}

export interface CreateBranchInput {
  workflowId: string;
  taskId: string;
  branches: Array<{ branchKey: string; role: string; input?: string }>;
}

export interface CompleteBranchInput {
  branchId: string;
  output: string;
}

export class ParallelExecutor {
  private storage: StorageBackend;
  private initialized = false;

  constructor(storage: StorageBackend) {
    this.storage = storage;
  }

  init(): void {
    if (this.initialized) return;
    this.storage.run(CREATE_TABLE_SQL);
    this.initialized = true;
    logger.info('ParallelExecutor initialized');
  }

  /** Create parallel branches for a task. */
  async createBranches(input: CreateBranchInput): Promise<Branch[]> {
    this.ensureInit();
    const branches: Branch[] = [];

    for (const spec of input.branches) {
      const id = randomUUID();
      this.storage.run(
        `INSERT INTO ${BRANCHES_TABLE} (id, workflow_id, task_id, branch_key, role, input, status) VALUES (?, ?, ?, ?, ?, ?, 'pending')`,
        [id, input.workflowId, input.taskId, spec.branchKey, spec.role, spec.input ?? null],
      );

      branches.push({
        id,
        workflowId: input.workflowId,
        taskId: input.taskId,
        branchKey: spec.branchKey,
        role: spec.role,
        input: spec.input ?? null,
        output: null,
        status: 'pending',
      });
    }

    logger.info(
      { workflowId: input.workflowId, taskId: input.taskId, count: branches.length },
      'Created parallel branches',
    );
    return branches;
  }

  /** List all branches for a task. */
  async listBranches(workflowId: string, taskId: string): Promise<Branch[]> {
    this.ensureInit();
    const rows = this.storage.query<BranchRow>(
      `SELECT * FROM ${BRANCHES_TABLE} WHERE workflow_id = ? AND task_id = ? ORDER BY created_at`,
      [workflowId, taskId],
    );
    return rows.map(rowToBranch);
  }

  /** Mark a branch as completed with output. */
  async completeBranch(input: CompleteBranchInput): Promise<Branch> {
    this.ensureInit();
    this.storage.run(
      `UPDATE ${BRANCHES_TABLE} SET output = ?, status = 'completed' WHERE id = ?`,
      [input.output, input.branchId],
    );

    const row = this.storage.queryOne<BranchRow>(
      `SELECT * FROM ${BRANCHES_TABLE} WHERE id = ?`,
      [input.branchId],
    );

    if (!row) {
      throw new Error(`Branch ${input.branchId} not found after update`);
    }

    logger.info({ branchId: input.branchId }, 'Completed branch');
    return rowToBranch(row);
  }

  /** Mark a branch as failed. */
  async failBranch(branchId: string, error: string): Promise<Branch> {
    this.ensureInit();
    this.storage.run(
      `UPDATE ${BRANCHES_TABLE} SET output = ?, status = 'failed' WHERE id = ?`,
      [error, branchId],
    );

    const row = this.storage.queryOne<BranchRow>(
      `SELECT * FROM ${BRANCHES_TABLE} WHERE id = ?`,
      [branchId],
    );

    if (!row) {
      throw new Error(`Branch ${branchId} not found after update`);
    }

    logger.info({ branchId }, 'Marked branch as failed');
    return rowToBranch(row);
  }

  /** Check if all branches for a task are completed. */
  async allBranchesCompleted(workflowId: string, taskId: string): Promise<boolean> {
    const branches = await this.listBranches(workflowId, taskId);
    return branches.length > 0 && branches.every((b) => b.status === 'completed');
  }

  private ensureInit(): void {
    if (!this.initialized) {
      this.init();
    }
  }
}
