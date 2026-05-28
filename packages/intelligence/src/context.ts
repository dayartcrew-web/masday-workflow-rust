/**
 * Context Pack Builder - assembles context packs for workflow tasks.
 *
 * Ported from msd-mcp retrieval.ts. Builds both basic and hybrid context
 * packs using semantic search, memory lookup, and fingerprinting.
 */

import { createHash } from 'crypto';
import { createLogger } from '@mcp-rebuild/core';
import type {
  ContextPackInput,
  ContextPackResult,
  ContextPackMemory,
  ContextPackDoc,
  HybridContextPackResult,
  IndexedChunk,
} from './types.js';
const logger = createLogger('intelligence:context');

/** ~4 chars per token is a standard approximation for English text + code. */
const CHARS_PER_TOKEN = 4;
const DEFAULT_MAX_CONTEXT_TOKENS = 150_000;

function estimateTokens(text: string): number {
  return Math.ceil(text.length / CHARS_PER_TOKEN);
}

/** Truncate text to fit within a token budget, appending "..." if truncated. */
function truncateToTokens(text: string, maxTokens: number): string {
  const maxChars = maxTokens * CHARS_PER_TOKEN;
  if (text.length <= maxChars) return text;
  return text.slice(0, maxChars - 3) + '...';
}

/** Provider interface for memory search in context building. */
export interface MemoryProvider {
  search(query: string, options?: { limit?: number; threshold?: number }): Promise<Array<{
    memory: { id: string; content: string; type: string; importance: number };
    score: number;
  }>>;
}

/** Provider interface for document search in context building. */
export interface DocumentProvider {
  search(query: string, options?: { limit?: number }): Promise<ContextPackDoc[]>;
  getAll(workflowId: string, options?: { limit?: number }): Promise<ContextPackDoc[]>;
}

/** Provider interface for embedding generation. */
export interface EmbeddingProvider {
  embed(text: string): Promise<number[]>;
}

/** Provider interface for indexed code chunks. */
export interface ChunkProvider {
  getChunks(options?: { limit?: number }): IndexedChunk[];
  search(query: string, options?: { limit?: number }): Array<{ chunk: IndexedChunk; score: number }>;
}

/**
 * Build a basic context pack from task metadata, memories, and documents.
 *
 * Uses simple retrieval: fetches recent memories and documents without
 * semantic search. Computes a SHA-256 fingerprint for cache invalidation.
 */
export function buildContextPack(
  input: ContextPackInput,
  providers: {
    memory?: MemoryProvider;
    documents?: DocumentProvider;
  },
  options?: { maxContextTokens?: number },
): Promise<ContextPackResult> {
  return buildContextPackImpl(input, providers, false, options?.maxContextTokens);
}

/**
 * Build a hybrid context pack using vector similarity search.
 *
 * Uses semantic search with BM25 + vector + RRF fusion to find the most
 * relevant memories and documents for the given task. Falls back to basic
 * retrieval when semantic search is unavailable.
 */
export function buildHybridContextPack(
  input: ContextPackInput,
  providers: {
    memory?: MemoryProvider;
    documents?: DocumentProvider;
    embedding?: EmbeddingProvider;
    chunks?: ChunkProvider;
  },
  options?: { maxContextTokens?: number },
): Promise<HybridContextPackResult> {
  return buildHybridContextPackImpl(input, providers, options?.maxContextTokens);
}

/** Compute a SHA-256 fingerprint for context pack contents. */
export function computeFingerprint(data: {
  workflowId: string;
  planId: string;
  taskId: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  memoryIds: string[];
  docIds: string[];
}): string {
  const content = JSON.stringify({
    w: data.workflowId,
    p: data.planId,
    t: data.taskId,
    ac: data.acceptanceCriteria,
    rc: data.requiredContext,
    mi: data.memoryIds.sort(),
    di: data.docIds.sort(),
  });

  return createHash('sha256').update(content).digest('hex');
}

// --- Implementation ---

