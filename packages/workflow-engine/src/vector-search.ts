/**
 * Vector search (msd-mcp business logic)
 *
 * Uses pgvector for semantic similarity search on memories and context documents.
 */

import { prisma } from "@mcp-rebuild/db";

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
  const vector = `[${input.embedding.join(",")}]`;

  const rows = await prisma.$queryRawUnsafe(
    `
    select
      id,
      summary,
      content,
      1 - (embedding <=> $2::vector) as score
    from "Memory"
    where "workflowId" = $1
      and embedding is not null
    order by embedding <=> $2::vector
    limit $3
    `,
    input.workflowId,
    vector,
    input.limit ?? 5,
  );

  return rows as MemorySearchResult[];
}

export async function vectorSearchContext(input: {
  workflowId: string;
  embedding: number[];
  limit?: number;
}): Promise<DocSearchResult[]> {
  const vector = `[${input.embedding.join(",")}]`;

  const rows = await prisma.$queryRawUnsafe(
    `
    select
      id,
      title,
      content,
      1 - (embedding <=> $2::vector) as score
    from "ContextDocument"
    where "workflowId" = $1
      and embedding is not null
    order by embedding <=> $2::vector
    limit $3
    `,
    input.workflowId,
    vector,
    input.limit ?? 5,
  );

  return rows as DocSearchResult[];
}
