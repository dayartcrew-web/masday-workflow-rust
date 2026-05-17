/**
 * Progress tracking (msd-mcp business logic)
 */

import { prisma } from "@mcp-rebuild/db";

export async function saveProgress(input: {
  workflowId: string;
  taskId: string;
  agentName: string;
  progressNote: string;
  evidence?: string[];
  statusBefore?: string;
  statusAfter?: string;
}) {
  return prisma.taskProgressLog.create({
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
