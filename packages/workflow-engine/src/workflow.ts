/**
 * Workflow queries (msd-mcp business logic)
 *
 * Primary API for MCP tool handlers: getActiveWorkflow, listWorkflows,
 * getPlan, getCurrentTask, getResumeSuggestion.
 */



export async function getActiveWorkflow(projectPath?: string) {
  return (await import("@mcp-rebuild/db")).prisma.workflow.findFirst({
    where: {
      projectPath: projectPath ?? null,
      status: { notIn: ["completed", "cancelled"] },
    },
    orderBy: { updatedAt: "desc" },
  });
}

export async function listWorkflows(status?: string) {
  return (await import("@mcp-rebuild/db")).prisma.workflow.findMany({
    where: status ? { status } : undefined,
    orderBy: { updatedAt: "desc" },
  });
}

export async function getPlan(workflowId: string) {
  const workflow = await (await import("@mcp-rebuild/db")).prisma.workflow.findUniqueOrThrow({
    where: { id: workflowId },
  });

  if (!workflow.currentPlanId) {
    throw new Error("No active plan");
  }

  const plan = await (await import("@mcp-rebuild/db")).prisma.plan.findUniqueOrThrow({
    where: { id: workflow.currentPlanId },
  });

  const tasks = await (await import("@mcp-rebuild/db")).prisma.task.findMany({
    where: { planId: plan.id },
    orderBy: { createdAt: "asc" },
  });

  return { plan, tasks };
}

export async function getCurrentTask(workflowId: string) {
  const workflow = await (await import("@mcp-rebuild/db")).prisma.workflow.findUniqueOrThrow({
    where: { id: workflowId },
  });

  if (!workflow.currentTaskId) {
    throw new Error("No current task");
  }

  return (await import("@mcp-rebuild/db")).prisma.task.findUniqueOrThrow({
    where: { id: workflow.currentTaskId },
  });
}

export async function getResumeSuggestion(workflowId: string) {
  const workflow = await (await import("@mcp-rebuild/db")).prisma.workflow.findUniqueOrThrow({
    where: { id: workflowId },
  });

  if (!workflow.currentPlanId) {
    return {
      workflowId,
      status: workflow.status,
      suggestion: "No plan yet. Create a plan first.",
    };
  }

  if (!workflow.currentTaskId) {
    const plan = await (await import("@mcp-rebuild/db")).prisma.plan.findUniqueOrThrow({
      where: { id: workflow.currentPlanId },
    });
    const nextTodo = await (await import("@mcp-rebuild/db")).prisma.task.findFirst({
      where: { planId: plan.id, status: "todo" },
      orderBy: { createdAt: "asc" },
    });

    return {
      workflowId,
      status: workflow.status,
      currentPlanId: workflow.currentPlanId,
      suggestion: nextTodo
        ? `Start next task: "${nextTodo.title}" (id: ${nextTodo.id})`
        : "All tasks completed or in review.",
    };
  }

  const task = await (await import("@mcp-rebuild/db")).prisma.task.findUniqueOrThrow({
    where: { id: workflow.currentTaskId },
  });

  const latestProgress = await (await import("@mcp-rebuild/db")).prisma.taskProgressLog.findFirst({
    where: { taskId: task.id },
    orderBy: { createdAt: "desc" },
  });

  return {
    workflowId,
    status: workflow.status,
    currentPlanId: workflow.currentPlanId,
    currentTaskId: workflow.currentTaskId,
    currentTaskTitle: task.title,
    currentTaskStatus: task.status,
    lastProgressNote: latestProgress?.progressNote ?? null,
    suggestion:
      task.status === "in_progress"
        ? `Continue task: "${task.title}" — ${task.progressPercent}% complete`
        : task.status === "reviewing"
          ? `Review task: "${task.title}" — submit review decision`
          : task.status === "blocked"
            ? `Blocked task: "${task.title}" — resolve blocker`
            : `Task "${task.title}" is ${task.status}`,
  };
}
