/**
 * Workflow creation (msd-mcp business logic)
 */

export async function createWorkflow(input: {
  name: string;
  projectPath?: string;
  metadata?: Record<string, unknown>;
}) {
  const { prisma } = await import("@mcp-rebuild/db");
  return prisma.workflow.create({
    data: {
      name: input.name,
      status: "planning",
      projectPath: input.projectPath,
      metadata: input.metadata ? (input.metadata as never) : {},
    },
  });
}
