/**
 * Review operations (msd-mcp business logic)
 */

import { prisma } from "@mcp-rebuild/db";
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
  const row = await prisma.reviewDecision.create({
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
    await prisma.task.update({
      where: { id: input.taskId },
      data: { status: "in_progress" },
    });
  }

  if (input.decision === "BLOCKED") {
    await prisma.task.update({
      where: { id: input.taskId },
      data: { status: "blocked" },
    });

    await prisma.workflow.update({
      where: { id: input.workflowId },
      data: { status: "blocked" },
    });
  }

  return row;
}

export async function getLatestReview(workflowId: string, taskId: string) {
  return prisma.reviewDecision.findFirst({
    where: { workflowId, taskId },
    orderBy: { createdAt: "desc" },
  });
}
