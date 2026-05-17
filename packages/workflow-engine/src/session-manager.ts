/**
 * Session Manager (reborn StorageBackend variant)
 *
 * Manages session readiness state via StorageBackend (SQLite/JSON).
 * This is the reborn equivalent of the msd-mcp Prisma-based session.ts.
 *
 * Ported from masday-workflow-reborn/packages/orchestrator/src/session.ts
 */

import type { StorageBackend } from "@mcp-rebuild/store";
import type { SessionReadiness } from "@mcp-rebuild/core";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("SessionManager");

const SESSIONS_TABLE = "session_readiness";

const CREATE_TABLE_SQL = `
CREATE TABLE IF NOT EXISTS ${SESSIONS_TABLE} (
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
)`;

interface SessionRow {
  session_key: string;
  workflow_loaded: number;
  plan_loaded: number;
  task_loaded: number;
  context_loaded: number;
  review_approved: number;
  workflow_id: string | null;
  plan_id: string | null;
  task_id: string | null;
  context_fingerprint: string | null;
  execution_mode: string | null;
  synthesis_ready: number;
  verification_ready: number;
}

function rowToReadiness(row: SessionRow): SessionReadiness {
  return {
    sessionKey: row.session_key,
    workflowLoaded: row.workflow_loaded === 1,
    planLoaded: row.plan_loaded === 1,
    taskLoaded: row.task_loaded === 1,
    contextLoaded: row.context_loaded === 1,
    reviewApproved: row.review_approved === 1,
    workflowId: row.workflow_id ?? undefined,
    planId: row.plan_id ?? undefined,
    taskId: row.task_id ?? undefined,
    contextFingerprint: row.context_fingerprint ?? undefined,
    executionMode:
      (row.execution_mode as "sequential" | "parallel") ?? undefined,
    synthesisReady: row.synthesis_ready === 1,
    verificationReady: row.verification_ready === 1,
  };
}

export class SessionManager {
  private storage: StorageBackend;
  private initialized = false;

  constructor(storage: StorageBackend) {
    this.storage = storage;
  }

  init(): void {
    if (this.initialized) return;
    this.storage.run(CREATE_TABLE_SQL);
    this.initialized = true;
    logger.info("SessionManager initialized");
  }

  async getOrCreateState(sessionKey: string): Promise<SessionReadiness> {
    this.ensureInit();
    const row = this.storage.queryOne<SessionRow>(
      `SELECT * FROM ${SESSIONS_TABLE} WHERE session_key = ?`,
      [sessionKey],
    );

    if (row) {
      return rowToReadiness(row);
    }

    const fresh: SessionReadiness = {
      sessionKey,
      workflowLoaded: false,
      planLoaded: false,
      taskLoaded: false,
      contextLoaded: false,
      reviewApproved: false,
    };

    this.storage.run(
      `INSERT INTO ${SESSIONS_TABLE} (session_key, workflow_loaded, plan_loaded, task_loaded, context_loaded, review_approved, workflow_id, plan_id, task_id, context_fingerprint, execution_mode, synthesis_ready, verification_ready) VALUES (?, 0, 0, 0, 0, 0, NULL, NULL, NULL, NULL, NULL, 0, 0)`,
      [sessionKey],
    );

    logger.info({ sessionKey }, "Created new session readiness record");
    return fresh;
  }

  async patchState(
    sessionKey: string,
    patch: Partial<SessionReadiness>,
  ): Promise<SessionReadiness> {
    this.ensureInit();

    await this.getOrCreateState(sessionKey);

    const sets: string[] = [];
    const values: unknown[] = [];

    const fieldMap: Record<string, (v: unknown) => unknown> = {
      workflowLoaded: (v) => (v ? 1 : 0),
      planLoaded: (v) => (v ? 1 : 0),
      taskLoaded: (v) => (v ? 1 : 0),
      contextLoaded: (v) => (v ? 1 : 0),
      reviewApproved: (v) => (v ? 1 : 0),
      workflowId: (v) => v,
      planId: (v) => v,
      taskId: (v) => v,
      contextFingerprint: (v) => v,
      executionMode: (v) => v,
      synthesisReady: (v) => (v ? 1 : 0),
      verificationReady: (v) => (v ? 1 : 0),
    };

    const colMap: Record<string, string> = {
      workflowLoaded: "workflow_loaded",
      planLoaded: "plan_loaded",
      taskLoaded: "task_loaded",
      contextLoaded: "context_loaded",
      reviewApproved: "review_approved",
      workflowId: "workflow_id",
      planId: "plan_id",
      taskId: "task_id",
      contextFingerprint: "context_fingerprint",
      executionMode: "execution_mode",
      synthesisReady: "synthesis_ready",
      verificationReady: "verification_ready",
    };

    for (const [key, transform] of Object.entries(fieldMap)) {
      if (key in patch) {
        const col = colMap[key];
        sets.push(`${col} = ?`);
        values.push(
          transform((patch as Record<string, unknown>)[key]),
        );
      }
    }

    if (sets.length === 0) {
      return this.getOrCreateState(sessionKey);
    }

    values.push(sessionKey);
    this.storage.run(
      `UPDATE ${SESSIONS_TABLE} SET ${sets.join(", ")} WHERE session_key = ?`,
      values,
    );

    logger.info(
      { sessionKey, patchKeys: Object.keys(patch) },
      "Patched session readiness",
    );

    return this.getOrCreateState(sessionKey);
  }

  async checkReadiness(
    sessionKey: string,
  ): Promise<{ ready: boolean; missing: string[] }> {
    const state = await this.getOrCreateState(sessionKey);
    const missing: string[] = [];

    if (!state.workflowLoaded) missing.push("workflowLoaded");
    if (!state.planLoaded) missing.push("planLoaded");
    if (!state.taskLoaded) missing.push("taskLoaded");
    if (!state.contextLoaded) missing.push("contextLoaded");
    if (!state.reviewApproved) missing.push("reviewApproved");

    return { ready: missing.length === 0, missing };
  }

  private ensureInit(): void {
    if (!this.initialized) {
      this.init();
    }
  }
}
