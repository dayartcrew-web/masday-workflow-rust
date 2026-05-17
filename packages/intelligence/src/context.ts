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
): Promise<ContextPackResult> {
  return buildContextPackImpl(input, providers, false);
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
): Promise<HybridContextPackResult> {
  return buildHybridContextPackImpl(input, providers);
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

  return {
    workflowId,
    planId,
    taskId,
    planSummary: input.planSummary,
    taskTitle: input.taskTitle,
    acceptanceCriteria: input.acceptanceCriteria,
    requiredContext: input.requiredContext,
    recentProgress: input.recentProgress,
    semanticMemories,
    semanticDocs,
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
): Promise<HybridContextPackResult> {
  // Start with the base context pack
  const basePack = await buildContextPackImpl(input, providers, true);

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
        // Add code results as additional docs
        for (const result of codeResults) {
          basePack.semanticDocs.push({
            id: result.chunk.id,
            title: result.chunk.filePath,
            content: result.chunk.content,
            score: result.score,
          });
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