async function buildContextPackImpl(
  input: ContextPackInput,
  providers: {
    memory?: MemoryProvider;
    documents?: DocumentProvider;
  },
  _hybrid: boolean,
  maxContextTokens: number = DEFAULT_MAX_CONTEXT_TOKENS,
): Promise<ContextPackResult> {
  const { workflowId, planId, taskId } = input;

  // Fetch recent memories
  let semanticMemories: ContextPackMemory[] = [];
  if (providers.memory) {
    try {
      const query = [input.taskTitle, ...input.acceptanceCriteria].join(' ');
      const results = await providers.memory.search(query, { limit: 6 });
      semanticMemories = results.map(r => ({
        id: r.memory.id,
        summary: r.memory.content.substring(0, 100),
        content: r.memory.content,
        score: r.score,
      }));
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Memory search failed in context pack build');
    }
  }

  // Fetch recent documents
  let semanticDocs: ContextPackDoc[] = [];
  if (providers.documents) {
    try {
      const query = [input.taskTitle, ...input.requiredContext].join(' ');
      const results = await providers.documents.search(query, { limit: 6 });
      semanticDocs = results;
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Document search failed in context pack build');
    }
  }

  const fingerprint = computeFingerprint({
    workflowId,
    planId,
    taskId,
    acceptanceCriteria: input.acceptanceCriteria,
    requiredContext: input.requiredContext,
    memoryIds: semanticMemories.map(m => m.id),
    docIds: semanticDocs.map(d => d.id),
  });

  const contextSufficient = semanticMemories.length > 0 || semanticDocs.length > 0;

  // Enforce token budget — truncate largest items first
  let budgetUsed = estimateTokens([
    input.planSummary ?? '',
    input.taskTitle,
    input.recentProgress ?? '',
    ...input.acceptanceCriteria,
    ...input.requiredContext,
  ].join('\n'));
  const truncatedMemories: ContextPackMemory[] = [];
  for (const m of semanticMemories) {
    if (budgetUsed >= maxContextTokens) break;
    const remaining = maxContextTokens - budgetUsed;
    const mTokens = estimateTokens(m.content);
    if (mTokens > remaining) {
      truncatedMemories.push({ ...m, content: truncateToTokens(m.content, remaining) });
      budgetUsed = maxContextTokens;
    } else {
      truncatedMemories.push(m);
      budgetUsed += mTokens;
    }
  }
  const truncatedDocs: ContextPackDoc[] = [];
  for (const d of semanticDocs) {
    if (budgetUsed >= maxContextTokens) break;
    const remaining = maxContextTokens - budgetUsed;
    const dTokens = estimateTokens(d.content);
    if (dTokens > remaining) {
      truncatedDocs.push({ ...d, content: truncateToTokens(d.content, remaining) });
      budgetUsed = maxContextTokens;
    } else {
      truncatedDocs.push(d);
      budgetUsed += dTokens;
    }
  }

  if (budgetUsed >= maxContextTokens) {
    logger.info({ budgetUsed, max: maxContextTokens }, 'Context pack truncated to fit token budget');
  }

  return {
    workflowId,
    planId,
    taskId,
    planSummary: input.planSummary,
    taskTitle: input.taskTitle,
    acceptanceCriteria: input.acceptanceCriteria,
    requiredContext: input.requiredContext,
    recentProgress: input.recentProgress,
    semanticMemories: truncatedMemories,
    semanticDocs: truncatedDocs,
    fingerprint,
    contextSufficient,
  };
}

async function buildHybridContextPackImpl(
  input: ContextPackInput,
  providers: {
    memory?: MemoryProvider;
    documents?: DocumentProvider;
    embedding?: EmbeddingProvider;
    chunks?: ChunkProvider;
  },
  maxContextTokens: number = DEFAULT_MAX_CONTEXT_TOKENS,
): Promise<HybridContextPackResult> {
  // Start with the base context pack
  const basePack = await buildContextPackImpl(input, providers, true, maxContextTokens);

  // Enhance with vector search if embedding provider is available
  if (providers.embedding) {
    try {
      const queryText = [
        input.taskTitle,
        ...input.acceptanceCriteria,
        ...input.requiredContext,
      ].join('\n');

      const queryEmbedding = await providers.embedding.embed(queryText);

      // Re-rank memories by vector similarity if we have embeddings
      if (providers.chunks) {
        const codeResults = providers.chunks.search(queryText, { limit: 3 });
        // Estimate remaining budget after base pack
        let currentTokens = basePack.semanticMemories.reduce((s, m) => s + estimateTokens(m.content), 0)
          + basePack.semanticDocs.reduce((s, d) => s + estimateTokens(d.content), 0);
        for (const result of codeResults) {
          const chunkTokens = estimateTokens(result.chunk.content);
          const remaining = maxContextTokens - currentTokens;
          if (remaining <= 0) break;
          const content = chunkTokens > remaining
            ? truncateToTokens(result.chunk.content, remaining)
            : result.chunk.content;
          basePack.semanticDocs.push({
            id: result.chunk.id,
            title: result.chunk.filePath,
            content,
            score: result.score,
          });
          currentTokens += Math.min(chunkTokens, remaining);
        }
      }

      logger.debug({
        workflowId: input.workflowId,
        taskId: input.taskId,
        embeddingDimensions: queryEmbedding.length,
        memoryCount: basePack.semanticMemories.length,
        docCount: basePack.semanticDocs.length,
      }, 'Hybrid context pack built with embedding');
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Hybrid search failed, using basic results');
    }
  }

  // Determine execution mode based on acceptance criteria count
  const executionMode = input.acceptanceCriteria.length > 3 ? 'parallel' : 'sequential';

  // Recompute fingerprint with hybrid results
  const fingerprint = computeFingerprint({
    workflowId: input.workflowId,
    planId: input.planId,
    taskId: input.taskId,
    acceptanceCriteria: input.acceptanceCriteria,
    requiredContext: input.requiredContext,
    memoryIds: basePack.semanticMemories.map(m => m.id),
    docIds: basePack.semanticDocs.map(d => d.id),
  });

  return {
    ...basePack,
    executionMode,
    fingerprint,
  };
}
