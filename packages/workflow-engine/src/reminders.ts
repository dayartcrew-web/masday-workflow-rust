/**
 * Workflow Reminder Engine
 *
 * Detects stale, stuck, and failed workflows/tasks and generates reminders.
 * Two detection modes:
 *   1. State-change: immediate notifications on FAILED workflows/tasks
 *   2. Time-based: workflows in EXECUTE with no recent progress, tasks stuck in RUNNING
 *
 * Persists reminders to PostgreSQL via Prisma (WorkflowReminder table).
 */

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

// ─── Prisma interface (set at startup) ───

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let prisma: any = null;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function setReminderPrisma(client: any): void {
  prisma = client;
  logger.info("ReminderEngine: Prisma client set");
}

// ─── Core: Check for reminders ───

export async function checkReminders(config: Partial<ReminderConfig> = {}): Promise<WorkflowReminder[]> {
  const cfg = { ...DEFAULT_REMINDER_CONFIG, ...config };
  const reminders: WorkflowReminder[] = [];

  if (!prisma) {
    logger.warn("ReminderEngine: No Prisma client, skipping check");
    return reminders;
  }

  const now = new Date();

  // 1. Active workflows in EXECUTE or FIX with no recent progress
  const activeWorkflows = await prisma.workflow.findMany({
    where: { status: { in: ["EXECUTE", "FIX"] } },
  }) as Array<{ id: string; name: string; status: string; updatedAt: Date; currentTaskId?: string }>;

  for (const wf of activeWorkflows) {
    const lastProgress = await prisma.taskProgressLog.findFirst({
      where: { workflowId: wf.id },
      orderBy: { createdAt: "desc" },
    }) as { createdAt: Date } | null;

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
      const runningTasks = await prisma.task.findMany({
        where: { workflowId: wf.id, status: "RUNNING" },
      }) as Array<{ id: string; title: string; status: string; updatedAt: Date }>;

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
      const pendingTasks = await prisma.task.findMany({
        where: { workflowId: wf.id, status: "PENDING" },
      }) as Array<{ id: string }>;

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
    const failedWorkflows = await prisma.workflow.findMany({
      where: { status: "FAILED" },
    }) as Array<{ id: string; name: string; status: string; updatedAt: Date }>;

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
    const failedTasks = await prisma.task.findMany({
      where: { status: "FAILED" },
    }) as Array<{ id: string; title: string; workflowId: string; status: string; updatedAt: Date }>;

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
      const existing = await prisma.workflowReminder.findFirst({
        where: {
          workflowId: reminder.workflowId,
          taskId: reminder.taskId ?? undefined,
          type: reminder.type,
          acknowledged: false,
        },
      });
      if (existing) continue;
      await prisma.workflowReminder.create({
        data: {
          workflowId: reminder.workflowId,
          taskId: reminder.taskId,
          type: reminder.type,
          severity: reminder.severity,
          message: reminder.message,
          acknowledged: false,
        },
      });
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
  if (!prisma) return [];

  const where: Record<string, unknown> = {};
  if (opts.workflowId) where.workflowId = opts.workflowId;
  if (opts.acknowledged !== undefined) where.acknowledged = opts.acknowledged;

  return prisma.workflowReminder.findMany({
    where,
    orderBy: { createdAt: "desc" },
    take: opts.limit ?? 50,
  });
}

// ─── Acknowledge a reminder ───

export async function acknowledgeReminder(id: string): Promise<unknown> {
  if (!prisma) throw new Error("No Prisma client");

  return prisma.workflowReminder.update({
    where: { id },
    data: { acknowledged: true },
  });
}

// ─── Dismiss all reminders for a workflow ───

export async function dismissWorkflowReminders(workflowId: string): Promise<{ count: number }> {
  if (!prisma) throw new Error("No Prisma client");

  const result = await prisma.workflowReminder.updateMany({
    where: { workflowId },
    data: { acknowledged: true },
  });
  return { count: result.count };
}

// ─── Reminder stats ───

export async function reminderStats(): Promise<{ total: number; unacknowledged: number; bySeverity: Record<string, number> }> {
  if (!prisma) return { total: 0, unacknowledged: 0, bySeverity: {} };

  const [total, unacknowledged, critical, warning, info] = await Promise.all([
    prisma.workflowReminder.count({ where: {} }),
    prisma.workflowReminder.count({ where: { acknowledged: false } }),
    prisma.workflowReminder.count({ where: { severity: "critical", acknowledged: false } }),
    prisma.workflowReminder.count({ where: { severity: "warning", acknowledged: false } }),
    prisma.workflowReminder.count({ where: { severity: "info", acknowledged: false } }),
  ]);

  return { total, unacknowledged, bySeverity: { critical, warning, info } };
}
