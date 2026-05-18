/**
 * Progress tracking (msd-mcp business logic)
 */



export async function saveProgress(input: {
  workflowId: string;
  taskId: string;
  agentName: string;
  progressNote: string;
  evidence?: string[];
  statusBefore?: string;
  statusAfter?: string;
}) {
  return (await import("@mcp-rebuild/db")).prisma.taskProgressLog.create({
    data: {
      workflowId: input.workflowId,
      taskId: input.taskId,
      agentName: input.agentName,
      progressNote: input.progressNote,
      evidence: input.evidence ?? [],
      statusBefore: input.statusBefore,
      statusAfter: input.statusAfter,
    },
  });
}
