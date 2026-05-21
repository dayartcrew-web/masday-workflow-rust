/**
 * Workflow queries (msd-mcp business logic)
 *
 * Primary API for MCP tool handlers: getActiveWorkflow, listWorkflows,
 * getPlan, getCurrentTask, getResumeSuggestion.
 */

import { eq, desc, asc, sql } from "drizzle-orm";

export async function getActiveWorkflow(projectPath?: string) {
  const { db, workflows } = await import("@mcp-rebuild/db");

  const conditions = projectPath !== undefined
    ? eq(workflows.projectPath, projectPath)
    : sql`${workflows.projectPath} IS NULL`;

  const rows = await db.select().from(workflows)
    .where(sql`${conditions} AND ${workflows.status} NOT IN ('completed', 'cancelled')`)
    .orderBy(desc(workflows.updatedAt))
    .limit(1);
  return rows[0] ?? null;
}

export async function listWorkflows(status?: string) {
  const { db, workflows } = await import("@mcp-rebuild/db");

  if (status) {
    return db.select().from(workflows).where(eq(workflows.status, status)).orderBy(desc(workflows.updatedAt));
  }
  return db.select().from(workflows).orderBy(desc(workflows.updatedAt));
}

export async function getPlan(workflowId: string) {
  const { db, workflows, plans, tasks: tasksTable } = await import("@mcp-rebuild/db");

  const [workflow] = await db.select().from(workflows).where(eq(workflows.id, workflowId)).limit(1);
  if (!workflow) throw new Error("Workflow not found");

  if (!workflow.currentPlanId) {
    throw new Error("No active plan");
  }

  const [plan] = await db.select().from(plans).where(eq(plans.id, workflow.currentPlanId)).limit(1);
  if (!plan) throw new Error("Plan not found");

  const tasksList = await db.select().from(tasksTable)
    .where(eq(tasksTable.planId, plan.id))
    .orderBy(asc(tasksTable.createdAt));

  return { plan, tasks: tasksList };
}

export async function getCurrentTask(workflowId: string) {
  const { db, workflows, tasks: tasksTable } = await import("@mcp-rebuild/db");

  const [workflow] = await db.select().from(workflows).where(eq(workflows.id, workflowId)).limit(1);
  if (!workflow) throw new Error("Workflow not found");

  if (!workflow.currentTaskId) {
    throw new Error("No current task");
  }

  const [task] = await db.select().from(tasksTable).where(eq(tasksTable.id, workflow.currentTaskId)).limit(1);
  if (!task) throw new Error("Task not found");
  return task;
}

export async function getResumeSuggestion(workflowId: string) {
  const { db, workflows, plans, tasks: tasksTable, taskProgressLogs } = await import("@mcp-rebuild/db");

  const [workflow] = await db.select().from(workflows).where(eq(workflows.id, workflowId)).limit(1);
  if (!workflow) throw new Error("Workflow not found");

  if (!workflow.currentPlanId) {
    return {
      workflowId,
      status: workflow.status,
      suggestion: "No plan yet. Create a plan first.",
    };
  }

  if (!workflow.currentTaskId) {
    const [plan] = await db.select().from(plans).where(eq(plans.id, workflow.currentPlanId)).limit(1);
    const [nextTodo] = await db.select().from(tasksTable)
      .where(sql`${tasksTable.planId} = ${plan!.id} AND ${tasksTable.status} = 'todo'`)
      .orderBy(asc(tasksTable.createdAt))
      .limit(1);

    return {
      workflowId,
      status: workflow.status,
      currentPlanId: workflow.currentPlanId,
      suggestion: nextTodo
        ? `Start next task: "${nextTodo.title}" (id: ${nextTodo.id})`
        : "All tasks completed or in review.",
    };
  }

  const [task] = await db.select().from(tasksTable).where(eq(tasksTable.id, workflow.currentTaskId)).limit(1);
  if (!task) throw new Error("Task not found");

  const [latestProgress] = await db.select().from(taskProgressLogs)
    .where(eq(taskProgressLogs.taskId, task.id))
    .orderBy(desc(taskProgressLogs.createdAt))
    .limit(1);

  return {
    workflowId,
    status: workflow.status,
    currentPlanId: workflow.currentPlanId,
    currentTaskId: workflow.currentTaskId,
    currentTaskTitle: task.title,
    currentTaskStatus: task.status,
    lastProgressNote: latestProgress?.progressNote ?? null,
    suggestion:
      task.status === "in_progress"
        ? `Continue task: "${task.title}" — ${task.progressPercent}% complete`
        : task.status === "reviewing"
          ? `Review task: "${task.title}" — submit review decision`
          : task.status === "blocked"
            ? `Blocked task: "${task.title}" — resolve blocker`
            : `Task "${task.title}" is ${task.status}`,
  };
}
