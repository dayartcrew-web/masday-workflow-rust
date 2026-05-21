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
  testsVerified?: boolean;
  testSummary?: { testFiles: string[]; testsPassed: boolean; coveragePercent?: number };
}) {
  const prisma = (await import("@mcp-rebuild/db")).prisma;

  const row = await prisma.reviewDecision.create({
    data: {
      workflowId: input.workflowId,
      taskId: input.taskId,
      reviewerAgent: input.reviewerAgent,
      decision: input.decision,
      notes: input.notes,
      gaps: input.gaps ?? [],
      testsVerified: input.testsVerified ?? false,
      testSummary: input.testSummary ?? {},
    },
  });

  if (input.decision === "APPROVED") {
    // TDD gate: check task requiresTdd before completing
    const task = await prisma.task.findUniqueOrThrow({
      where: { id: input.taskId },
    });

    if (task.requiresTdd && !input.testsVerified) {
      // Don't auto-complete: reviewer must verify tests
      await prisma.task.update({
        where: { id: input.taskId },
        data: { status: "RUNNING" },
      });
      return {
        ...row,
        _tddWarning: "Task requires TDD but testsVerified was not set to true. Task remains RUNNING.",
      };
    }

    await completeTask({
      taskId: input.taskId,
      workflowId: input.workflowId,
    });
  }

  if (input.decision === "REWORK_REQUIRED") {
    await prisma.task.update({
      where: { id: input.taskId },
      data: { status: "RUNNING" },
    });
  }

  if (input.decision === "BLOCKED") {
    await prisma.task.update({
      where: { id: input.taskId },
      data: { status: "BLOCKED" },
    });

    await prisma.workflow.update({
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
