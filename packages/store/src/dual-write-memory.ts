import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('DualWriteMemory');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let prismaClient: any = null;

export function setDualWriteMemoryDb(client: unknown): void {
  prismaClient = client;
}

export interface DualWriteMemoryEntry {
  id: string;
  memoryType: string;
  summary: string;
  content: string;
  importanceScore?: number;
  createdByAgent: string;
  tags?: string[];
  workflowId?: string;
  taskId?: string;
}

/**
 * replicateMemory writes a memory record to PostgreSQL.
 * Call this after the primary MemoryStore saves to file.
 * Fire-and-forget — errors are logged but don't block the caller.
 */
export function replicateMemory(entry: DualWriteMemoryEntry): void {
  if (!prismaClient) return;

  const data = {
    id: entry.id,
    memoryType: entry.memoryType,
    summary: entry.summary,
    content: entry.content,
    importanceScore: entry.importanceScore ?? 0.5,
    createdByAgent: entry.createdByAgent,
    tags: entry.tags ?? [],
    workflowId: entry.workflowId ?? null,
    taskId: entry.taskId ?? null,
  };

  prismaClient.memory.upsert({
    where: { id: entry.id },
    update: {
      summary: data.summary,
      content: data.content,
      importanceScore: data.importanceScore,
      tags: data.tags,
      updatedAt: new Date(),
    },
    create: data,
  }).catch((err: unknown) => {
    logger.warn({ err: String(err), memoryId: entry.id }, 'Failed to replicate memory to PostgreSQL');
  });
}

export function replicateMemoryDelete(id: string): void {
  if (!prismaClient) return;

  prismaClient.memory.deleteMany({ where: { id } }).catch((err: unknown) => {
    logger.warn({ err: String(err), memoryId: id }, 'Failed to replicate memory deletion to PostgreSQL');
  });
}
