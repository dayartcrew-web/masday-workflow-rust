/**
 * Plan creation (msd-mcp business logic)
 */


import type { PlanContent } from "@mcp-rebuild/core";
import type { Prisma } from "@prisma/client";

export async function createPlan(input: {
  workflowId: string;
  summary: string;
  content: PlanContent;
  createdByAgent: string;
}) {
  const existingCount = await (await import("@mcp-rebuild/db")).prisma.plan.count({
    where: { workflowId: input.workflowId },
  });

  return (await import("@mcp-rebuild/db")).prisma.$transaction(async (tx: Prisma.TransactionClient) => {
    const plan = await tx.plan.create({
      data: {
        workflowId: input.workflowId,
        version: existingCount + 1,
        status: "ACTIVE",
        summary: input.summary,
        content: input.content as never,
        createdByAgent: input.createdByAgent,
      },
    });

    for (const task of input.content.tasks) {
      await tx.task.create({
        data: {
          workflowId: input.workflowId,
          planId: plan.id,
          title: task.title,
          status: "PENDING",
          priority: task.priority,
          ownerAgent: task.ownerAgent,
          acceptanceCriteria: task.acceptanceCriteria ?? [],
          requiredContext: task.requiredContext ?? [],
          verificationSteps: task.verificationSteps ?? [],
        },
      });
    }

    await tx.workflow.update({
      where: { id: input.workflowId },
      data: {
        currentPlanId: plan.id,
        status: "READY",
      },
    });

    return plan;
  });
}
