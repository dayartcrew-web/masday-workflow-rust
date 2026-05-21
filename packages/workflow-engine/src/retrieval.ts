/**
 * Context retrieval / context pack building (msd-mcp business logic)
 */

import { eq, desc } from "drizzle-orm";
import type { ContextPack } from "@mcp-rebuild/core";
import { logger } from "@mcp-rebuild/shared-utils";
import { createEmbeddingProvider } from "./embedding.js";
import { vectorSearchContext, vectorSearchMemories } from "./vector-search.js";
import { makeFingerprint } from "./fingerprint.js";
import { logRetrieval } from "./audit.js";

const embedder = createEmbeddingProvider();

export async function buildContextPack(
  workflowId: string,
  planId: string,
  taskId: string,
): Promise<ContextPack> {
  const { db, plans, tasks, taskProgressLogs, memories, contextDocuments } = await import("@mcp-rebuild/db");

  const [plan] = await db.select().from(plans).where(eq(plans.id, planId)).limit(1);
  if (!plan) throw new Error(`Plan ${planId} not found`);

  const [task] = await db.select().from(tasks).where(eq(tasks.id, taskId)).limit(1);
  if (!task) throw new Error(`Task ${taskId} not found`);

  const progress = await db.select().from(taskProgressLogs)
    .where(eq(taskProgressLogs.taskId, taskId))
    .orderBy(desc(taskProgressLogs.createdAt))
    .limit(5);

  const mems = await db.select().from(memories)
    .where(eq(memories.workflowId, workflowId))
    .orderBy(desc(memories.createdAt))
    .limit(5);

  const docs = await db.select().from(contextDocuments)
    .where(eq(contextDocuments.workflowId, workflowId))
    .orderBy(desc(contextDocuments.createdAt))
    .limit(5);

  const memoryIds = mems.map((m) => m.id);
  const docIds = docs.map((d) => d.id);

  const fingerprint = makeFingerprint({
    workflowId,
    planId,
    taskId,
    acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
    requiredContext: (task.requiredContext as string[]) ?? [],
    documentIds: docIds,
    memoryIds,
  });

  return {
    workflowId,
    planId,
    taskId,
    planSummary: plan.summary,
    taskTitle: task.title,
    acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
    requiredContext: (task.requiredContext as string[]) ?? [],
    recentProgress: progress.map(
      (p) => p.progressNote,
    ),
    semanticMemories: mems.map(
      (m) => ({
        id: m.id,
        summary: m.summary,
        content: m.content,
      }),
    ),
    semanticDocs: docs.map(
      (d) => ({
        id: d.id,
        title: d.title,
        content: d.content,
      }),
    ),
    fingerprint,
    contextSufficient: docs.length > 0 || mems.length > 0,
  };
}

export async function buildHybridContextPack(
  workflowId: string,
  planId: string,
  taskId: string,
): Promise<ContextPack> {
  const { db, plans, tasks, taskProgressLogs } = await import("@mcp-rebuild/db");

  const [plan] = await db.select().from(plans).where(eq(plans.id, planId)).limit(1);
  if (!plan) throw new Error(`Plan ${planId} not found`);

  const [task] = await db.select().from(tasks).where(eq(tasks.id, taskId)).limit(1);
  if (!task) throw new Error(`Task ${taskId} not found`);

  const queryText = [
    task.title,
    ...((task.acceptanceCriteria as string[]) ?? []),
    ...((task.requiredContext as string[]) ?? []),
  ].join("\n");

  const queryEmbedding = await embedder.embed(queryText);

  const recentProgress = await db.select().from(taskProgressLogs)
    .where(eq(taskProgressLogs.taskId, taskId))
    .orderBy(desc(taskProgressLogs.createdAt))
    .limit(5);

  const semanticMemories = await vectorSearchMemories({
    workflowId,
    embedding: queryEmbedding,
    limit: 6,
  });

  const semanticDocs = await vectorSearchContext({
    workflowId,
    embedding: queryEmbedding,
    limit: 6,
  });

  const fingerprint = makeFingerprint({
    workflowId,
    planId,
    taskId,
    acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
    requiredContext: (task.requiredContext as string[]) ?? [],
    documentIds: semanticDocs.map((d: { id: string }) => d.id),
    memoryIds: semanticMemories.map((m: { id: string }) => m.id),
  });

  await db.update(tasks).set({ contextFingerprint: fingerprint }).where(eq(tasks.id, taskId));

  logger.info("hybrid_context_pack_built", {
    workflowId,
    taskId,
    memoryCount: semanticMemories.length,
    docCount: semanticDocs.length,
    fingerprint,
  });

  await logRetrieval({
    workflowId,
    taskId,
    agentName: "system",
    query: queryText,
    source: "hybrid",
    results: {
      memories: semanticMemories.length,
      docs: semanticDocs.length,
    },
  });

  return {
    workflowId,
    planId,
    taskId,
    planSummary: plan.summary,
    taskTitle: task.title,
    acceptanceCriteria: (task.acceptanceCriteria as string[]) ?? [],
    requiredContext: (task.requiredContext as string[]) ?? [],
    recentProgress: recentProgress.map(
      (p) => p.progressNote,
    ),
    semanticMemories,
    semanticDocs,
    fingerprint,
    contextSufficient: semanticDocs.length > 0 || semanticMemories.length > 0,
  };
}
