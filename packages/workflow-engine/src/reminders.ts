/**
 * Workflow Reminder Engine
 *
 * Detects stale, stuck, and failed workflows/tasks and generates reminders.
 * Two detection modes:
 *   1. State-change: immediate notifications on FAILED workflows/tasks
 *   2. Time-based: workflows in EXECUTE with no recent progress, tasks stuck in RUNNING
 *
 * Persists reminders to PostgreSQL via Drizzle (WorkflowReminder table).
 */

import { eq, and, or, desc, count, inArray } from "drizzle-orm";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("ReminderEngine");

// ─── Types ───

export type ReminderSeverity = "critical" | "warning" | "info";

export interface WorkflowReminder {
  id: string;
  workflowId: string;
  taskId?: string;
  type: "STALE_EXECUTION" | "STUCK_TASK" | "FAILED_WORKFLOW" | "FAILED_TASK" | "IDLE_EXECUTION" | "NO_PROGRESS";
  severity: ReminderSeverity;
  message: string;
  acknowledged: boolean;
  createdAt: Date;
}

export interface ReminderConfig {
  /** Minutes before an EXECUTE workflow with no progress is considered stale (default: 30) */
  staleExecutionMinutes: number;
  /** Minutes before a RUNNING task is considered stuck (default: 15) */
  stuckTaskMinutes: number;
  /** Include FAILED workflows/tasks in reminders (default: true) */
  includeFailed: boolean;
}

export const DEFAULT_REMINDER_CONFIG: ReminderConfig = {
  staleExecutionMinutes: 30,
  stuckTaskMinutes: 15,
  includeFailed: true,
};

// ─── Drizzle interface (set at startup) ───

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let drizzleDb: any = null;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function setReminderDb(client: any): void {
  drizzleDb = client;
  logger.info("ReminderEngine: Drizzle db instance set");
}

// ─── Core: Check for reminders ───

export async function checkReminders(config: Partial<ReminderConfig> = {}): Promise<WorkflowReminder[]> {
  const cfg = { ...DEFAULT_REMINDER_CONFIG, ...config };
  const reminders: WorkflowReminder[] = [];

  if (!drizzleDb) {
    logger.warn("ReminderEngine: No Drizzle db instance, skipping check");
    return reminders;
  }

  const { workflows, tasks, taskProgressLogs, workflowReminders } = await import("@mcp-rebuild/db");
  const now = new Date();

  // 1. Active workflows in EXECUTE or FIX with no recent progress
  const activeWorkflows = await drizzleDb.select().from(workflows)
    .where(inArray(workflows.status, ["EXECUTE", "FIX"]));

  for (const wf of activeWorkflows) {
    const [lastProgress] = await drizzleDb.select().from(taskProgressLogs)
      .where(eq(taskProgressLogs.workflowId, wf.id))
      .orderBy(desc(taskProgressLogs.createdAt))
      .limit(1);

    const lastActivity = lastProgress?.createdAt ?? new Date(wf.updatedAt);
    const minutesSinceActivity = (now.getTime() - new Date(lastActivity).getTime()) / 60_000;

    if (minutesSinceActivity > cfg.staleExecutionMinutes) {
      reminders.push({
        id: `reminder_stale_${wf.id}_${Date.now()}`,
        workflowId: wf.id,
        type: "STALE_EXECUTION",
        severity: "warning",
        message: `Workflow "${wf.name}" has been in EXECUTE for ${Math.round(minutesSinceActivity)}m with no progress (threshold: ${cfg.staleExecutionMinutes}m)`,
        acknowledged: false,
        createdAt: now,
      });
    } else {
      // Check for tasks stuck in RUNNING
      const runningTasks = await drizzleDb.select().from(tasks)
        .where(and(eq(tasks.workflowId, wf.id), eq(tasks.status, "RUNNING")));

      for (const task of runningTasks) {
        const taskMinutes = (now.getTime() - new Date(task.updatedAt).getTime()) / 60_000;
        if (taskMinutes > cfg.stuckTaskMinutes) {
          reminders.push({
            id: `reminder_stuck_${task.id}_${Date.now()}`,
            workflowId: wf.id,
            taskId: task.id,
            type: "STUCK_TASK",
            severity: "warning",
            message: `Task "${task.title}" has been RUNNING for ${Math.round(taskMinutes)}m (threshold: ${cfg.stuckTaskMinutes}m)`,
            acknowledged: false,
            createdAt: now,
          });
        }
      }

      // Check for idle execution (all tasks PENDING but workflow in EXECUTE)
      const pendingTasks = await drizzleDb.select({ id: tasks.id }).from(tasks)
        .where(and(eq(tasks.workflowId, wf.id), eq(tasks.status, "PENDING")));

      if (runningTasks.length === 0 && pendingTasks.length > 0 && minutesSinceActivity > 5) {
        reminders.push({
          id: `reminder_idle_${wf.id}_${Date.now()}`,
          workflowId: wf.id,
          type: "IDLE_EXECUTION",
          severity: "info",
          message: `Workflow "${wf.name}" is in EXECUTE but no tasks are running (${pendingTasks.length} pending)`,
          acknowledged: false,
          createdAt: now,
        });
      }
    }
  }

  // 2. Failed workflows (recent: < 1 hour)
  if (cfg.includeFailed) {
    const failedWorkflows = await drizzleDb.select().from(workflows)
      .where(eq(workflows.status, "FAILED"));

    for (const wf of failedWorkflows) {
      const minutesSinceFailure = (now.getTime() - new Date(wf.updatedAt).getTime()) / 60_000;
      if (minutesSinceFailure < 60) {
        reminders.push({
          id: `reminder_failed_wf_${wf.id}_${Date.now()}`,
          workflowId: wf.id,
          type: "FAILED_WORKFLOW",
          severity: "critical",
          message: `Workflow "${wf.name}" has FAILED`,
          acknowledged: false,
          createdAt: now,
        });
      }
    }

    // 3. Failed tasks in active workflows
    const failedTasks = await drizzleDb.select().from(tasks)
      .where(eq(tasks.status, "FAILED"));

    for (const task of failedTasks) {
      const minutesSinceFailure = (now.getTime() - new Date(task.updatedAt).getTime()) / 60_000;
      if (minutesSinceFailure < 60) {
        reminders.push({
          id: `reminder_failed_task_${task.id}_${Date.now()}`,
          workflowId: task.workflowId,
          taskId: task.id,
          type: "FAILED_TASK",
          severity: "critical",
          message: `Task "${task.title}" has FAILED`,
          acknowledged: false,
          createdAt: now,
        });
      }
    }
  }

  // Persist new reminders to DB (skip if unacknowledged reminder already exists for same key)
  for (const reminder of reminders) {
    try {
      const conditions = [
        eq(workflowReminders.workflowId, reminder.workflowId),
        eq(workflowReminders.type, reminder.type),
        eq(workflowReminders.acknowledged, false),
      ];
      if (reminder.taskId) {
        conditions.push(eq(workflowReminders.taskId, reminder.taskId));
      }

      const [existing] = await drizzleDb.select().from(workflowReminders)
        .where(and(...conditions))
        .limit(1);
      if (existing) continue;

      await drizzleDb.insert(workflowReminders).values({
        workflowId: reminder.workflowId,
        taskId: reminder.taskId,
        type: reminder.type,
        severity: reminder.severity,
        message: reminder.message,
        acknowledged: false,
      }).returning();
    } catch (e) {
      logger.error({ error: String(e) }, "Failed to persist reminder");
    }
  }

  if (reminders.length > 0) {
    logger.info({ count: reminders.length }, "Reminders detected");
  }

  return reminders;
}

