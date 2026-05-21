/**
 * Task operations (msd-mcp business logic)
 */

import { eq, asc } from "drizzle-orm";

export async function startTask(input: {
  workflowId: string;
  taskId: string;
}) {
  const { db, tasks: tasksTable, workflows } = await import("@mcp-rebuild/db");

  return db.transaction(async (tx) => {
    const [task] = await tx.select().from(tasksTable).where(eq(tasksTable.id, input.taskId)).limit(1);
    if (!task) throw new Error(`Task not found: ${input.taskId}`);

    await tx.update(tasksTable).set({
      status: "RUNNING",
      progressPercent: 1,
    }).where(eq(tasksTable.id, task.id));

    await tx.update(workflows).set({
      currentTaskId: task.id,
      status: "EXECUTE",
    }).where(eq(workflows.id, input.workflowId));

    const [updated] = await tx.select().from(tasksTable).where(eq(tasksTable.id, task.id)).limit(1);
    if (!updated) throw new Error(`Task not found after update: ${task.id}`);
    return updated;
  });
}

export async function listTasks(workflowId: string) {
  const { db, tasks: tasksTable } = await import("@mcp-rebuild/db");

  return db.select().from(tasksTable)
    .where(eq(tasksTable.workflowId, workflowId))
    .orderBy(asc(tasksTable.createdAt));
}

export async function completeTask(input: {
  workflowId: string;
  taskId: string;
}) {
  const { db, tasks: tasksTable, plans, workflows } = await import("@mcp-rebuild/db");

  return db.transaction(async (tx) => {
    const [task] = await tx.select().from(tasksTable).where(eq(tasksTable.id, input.taskId)).limit(1);
    if (!task) throw new Error(`Task not found: ${input.taskId}`);

    await tx.update(tasksTable).set({
      status: "DONE",
      progressPercent: 100,
    }).where(eq(tasksTable.id, task.id));

    if (task.planId) {
      const allTasks = await tx.select().from(tasksTable)
        .where(eq(tasksTable.planId, task.planId));
      const allDone =
        allTasks.length > 0 && allTasks.every((t) => t.status === "DONE");

      if (allDone) {
        await tx.update(plans).set({
          status: "DONE",
        }).where(eq(plans.id, task.planId));

        await tx.update(workflows).set({
          status: "DONE",
          currentTaskId: null,
        }).where(eq(workflows.id, input.workflowId));
      } else {
        await tx.update(workflows).set({
          currentTaskId: null,
        }).where(eq(workflows.id, input.workflowId));
      }
    } else {
      await tx.update(workflows).set({
        currentTaskId: null,
      }).where(eq(workflows.id, input.workflowId));
    }

    const [updated] = await tx.select().from(tasksTable).where(eq(tasksTable.id, task.id)).limit(1);
    if (!updated) throw new Error(`Task not found after update: ${task.id}`);
    return updated;
  });
}
