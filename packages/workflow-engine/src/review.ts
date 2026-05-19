/**
 * Review operations (msd-mcp business logic)
 */


import type { MsdReviewDecision } from "@mcp-rebuild/core";
import { completeTask } from "./task.js";

export async function submitReview(input: {
  workflowId: string;
  taskId: string;
  reviewerAgent: string;
  decision: MsdReviewDecision;
  notes: string;
  gaps?: string[];
}) {
  const row = await (await import("@mcp-rebuild/db")).prisma.reviewDecision.create({
    data: {
      workflowId: input.workflowId,
      taskId: input.taskId,
      reviewerAgent: input.reviewerAgent,
      decision: input.decision,
      notes: input.notes,
      gaps: input.gaps ?? [],
    },
  });

  if (input.decision === "APPROVED") {
    await completeTask({
      taskId: input.taskId,
      workflowId: input.workflowId,
    });
  }

  if (input.decision === "REWORK_REQUIRED") {
    await (await import("@mcp-rebuild/db")).prisma.task.update({
      where: { id: input.taskId },
      data: { status: "RUNNING" },
    });
  }

  if (input.decision === "BLOCKED") {
    await (await import("@mcp-rebuild/db")).prisma.task.update({
      where: { id: input.taskId },
      data: { status: "BLOCKED" },
    });

    await (await import("@mcp-rebuild/db")).prisma.workflow.update({
      where: { id: input.workflowId },
      data: { status: "BLOCKED" },
    });
  }

  return row;
}

export async function getLatestReview(workflowId: string, taskId: string) {
  return (await import("@mcp-rebuild/db")).prisma.reviewDecision.findFirst({
    where: { workflowId, taskId },
    orderBy: { createdAt: "desc" },
  });
}
