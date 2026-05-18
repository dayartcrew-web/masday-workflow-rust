/**
 * Policy validation (msd-mcp business logic)
 */


import { detectScopeDrift } from "./drift-detector.js";

export async function validateExecution(input: {
  workflowId: string;
  taskId: string;
  sessionKey: string;
}) {
  const workflow = await (await import("@mcp-rebuild/db")).prisma.workflow.findUniqueOrThrow({
    where: { id: input.workflowId },
  });

  const session = await (await import("@mcp-rebuild/db")).prisma.sessionState.findUnique({
    where: { sessionKey: input.sessionKey },
  });

  if (workflow.currentTaskId !== input.taskId) {
    throw new Error("Execution blocked: task is not current active task");
  }

  if (
    !session?.workflowLoaded ||
    !session?.planLoaded ||
    !session?.taskLoaded ||
    !session?.contextLoaded
  ) {
    throw new Error("Execution blocked: session state is incomplete");
  }

  return { ok: true };
}

export async function validateCompletion(
  workflowId: string,
  taskId: string,
  outputText?: string,
) {
  const review = await (await import("@mcp-rebuild/db")).prisma.reviewDecision.findFirst({
    where: { workflowId, taskId },
    orderBy: { createdAt: "desc" },
  });

  if (!review || review.decision !== "APPROVED") {
    throw new Error("Completion blocked: review is not APPROVED");
  }

  if (outputText) {
    const task = await (await import("@mcp-rebuild/db")).prisma.task.findUniqueOrThrow({
      where: { id: taskId },
    });

    const result = detectScopeDrift({
      taskTitle: task.title,
      acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
      requiredContext: (task.requiredContext as string[]) ?? [],
      outputText,
    });

    if (result.drift) {
      throw new Error("Completion blocked: scope drift detected");
    }
  }

  return { ok: true };
}

export async function validateParallelCompletion(input: {
  workflowId: string;
  taskId: string;
  sessionKey: string;
  outputText?: string;
}) {
  const session = await (await import("@mcp-rebuild/db")).prisma.sessionState.findUniqueOrThrow({
    where: { sessionKey: input.sessionKey },
  });

  if (session.executionMode !== "parallel") {
    return { ok: true, skipped: true };
  }

  const branches = await (await import("@mcp-rebuild/db")).prisma.parallelBranch.findMany({
    where: {
      workflowId: input.workflowId,
      taskId: input.taskId,
    },
  });

  const incomplete = branches.filter(
    (b: { status: string }) => b.status !== "completed",
  );
  if (incomplete.length > 0) {
    throw new Error(
      "Parallel completion blocked: some branches are not completed",
    );
  }

  if (!session.synthesisReady) {
    throw new Error("Parallel completion blocked: synthesis is not ready");
  }

  if (!session.verificationReady) {
    throw new Error(
      "Parallel completion blocked: verification is not ready",
    );
  }

  const review = await (await import("@mcp-rebuild/db")).prisma.reviewDecision.findFirst({
    where: {
      workflowId: input.workflowId,
      taskId: input.taskId,
    },
    orderBy: { createdAt: "desc" },
  });

  if (!review || review.decision !== "APPROVED") {
    throw new Error(
      "Parallel completion blocked: review is not approved",
    );
  }

  if (input.outputText) {
    const task = await (await import("@mcp-rebuild/db")).prisma.task.findUniqueOrThrow({
      where: { id: input.taskId },
    });

    const drift = detectScopeDrift({
      taskTitle: task.title,
      acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
      requiredContext: (task.requiredContext as string[]) ?? [],
      outputText: input.outputText,
    });

    if (drift.drift) {
      throw new Error(
        "Parallel completion blocked: scope drift detected",
      );
    }
  }

  return { ok: true };
}