// ─── List stored reminders ───

export async function listReminders(opts: {
  workflowId?: string;
  acknowledged?: boolean;
  limit?: number;
}): Promise<unknown[]> {
  if (!drizzleDb) return [];

  const { workflowReminders } = await import("@mcp-rebuild/db");

  const conditions = [];
  if (opts.workflowId) conditions.push(eq(workflowReminders.workflowId, opts.workflowId));
  if (opts.acknowledged !== undefined) conditions.push(eq(workflowReminders.acknowledged, opts.acknowledged));

  const query = drizzleDb.select().from(workflowReminders)
    .orderBy(desc(workflowReminders.createdAt))
    .limit(opts.limit ?? 50);

  if (conditions.length > 0) {
    query.where(and(...conditions));
  }

  return query;
}

// ─── Acknowledge a reminder ───

export async function acknowledgeReminder(id: string): Promise<unknown> {
  if (!drizzleDb) throw new Error("No Drizzle db instance");

  const { workflowReminders } = await import("@mcp-rebuild/db");

  const [row] = await drizzleDb.update(workflowReminders)
    .set({ acknowledged: true })
    .where(eq(workflowReminders.id, id))
    .returning();

  return row;
}

// ─── Dismiss all reminders for a workflow ───

export async function dismissWorkflowReminders(workflowId: string): Promise<{ count: number }> {
  if (!drizzleDb) throw new Error("No Drizzle db instance");

  const { workflowReminders } = await import("@mcp-rebuild/db");

  const result = await drizzleDb.update(workflowReminders)
    .set({ acknowledged: true })
    .where(eq(workflowReminders.workflowId, workflowId))
    .returning();

  return { count: result.length };
}

// ─── Reminder stats ───

export async function reminderStats(): Promise<{ total: number; unacknowledged: number; bySeverity: Record<string, number> }> {
  if (!drizzleDb) return { total: 0, unacknowledged: 0, bySeverity: {} };

  const { workflowReminders } = await import("@mcp-rebuild/db");

  const [totalRow, unackRow, criticalRow, warningRow, infoRow] = await Promise.all([
    drizzleDb.select({ count: count() }).from(workflowReminders),
    drizzleDb.select({ count: count() }).from(workflowReminders).where(eq(workflowReminders.acknowledged, false)),
    drizzleDb.select({ count: count() }).from(workflowReminders).where(and(eq(workflowReminders.severity, "critical"), eq(workflowReminders.acknowledged, false))),
    drizzleDb.select({ count: count() }).from(workflowReminders).where(and(eq(workflowReminders.severity, "warning"), eq(workflowReminders.acknowledged, false))),
    drizzleDb.select({ count: count() }).from(workflowReminders).where(and(eq(workflowReminders.severity, "info"), eq(workflowReminders.acknowledged, false))),
  ]);

  const total = totalRow[0]?.count ?? 0;
  const unacknowledged = unackRow[0]?.count ?? 0;
  const critical = criticalRow[0]?.count ?? 0;
  const warning = warningRow[0]?.count ?? 0;
  const info = infoRow[0]?.count ?? 0;

  return { total, unacknowledged, bySeverity: { critical, warning, info } };
}
