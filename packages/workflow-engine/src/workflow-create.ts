/**
 * Workflow creation (msd-mcp business logic)
 */

import { prisma } from "@mcp-rebuild/db";

export async function createWorkflow(input: {
  name: string;
  projectPath?: string;
  metadata?: Record<string, unknown>;
}) {
  return prisma.workflow.create({
    data: {
      name: input.name,
      status: "planning",
      projectPath: input.projectPath,
      metadata: input.metadata ? (input.metadata as never) : {},
    },
  });
}
