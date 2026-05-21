/**
 * Policy validation (msd-mcp business logic)
 */

import { eq, and, desc } from "drizzle-orm";
import { detectScopeDrift } from "./drift-detector.js";

export async function validateExecution(input: {
  workflowId: string;
  taskId: string;
  sessionKey: string;
}) {
  const { db, workflows, sessionStates } = await import("@mcp-rebuild/db");

  const [workflow] = await db.select().from(workflows).where(eq(workflows.id, input.workflowId)).limit(1);
  if (!workflow) throw new Error(`Workflow ${input.workflowId} not found`);

  const [session] = await db.select().from(sessionStates).where(eq(sessionStates.sessionKey, input.sessionKey)).limit(1);

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
  const { db, reviewDecisions, tasks } = await import("@mcp-rebuild/db");

  const [review] = await db.select().from(reviewDecisions)
    .where(and(eq(reviewDecisions.workflowId, workflowId), eq(reviewDecisions.taskId, taskId)))
    .orderBy(desc(reviewDecisions.createdAt))
    .limit(1);

  if (!review || review.decision !== "APPROVED") {
    throw new Error("Completion blocked: review is not APPROVED");
  }

  const [task] = await db.select().from(tasks).where(eq(tasks.id, taskId)).limit(1);
  if (!task) throw new Error(`Task ${taskId} not found`);

  // TDD gate: if task requires TDD, enforce test evidence
  if (task.requiresTdd) {
    const testEvidence = task.testEvidence as Record<string, unknown> | null;
    const hasTests = Array.isArray(testEvidence?.testFiles) && (testEvidence?.testFiles as string[]).length > 0;
    const testsPassed = testEvidence?.testsPassed === true;

    if (!hasTests) {
      throw new Error(
        "Completion blocked: task requires TDD but no test files found. " +
        "Write tests first, then update task testEvidence via workflow.saveProgress.",
      );
    }

    if (!testsPassed) {
      throw new Error(
        "Completion blocked: task requires TDD but tests have not passed. " +
        "Run tests and ensure they pass before completing this task.",
      );
    }

    if (!review.testsVerified) {
      throw new Error(
        "Completion blocked: task requires TDD but review did not verify tests. " +
        "Reviewer must confirm tests exist and pass (testsVerified: true).",
      );
    }
  }

  if (outputText) {
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
  const { db, sessionStates, parallelBranches, reviewDecisions, tasks } = await import("@mcp-rebuild/db");

  const [session] = await db.select().from(sessionStates).where(eq(sessionStates.sessionKey, input.sessionKey)).limit(1);
  if (!session) throw new Error(`SessionState for key ${input.sessionKey} not found`);

  if (session.executionMode !== "parallel") {
    return { ok: true, skipped: true };
  }

  const branches = await db.select().from(parallelBranches).where(
    and(eq(parallelBranches.workflowId, input.workflowId), eq(parallelBranches.taskId, input.taskId)),
  );

  const incomplete = branches.filter(
    (b) => b.status !== "completed",
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

  const [review] = await db.select().from(reviewDecisions)
    .where(and(eq(reviewDecisions.workflowId, input.workflowId), eq(reviewDecisions.taskId, input.taskId)))
    .orderBy(desc(reviewDecisions.createdAt))
    .limit(1);

  if (!review || review.decision !== "APPROVED") {
    throw new Error(
      "Parallel completion blocked: review is not approved",
    );
  }

  if (input.outputText) {
    const [task] = await db.select().from(tasks).where(eq(tasks.id, input.taskId)).limit(1);
    if (!task) throw new Error(`Task ${input.taskId} not found`);

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
