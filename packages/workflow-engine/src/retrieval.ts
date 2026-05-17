/**
 * Context retrieval / context pack building (msd-mcp business logic)
 */

import { prisma } from "@mcp-rebuild/db";
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
  const plan = await prisma.plan.findUniqueOrThrow({ where: { id: planId } });
  const task = await prisma.task.findUniqueOrThrow({ where: { id: taskId } });

  const progress = await prisma.taskProgressLog.findMany({
    where: { taskId },
    orderBy: { createdAt: "desc" },
    take: 5,
  });

  const memories = await prisma.memory.findMany({
    where: { workflowId },
    orderBy: { createdAt: "desc" },
    take: 5,
  });

  const docs = await prisma.contextDocument.findMany({
    where: { workflowId },
    orderBy: { createdAt: "desc" },
    take: 5,
  });

  const memoryIds = memories.map((m: { id: string }) => m.id);
  const docIds = docs.map((d: { id: string }) => d.id);

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
      (p: { progressNote: string }) => p.progressNote,
    ),
    semanticMemories: memories.map(
      (m: { id: string; summary: string; content: string }) => ({
        id: m.id,
        summary: m.summary,
        content: m.content,
      }),
    ),
    semanticDocs: docs.map(
      (d: { id: string; title: string | null; content: string }) => ({
        id: d.id,
        title: d.title,
        content: d.content,
      }),
    ),
    fingerprint,
    contextSufficient: docs.length > 0 || memories.length > 0,
  };
}

export async function buildHybridContextPack(
  workflowId: string,
  planId: string,
  taskId: string,
): Promise<ContextPack> {
  const plan = await prisma.plan.findUniqueOrThrow({ where: { id: planId } });
  const task = await prisma.task.findUniqueOrThrow({ where: { id: taskId } });

  const queryText = [
    task.title,
    ...((task.acceptanceCriteria as string[]) ?? []),
    ...((task.requiredContext as string[]) ?? []),
  ].join("\n");

  const queryEmbedding = await embedder.embed(queryText);

  const recentProgress = await prisma.taskProgressLog.findMany({
    where: { taskId },
    orderBy: { createdAt: "desc" },
    take: 5,
  });

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

  await prisma.task.update({
    where: { id: taskId },
    data: { contextFingerprint: fingerprint },
  });

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
      (p: { progressNote: string }) => p.progressNote,
    ),
    semanticMemories,
    semanticDocs,
    fingerprint,
    contextSufficient: semanticDocs.length > 0 || semanticMemories.length > 0,
  };
}
