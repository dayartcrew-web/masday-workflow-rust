/**
 * Vector search (msd-mcp business logic)
 *
 * Uses pgvector for semantic similarity search on memories and context documents.
 */

import { sql } from "drizzle-orm";

interface MemorySearchResult {
  id: string;
  summary: string;
  content: string;
  score: number;
}

interface DocSearchResult {
  id: string;
  title: string | null;
  content: string;
  score: number;
}

export async function vectorSearchMemories(input: {
  workflowId: string;
  embedding: number[];
  limit?: number;
}): Promise<MemorySearchResult[]> {
  const vecStr = `[${input.embedding.join(",")}]`;
  const { db } = await import("@mcp-rebuild/db");

  const rows = await db.execute(sql`
    SELECT
      id,
      summary,
      content,
      1 - (embedding <=> ${vecStr}::vector) AS score
    FROM "Memory"
    WHERE "workflowId" = ${input.workflowId}
      AND embedding IS NOT NULL
    ORDER BY embedding <=> ${vecStr}::vector
    LIMIT ${input.limit ?? 5}
  `);

  return rows as unknown as MemorySearchResult[];
}

export async function vectorSearchContext(input: {
  workflowId: string;
  embedding: number[];
  limit?: number;
}): Promise<DocSearchResult[]> {
  const vecStr = `[${input.embedding.join(",")}]`;
  const { db } = await import("@mcp-rebuild/db");

  const rows = await db.execute(sql`
    SELECT
      id,
      title,
      content,
      1 - (embedding <=> ${vecStr}::vector) AS score
    FROM "ContextDocument"
    WHERE "workflowId" = ${input.workflowId}
      AND embedding IS NOT NULL
    ORDER BY embedding <=> ${vecStr}::vector
    LIMIT ${input.limit ?? 5}
  `);

  return rows as unknown as DocSearchResult[];
}
