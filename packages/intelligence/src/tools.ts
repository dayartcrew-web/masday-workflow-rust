/**
 * MCP Tool Business Logic for semantic search and context retrieval.
 *
 * Provides the core logic for MCP tools:
 * - search_hybrid_context_pack: hybrid context pack with vector search
 * - search_context_fingerprint: context fingerprint and sufficiency check
 * - code_search: search indexed code with BM25 + vector fusion
 */

import { z } from 'zod';
import { createLogger } from '@mcp-rebuild/core';
import type {
  ContextPackInput,
  HybridContextPackResult,
  SearchResultWithChunk,
} from './types.js';
import {
  buildContextPack,
  buildHybridContextPack,
  type MemoryProvider,
  type DocumentProvider,
  type EmbeddingProvider,
  type ChunkProvider,
} from './context.js';

const logger = createLogger('intelligence:tools');

// --- Zod Schemas for MCP Tool Inputs ---

export const hybridContextPackInputSchema = z.object({
  workflow_id: z.string().min(1),
  plan_id: z.string().min(1),
  task_id: z.string().min(1),
  cwd: z.string().optional(),
});

export const contextFingerprintInputSchema = z.object({
  workflow_id: z.string().min(1),
  plan_id: z.string().min(1),
  task_id: z.string().min(1),
});

export const codeSearchInputSchema = z.object({
  query: z.string().min(1),
  limit: z.number().min(1).max(50).optional().default(10),
  language: z.string().optional(),
  threshold: z.number().min(0).max(1).optional().default(0.1),
});

// --- Inferred Types ---

export type HybridContextPackInput = z.infer<typeof hybridContextPackInputSchema>;
export type ContextFingerprintInput = z.infer<typeof contextFingerprintInputSchema>;
export type CodeSearchInput = z.infer<typeof codeSearchInputSchema>;

// --- Tool Providers ---

/** Provider for task/workflow metadata used by MCP tools. */
export interface TaskMetadataProvider {
  getTask(input: { workflowId: string; planId: string; taskId: string }): Promise<{
    planSummary: string;
    taskTitle: string;
    acceptanceCriteria: string[];
    requiredContext: string[];
    recentProgress: string[];
  }>;
}

/** Combined providers for all MCP tools. */
export interface ToolProviders {
  taskMetadata: TaskMetadataProvider;
  memory?: MemoryProvider;
  documents?: DocumentProvider;
  embedding?: EmbeddingProvider;
  chunks?: ChunkProvider;
}

// --- Tool: search_hybrid_context_pack ---

/**
 * Build a hybrid context pack using vector similarity search +
 * exact context + fingerprinting.
 */
export async function searchHybridContextPack(
  input: HybridContextPackInput,
  providers: ToolProviders,
): Promise<HybridContextPackResult> {
  const { workflow_id, plan_id, task_id } = input;

  const taskData = await providers.taskMetadata.getTask({
    workflowId: workflow_id,
    planId: plan_id,
    taskId: task_id,
  });

  const contextInput: ContextPackInput = {
    workflowId: workflow_id,
    planId: plan_id,
    taskId: task_id,
    planSummary: taskData.planSummary,
    taskTitle: taskData.taskTitle,
    acceptanceCriteria: taskData.acceptanceCriteria,
    requiredContext: taskData.requiredContext,
    recentProgress: taskData.recentProgress,
  };

  const pack = await buildHybridContextPack(contextInput, {
    memory: providers.memory,
    documents: providers.documents,
    embedding: providers.embedding,
    chunks: providers.chunks,
  });

  logger.info({
    workflowId: workflow_id,
    taskId: task_id,
    memoryCount: pack.semanticMemories.length,
    docCount: pack.semanticDocs.length,
    fingerprint: pack.fingerprint,
  }, 'Hybrid context pack built');

  return pack;
}

// --- Tool: search_context_fingerprint ---

/**
 * Get the current context fingerprint and sufficiency for a task.
 */
export async function searchContextFingerprint(
  input: ContextFingerprintInput,
  providers: ToolProviders,
): Promise<{ fingerprint: string; contextSufficient: boolean }> {
  const { workflow_id, plan_id, task_id } = input;

  const taskData = await providers.taskMetadata.getTask({
    workflowId: workflow_id,
    planId: plan_id,
    taskId: task_id,
  });

  const contextInput: ContextPackInput = {
    workflowId: workflow_id,
    planId: plan_id,
    taskId: task_id,
    planSummary: taskData.planSummary,
    taskTitle: taskData.taskTitle,
    acceptanceCriteria: taskData.acceptanceCriteria,
    requiredContext: taskData.requiredContext,
    recentProgress: taskData.recentProgress,
  };

  const pack = await buildContextPack(contextInput, {
    memory: providers.memory,
    documents: providers.documents,
  });

  return {
    fingerprint: pack.fingerprint,
    contextSufficient: pack.contextSufficient,
  };
}

// --- Tool: code_search ---

/**
 * Search indexed code using BM25 + vector similarity with RRF fusion.
 */
export async function codeSearch(
  input: CodeSearchInput,
  chunkProvider: ChunkProvider,
): Promise<SearchResultWithChunk[]> {
  const { query, limit, threshold } = input;

  const results = chunkProvider.search(query, { limit });

  const filtered = results.filter(r => r.score >= (threshold ?? 0.1));

  logger.info({
    query: query.substring(0, 50),
    resultCount: filtered.length,
  }, 'Code search completed');

  return filtered.map(r => ({
    chunk: { ...r.chunk },
    score: r.score,
    source: 'hybrid' as const,
  }));
}
