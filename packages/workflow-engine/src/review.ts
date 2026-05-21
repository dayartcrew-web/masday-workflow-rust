/**
 * Review operations (msd-mcp business logic)
 */

import { eq, and, desc } from "drizzle-orm";
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
  const mod = await import("@mcp-rebuild/db");
  const { db, reviewDecisions, workflows } = mod;
  const tasksTable = mod.tasks;

  const [row] = await db.insert(reviewDecisions).values({
    workflowId: input.workflowId,
    taskId: input.taskId,
    reviewerAgent: input.reviewerAgent,
    decision: input.decision,
    notes: input.notes,
    gaps: input.gaps ?? [],
    testsVerified: input.testsVerified ?? false,
    testSummary: input.testSummary ?? {},
  }).returning();

  if (input.decision === "APPROVED") {
    const [task] = await db.select().from(tasksTable).where(eq(tasksTable.id, input.taskId)).limit(1);
    if (!task) throw new Error("Task not found");

    if (task.requiresTdd && !input.testsVerified) {
      await db.update(tasksTable).set({ status: "RUNNING" }).where(eq(tasksTable.id, input.taskId));
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
    await db.update(tasksTable).set({ status: "RUNNING" }).where(eq(tasksTable.id, input.taskId));
  }

  if (input.decision === "BLOCKED") {
    await db.update(tasksTable).set({ status: "BLOCKED" }).where(eq(tasksTable.id, input.taskId));
    await db.update(workflows).set({ status: "BLOCKED" }).where(eq(workflows.id, input.workflowId));
  }

  return row;
}

export async function getLatestReview(workflowId: string, taskId: string) {
  const { db, reviewDecisions } = await import("@mcp-rebuild/db");
  const [row] = await db.select().from(reviewDecisions)
    .where(and(eq(reviewDecisions.workflowId, workflowId), eq(reviewDecisions.taskId, taskId)))
    .orderBy(desc(reviewDecisions.createdAt))
    .limit(1);
  return row ?? null;
}
