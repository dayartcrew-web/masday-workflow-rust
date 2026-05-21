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
  const { db, taskProgressLogs } = await import("@mcp-rebuild/db");
  const [row] = await db.insert(taskProgressLogs).values({
    workflowId: input.workflowId,
    taskId: input.taskId,
    agentName: input.agentName,
    progressNote: input.progressNote,
    evidence: input.evidence ?? [],
    statusBefore: input.statusBefore,
    statusAfter: input.statusAfter,
  }).returning();
  return row;
}
