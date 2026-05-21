/**
 * Audit/retrieval logging (msd-mcp business logic)
 */

export async function logRetrieval(input: {
  workflowId?: string;
  taskId?: string;
  agentName: string;
  query: string;
  source: string;
  results: Record<string, unknown>;
}) {
  const { db, retrievalLogs } = await import("@mcp-rebuild/db");
  const [row] = await db.insert(retrievalLogs).values({
    workflowId: input.workflowId,
    taskId: input.taskId,
    agentName: input.agentName,
    query: input.query,
    source: input.source,
    results: input.results,
  }).returning();
  return row;
}
