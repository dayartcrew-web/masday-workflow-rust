/**
 * Workflow creation (msd-mcp business logic)
 */

export async function createWorkflow(input: {
  name: string;
  projectPath?: string;
  metadata?: Record<string, unknown>;
}) {
  const { db, workflows } = await import("@mcp-rebuild/db");
  const [row] = await db.insert(workflows).values({
    name: input.name,
    status: "INIT",
    projectPath: input.projectPath,
    metadata: input.metadata ?? {},
  }).returning();
  return row;
}
