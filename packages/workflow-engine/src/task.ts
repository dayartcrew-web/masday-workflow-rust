/**
 * Task operations (msd-mcp business logic)
 */


import type { Prisma } from "@prisma/client";

export async function startTask(input: {
  workflowId: string;
  taskId: string;
}) {
  return (await import("@mcp-rebuild/db")).prisma.$transaction(async (tx: Prisma.TransactionClient) => {
    const task = await tx.task.findUniqueOrThrow({
      where: { id: input.taskId },
    });

    await tx.task.update({
      where: { id: task.id },
      data: {
        status: "in_progress",
        progressPercent: 1,
      },
    });

    await tx.workflow.update({
      where: { id: input.workflowId },
      data: {
        currentTaskId: task.id,
        status: "executing",
      },
    });

    return tx.task.findUniqueOrThrow({
      where: { id: task.id },
    });
  });
}

export async function listTasks(workflowId: string) {
  return (await import("@mcp-rebuild/db")).prisma.task.findMany({
    where: { workflowId },
    orderBy: { createdAt: "asc" },
  });
}

export async function completeTask(input: {
  workflowId: string;
  taskId: string;
}) {
  return (await import("@mcp-rebuild/db")).prisma.$transaction(async (tx: Prisma.TransactionClient) => {
    const task = await tx.task.findUniqueOrThrow({
      where: { id: input.taskId },
    });

    await tx.task.update({
      where: { id: task.id },
      data: {
        status: "done",
        progressPercent: 100,
      },
    });

    if (task.planId) {
      const allTasks = await tx.task.findMany({
        where: { planId: task.planId },
      });
      const allDone =
        allTasks.length > 0 && allTasks.every((t: { status: string }) => t.status === "done");

      if (allDone) {
        await tx.plan.update({
          where: { id: task.planId },
          data: { status: "completed" },
        });
        await tx.workflow.update({
          where: { id: input.workflowId },
          data: { status: "completed", currentTaskId: null },
        });
      } else {
        await tx.workflow.update({
          where: { id: input.workflowId },
          data: { currentTaskId: null },
        });
      }
    } else {
      await tx.workflow.update({
        where: { id: input.workflowId },
        data: { currentTaskId: null },
      });
    }

    return tx.task.findUniqueOrThrow({
      where: { id: task.id },
    });
  });
}
