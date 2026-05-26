/**
 * Plan creation (msd-mcp business logic)
 */

import { eq, count } from "drizzle-orm";
import type { PlanContent } from "@mcp-rebuild/core";

export async function createPlan(input: {
  workflowId: string;
  summary: string;
  content: PlanContent;
  createdByAgent: string;
}) {
  const { db, plans, tasks: tasksTable, workflows } = await import("@mcp-rebuild/db");

  const countResult = await db
    .select({ count: count() })
    .from(plans)
    .where(eq(plans.workflowId, input.workflowId));
  const existingCount = countResult[0]?.count ?? 0;

  return db.transaction(async (tx) => {
    const [plan] = await tx.insert(plans).values({
      id: crypto.randomUUID(),
      workflowId: input.workflowId,
      version: existingCount + 1,
      status: "ACTIVE",
      summary: input.summary,
      content: input.content as unknown as Record<string, unknown>,
      createdByAgent: input.createdByAgent,
    }).returning();

    for (const task of input.content.tasks) {
      await tx.insert(tasksTable).values({
        id: crypto.randomUUID(),
        workflowId: input.workflowId,
        planId: plan.id,
        title: task.title,
        status: "PENDING",
        priority: task.priority,
        ownerAgent: task.ownerAgent,
        acceptanceCriteria: task.acceptanceCriteria ?? [],
        requiredContext: task.requiredContext ?? [],
        verificationSteps: task.verificationSteps ?? [],
      });
    }

    await tx.update(workflows).set({
      currentPlanId: plan.id,
      status: "READY",
    }).where(eq(workflows.id, input.workflowId));

    return plan;
  });
}
