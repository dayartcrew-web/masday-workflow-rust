/**
 * Parallel execution branches (msd-mcp business logic)
 */

import { eq, and, asc } from "drizzle-orm";

type JsonValue = string | number | boolean | JsonValue[] | { [key: string]: JsonValue };

export async function setExecutionMode(sessionKey: string, mode: "sequential" | "parallel") {
  const mod = await import("@mcp-rebuild/db");
  const [row] = await mod.db.update(mod.sessionStates).set({ executionMode: mode }).where(eq(mod.sessionStates.sessionKey, sessionKey)).returning();
  return row;
}

export async function createParallelBranches(input: {
  workflowId: string;
  taskId: string;
  branches: Array<{ branchKey: string; role: string; input: Record<string, unknown> }>;
}) {
  const mod = await import("@mcp-rebuild/db");
  const { db } = mod;
  const branches = mod.parallelBranches;

  for (const branch of input.branches) {
    await db.insert(branches).values({
      workflowId: input.workflowId,
      taskId: input.taskId,
      branchKey: branch.branchKey,
      role: branch.role,
      status: "pending",
      input: branch.input as JsonValue,
    });
  }

  return db.select().from(branches)
    .where(and(eq(branches.workflowId, input.workflowId), eq(branches.taskId, input.taskId)))
    .orderBy(asc(branches.createdAt));
}

export async function listParallelBranches(workflowId: string, taskId: string) {
  const mod = await import("@mcp-rebuild/db");
  return mod.db.select().from(mod.parallelBranches)
    .where(and(eq(mod.parallelBranches.workflowId, workflowId), eq(mod.parallelBranches.taskId, taskId)))
    .orderBy(asc(mod.parallelBranches.createdAt));
}

export async function completeParallelBranch(input: { branchId: string; output: Record<string, unknown> }) {
  const mod = await import("@mcp-rebuild/db");
  const [row] = await mod.db.update(mod.parallelBranches)
    .set({ status: "completed", output: input.output as JsonValue })
    .where(eq(mod.parallelBranches.id, input.branchId))
    .returning();
  return row;
}

export async function markSynthesisReady(sessionKey: string, ready: boolean) {
  const mod = await import("@mcp-rebuild/db");
  const [row] = await mod.db.update(mod.sessionStates).set({ synthesisReady: ready }).where(eq(mod.sessionStates.sessionKey, sessionKey)).returning();
  return row;
}

export async function markVerificationReady(sessionKey: string, ready: boolean) {
  const mod = await import("@mcp-rebuild/db");
  const [row] = await mod.db.update(mod.sessionStates).set({ verificationReady: ready }).where(eq(mod.sessionStates.sessionKey, sessionKey)).returning();
  return row;
}
