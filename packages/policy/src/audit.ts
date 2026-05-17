/**
 * Workflow Audit
 *
 * Detects stuck tasks, missing reviews, stale sessions,
 * and blocked tasks without reason.
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import type { Workflow } from '@mcp-rebuild/core';
import { WorkflowStore } from '@mcp-rebuild/store';
import { ReviewManager } from './review-manager.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('WorkflowAuditor');

const STUCK_THRESHOLD_MS = 30 * 60 * 1000; // 30 minutes

export interface StuckTaskIssue {
  type: 'stuck_task';
  taskId: string;
  taskName: string;
  workflowId: string;
  durationMs: number;
  message: string;
}

export interface MissingReviewIssue {
  type: 'missing_review';
  taskId: string;
  taskName: string;
  workflowId: string;
  message: string;
}

export interface BlockedTaskIssue {
  type: 'blocked_without_reason';
  taskId: string;
  taskName: string;
  workflowId: string;
  message: string;
}

export interface StaleSessionIssue {
  type: 'stale_session';
  sessionKey: string;
  workflowId: string | undefined;
  message: string;
}

export type AuditIssue = StuckTaskIssue | MissingReviewIssue | BlockedTaskIssue | StaleSessionIssue;

export interface AuditResult {
  workflowId: string;
  workflowName: string;
  workflowState: string;
  issues: AuditIssue[];
  checkedAt: string;
}

export class WorkflowAuditor {
  private workflowStore: WorkflowStore;
  private reviewManager: ReviewManager;
  private storage: StorageBackend;

  constructor(storage: StorageBackend) {
    this.storage = storage;
    this.workflowStore = new WorkflowStore(storage);
    this.reviewManager = new ReviewManager(storage);
    this.reviewManager.init();
  }

  /**
   * Audit a single workflow for issues.
   */
  async audit(workflowId: string): Promise<AuditResult> {
    const workflow = this.workflowStore.load(workflowId);

    if (!workflow) {
      logger.warn({ workflowId }, 'Workflow not found for audit');
      return {
        workflowId,
        workflowName: 'unknown',
        workflowState: 'unknown',
        issues: [{
          type: 'stale_session',
          sessionKey: '',
          workflowId,
          message: `Workflow ${workflowId} not found in store`,
        }],
        checkedAt: new Date().toISOString(),
      };
    }

    const issues: AuditIssue[] = [];
    const now = Date.now();

    for (const task of workflow.tasks) {
      // Check for stuck tasks (in_progress > 30min)
      if (task.state === 'running' && task.startedAt) {
        const durationMs = now - task.startedAt.getTime();
        if (durationMs > STUCK_THRESHOLD_MS) {
          issues.push({
            type: 'stuck_task',
            taskId: task.id,
            taskName: task.name,
            workflowId,
            durationMs,
            message: `Task "${task.name}" has been running for ${Math.round(durationMs / 60000)} minutes`,
          });
        }
      }

      // Check for done tasks without reviews
      if (task.state === 'done') {
        const reviews = await this.reviewManager.getReviews(workflowId, task.id);
        if (reviews.length === 0) {
          issues.push({
            type: 'missing_review',
            taskId: task.id,
            taskName: task.name,
            workflowId,
            message: `Task "${task.name}" is done but has no review`,
          });
        }
      }

      // Check for blocked tasks without reason
      if (task.state === 'failed' && !task.error) {
        issues.push({
          type: 'blocked_without_reason',
          taskId: task.id,
          taskName: task.name,
          workflowId,
          message: `Task "${task.name}" is failed but has no error reason`,
        });
      }
    }

    // Check for stale sessions
    const staleSessions = this.findStaleSessions(workflowId, workflow);
    issues.push(...staleSessions);

    const result: AuditResult = {
      workflowId,
      workflowName: workflow.name,
      workflowState: workflow.state,
      issues,
      checkedAt: new Date().toISOString(),
    };

    logger.info(
      { workflowId, issueCount: issues.length },
      'Workflow audit completed',
    );

    return result;
  }

  /**
   * Find stale sessions associated with a workflow.
   */
  private findStaleSessions(workflowId: string, _workflow: Workflow): StaleSessionIssue[] {
    const issues: StaleSessionIssue[] = [];

    try {
      // Ensure the table exists before querying
      this.storage.run(`
        CREATE TABLE IF NOT EXISTS session_readiness (
          session_key       TEXT PRIMARY KEY,
          workflow_loaded   INTEGER NOT NULL DEFAULT 0,
          plan_loaded       INTEGER NOT NULL DEFAULT 0,
          task_loaded       INTEGER NOT NULL DEFAULT 0,
          context_loaded    INTEGER NOT NULL DEFAULT 0,
          review_approved   INTEGER NOT NULL DEFAULT 0,
          workflow_id       TEXT,
          plan_id           TEXT,
          task_id           TEXT,
          context_fingerprint TEXT,
          execution_mode    TEXT,
          synthesis_ready   INTEGER NOT NULL DEFAULT 0,
          verification_ready INTEGER NOT NULL DEFAULT 0
        )
      `);

      const rows = this.storage.query<{
        session_key: string;
        workflow_id: string | null;
        workflow_loaded: number;
        plan_loaded: number;
        task_loaded: number;
        context_loaded: number;
        review_approved: number;
      }>(
        `SELECT session_key, workflow_id, workflow_loaded, plan_loaded, task_loaded, context_loaded, review_approved
         FROM session_readiness
         WHERE workflow_id = ?`,
        [workflowId],
      );

      for (const row of rows) {
        const loadedCount =
          row.workflow_loaded + row.plan_loaded + row.task_loaded +
          row.context_loaded + row.review_approved;

        if (loadedCount > 0 && loadedCount < 5) {
          issues.push({
            type: 'stale_session',
            sessionKey: row.session_key,
            workflowId,
            message: `Session "${row.session_key}" is partially loaded (${loadedCount}/5 flags) for workflow ${workflowId}`,
          });
        }
      }
    } catch {
      logger.debug('No session_readiness table found, skipping stale session check');
    }

    return issues;
  }

  /**
   * Audit all active workflows (not DONE or FAILED).
   */
  async auditAll(): Promise<AuditResult[]> {
    const allWorkflows = this.workflowStore.loadAll();
    const activeWorkflows = allWorkflows.filter(
      (w: Workflow) => w.state !== 'DONE' && w.state !== 'FAILED',
    );

    const results: AuditResult[] = [];
    for (const wf of activeWorkflows) {
      const result = await this.audit(wf.id);
      results.push(result);
    }

    logger.info(
      { total: allWorkflows.length, active: activeWorkflows.length },
      'Audited all workflows',
    );

    return results;
  }
}
