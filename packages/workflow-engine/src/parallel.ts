/**
 * Parallel execution branches (msd-mcp business logic)
 */



/** JSON-compatible value that Prisma's Json field accepts */
type JsonValue =
  | string
  | number
  | boolean
  | JsonValue[]
  | { [key: string]: JsonValue };

export async function setExecutionMode(
  sessionKey: string,
  mode: "sequential" | "parallel",
) {
  return (await import("@mcp-rebuild/db")).prisma.sessionState.update({
    where: { sessionKey },
    data: { executionMode: mode },
  });
}

export async function createParallelBranches(input: {
  workflowId: string;
  taskId: string;
  branches: Array<{
    branchKey: string;
    role: string;
    input: Record<string, unknown>;
  }>;
}) {
  for (const branch of input.branches) {
    await (await import("@mcp-rebuild/db")).prisma.parallelBranch.create({
      data: {
        workflowId: input.workflowId,
        taskId: input.taskId,
        branchKey: branch.branchKey,
        role: branch.role,
        status: "pending",
        input: branch.input as JsonValue,
      },
    });
  }

  return (await import("@mcp-rebuild/db")).prisma.parallelBranch.findMany({
    where: {
      workflowId: input.workflowId,
      taskId: input.taskId,
    },
    orderBy: { createdAt: "asc" },
  });
}

export async function listParallelBranches(
  workflowId: string,
  taskId: string,
) {
  return (await import("@mcp-rebuild/db")).prisma.parallelBranch.findMany({
    where: { workflowId, taskId },
    orderBy: { createdAt: "asc" },
  });
}

export async function completeParallelBranch(input: {
  branchId: string;
  output: Record<string, unknown>;
}) {
  return (await import("@mcp-rebuild/db")).prisma.parallelBranch.update({
    where: { id: input.branchId },
    data: {
      status: "completed",
      output: input.output as JsonValue,
    },
  });
}

export async function markSynthesisReady(
  sessionKey: string,
  ready: boolean,
) {
  return (await import("@mcp-rebuild/db")).prisma.sessionState.update({
    where: { sessionKey },
    data: { synthesisReady: ready },
  });
}

export async function markVerificationReady(
  sessionKey: string,
  ready: boolean,
) {
  return (await import("@mcp-rebuild/db")).prisma.sessionState.update({
    where: { sessionKey },
    data: { verificationReady: ready },
  });
}
