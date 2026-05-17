/**
 * Review Manager (StorageBackend-backed)
 *
 * Manages review records for tasks, supporting APPROVED,
 * REWORK_REQUIRED, and BLOCKED decisions.
 *
 * Ported from masday-workflow-reborn/packages/orchestrator/src/review.ts
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import type { MsdReviewDecision } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';
import { randomUUID } from 'crypto';

const logger = createLogger('ReviewManager');

const REVIEWS_TABLE = 'review_records';

const CREATE_TABLE_SQL = `
CREATE TABLE IF NOT EXISTS ${REVIEWS_TABLE} (
  id              TEXT NOT NULL,
  workflow_id     TEXT NOT NULL,
  task_id         TEXT NOT NULL,
  reviewer_agent  TEXT NOT NULL,
  decision        TEXT NOT NULL,
  notes           TEXT,
  gaps            TEXT,
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  seq             INTEGER PRIMARY KEY AUTOINCREMENT
)`;

export interface ReviewRecord {
  id: string;
  workflowId: string;
  taskId: string;
  reviewerAgent: string;
  decision: MsdReviewDecision;
  notes: string | null;
  gaps: string[] | null;
  createdAt: string;
}

interface ReviewRow {
  id: string;
  workflow_id: string;
  task_id: string;
  reviewer_agent: string;
  decision: string;
  notes: string | null;
  gaps: string | null;
  created_at: string;
}

function rowToReview(row: ReviewRow): ReviewRecord {
  return {
    id: row.id,
    workflowId: row.workflow_id,
    taskId: row.task_id,
    reviewerAgent: row.reviewer_agent,
    decision: row.decision as MsdReviewDecision,
    notes: row.notes,
    gaps: row.gaps ? JSON.parse(row.gaps) : null,
    createdAt: row.created_at,
  };
}

export interface SubmitReviewInput {
  workflowId: string;
  taskId: string;
  reviewerAgent: string;
  decision: MsdReviewDecision;
  notes?: string;
  gaps?: string[];
}

export class ReviewManager {
  private storage: StorageBackend;
  private initialized = false;

  constructor(storage: StorageBackend) {
    this.storage = storage;
  }

  init(): void {
    if (this.initialized) return;
    this.storage.run(CREATE_TABLE_SQL);
    this.initialized = true;
    logger.info('ReviewManager initialized');
  }

  /** Submit a review for a task. */
  async submitReview(input: SubmitReviewInput): Promise<ReviewRecord> {
    this.ensureInit();
    const id = randomUUID();
    const now = new Date().toISOString();
    const gapsJson = input.gaps ? JSON.stringify(input.gaps) : null;

    this.storage.run(
      `INSERT INTO ${REVIEWS_TABLE} (id, workflow_id, task_id, reviewer_agent, decision, notes, gaps, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        id,
        input.workflowId,
        input.taskId,
        input.reviewerAgent,
        input.decision,
        input.notes ?? null,
        gapsJson,
        now,
      ],
    );

    logger.info(
      {
        reviewId: id,
        workflowId: input.workflowId,
        taskId: input.taskId,
        decision: input.decision,
      },
      'Review submitted',
    );

    return {
      id,
      workflowId: input.workflowId,
      taskId: input.taskId,
      reviewerAgent: input.reviewerAgent,
      decision: input.decision,
      notes: input.notes ?? null,
      gaps: input.gaps ?? null,
      createdAt: now,
    };
  }

  /** Get the most recent review for a task. */
  async getLatestReview(
    workflowId: string,
    taskId: string,
  ): Promise<ReviewRecord | null> {
    this.ensureInit();
    const row = this.storage.queryOne<ReviewRow>(
      `SELECT * FROM ${REVIEWS_TABLE} WHERE workflow_id = ? AND task_id = ? ORDER BY seq DESC LIMIT 1`,
      [workflowId, taskId],
    );

    return row ? rowToReview(row) : null;
  }

  /** Get all reviews for a task, ordered by time. */
  async getReviews(
    workflowId: string,
    taskId: string,
  ): Promise<ReviewRecord[]> {
    this.ensureInit();
    const rows = this.storage.query<ReviewRow>(
      `SELECT * FROM ${REVIEWS_TABLE} WHERE workflow_id = ? AND task_id = ? ORDER BY seq`,
      [workflowId, taskId],
    );
    return rows.map(rowToReview);
  }

  private ensureInit(): void {
    if (!this.initialized) {
      this.init();
    }
  }
}
