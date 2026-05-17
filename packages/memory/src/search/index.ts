import type { MemoryRecord, MemorySearchResult } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';
import { BM25Search } from './bm25.js';
import { ScoringEngine } from '../scoring.js';

const logger = createLogger('memory:search');

export interface TripleStreamConfig {
  /** RRF constant k (default: 60) */
  rrfK?: number;
  /** Whether to enable BM25 stream */
  enableBM25?: boolean;
  /** Whether to enable vector similarity stream */
  enableVector?: boolean;
  /** Whether to enable knowledge graph stream */
  enableKG?: boolean;
}

export interface VectorProvider {
  embed(text: string): Promise<number[]>;
}

export interface KGProvider {
  getNeighbors(nodeId: string, options?: { relation?: string; direction?: 'in' | 'out' | 'both' }): Array<{ id: string; label: string }>;
  findNodes(predicate: (node: { id: string; label: string; type: string; properties: Record<string, unknown> }) => boolean): Array<{ id: string; label: string }>;
}

const DEFAULT_CONFIG: Required<TripleStreamConfig> = {
  rrfK: 60,
  enableBM25: true,
  enableVector: true,
  enableKG: true,
};

/**
 * TripleStreamSearch - combines BM25, Vector, and KG search using RRF fusion.
 *
 * RRF (Reciprocal Rank Fusion) formula:
 *   score = sum(1 / (k + rank_i)) for each stream
 *
 * This merges results from multiple search strategies into a unified ranking.
 */
export class TripleStreamSearch {
  private readonly config: Required<TripleStreamConfig>;
  private readonly bm25: BM25Search;
  private readonly vectorProvider: VectorProvider | null;
  private readonly kgProvider: KGProvider | null;

  constructor(
    config?: TripleStreamConfig,
    vectorProvider?: VectorProvider,
    kgProvider?: KGProvider,
  ) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.bm25 = new BM25Search();
    this.vectorProvider = vectorProvider ?? null;
    this.kgProvider = kgProvider ?? null;
  }

  /** Index memories for BM25 search. */
  indexMemories(memories: MemoryRecord[]): void {
    this.bm25.index(
      memories.map(m => ({ id: m.id, content: m.content }))
    );
    logger.debug({ count: memories.length }, 'Indexed memories for BM25');
  }

  /** Execute triple-stream search with RRF fusion. */
  async search(
    query: string,
    memories: MemoryRecord[],
    options?: { limit?: number },
  ): Promise<MemorySearchResult[]> {
    const limit = options?.limit ?? 10;
    const memoryMap = new Map<string, MemoryRecord>();
    for (const m of memories) {
      memoryMap.set(m.id, m);
    }

    const streamResults: Map<string, number>[] = [];
    const enabledStreams: string[] = [];

    // Stream 1: BM25 keyword search
    if (this.config.enableBM25) {
      const bm25Results = this.bm25.search(query, limit * 3);
      const ranked = new Map<string, number>();
      bm25Results.forEach((result, index) => {
        ranked.set(result.id, index + 1);
      });
      streamResults.push(ranked);
      enabledStreams.push('bm25');
    }

    // Stream 2: Vector similarity search
    if (this.config.enableVector && this.vectorProvider) {
      const vectorRanked = await this.vectorSearch(query, memories, limit * 3);
      streamResults.push(vectorRanked);
      enabledStreams.push('vector');
    }

    // Stream 3: Knowledge graph traversal
    if (this.config.enableKG && this.kgProvider) {
      const kgRanked = this.kgSearch(query, limit * 3);
      streamResults.push(kgRanked);
      enabledStreams.push('kg');
    }

    // Fallback: if no streams produced results, use simple Jaccard
    if (streamResults.length === 0) {
      return this.fallbackSearch(query, memories, limit);
    }

    // RRF fusion
    const fusedScores = this.rrfFuse(streamResults, memoryMap, limit);

    logger.debug(
      { query, streams: enabledStreams, resultCount: fusedScores.length },
      'Triple-stream search completed'
    );

    return fusedScores;
  }

  /** Vector similarity search stream. */
  private async vectorSearch(
    query: string,
    memories: MemoryRecord[],
    limit: number,
  ): Promise<Map<string, number>> {
    const ranked = new Map<string, number>();

    try {
      const queryEmbedding = await this.vectorProvider!.embed(query);
      const scored: Array<{ id: string; score: number }> = [];

      for (const memory of memories) {
        if (memory.embedding && memory.embedding.length > 0) {
          const similarity = ScoringEngine.cosineSimilarity(queryEmbedding, memory.embedding);
          scored.push({ id: memory.id, score: similarity });
        }
      }

      scored.sort((a, b) => b.score - a.score);
      scored.slice(0, limit).forEach((item, index) => {
        ranked.set(item.id, index + 1);
      });
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Vector search failed');
    }

    return ranked;
  }

  /** Knowledge graph search stream. */
  private kgSearch(query: string, limit: number): Map<string, number> {
    const ranked = new Map<string, number>();

    try {
      const queryTokens = new Set(query.toLowerCase().split(/\W+/).filter(t => t.length > 2));
      const matchingNodes = this.kgProvider!.findNodes(node => {
        const labelTokens = new Set(node.label.toLowerCase().split(/\W+/));
        for (const token of queryTokens) {
          if (labelTokens.has(token)) return true;
        }
        return false;
      });

      // Expand to neighbors and rank by distance
      const expanded = new Map<string, number>(); // id -> distance
      for (const node of matchingNodes) {
        expanded.set(node.id, 0);
        const neighbors = this.kgProvider!.getNeighbors(node.id);
        for (const neighbor of neighbors) {
          if (!expanded.has(neighbor.id)) {
            expanded.set(neighbor.id, 1);
          }
        }
      }

      // Rank by distance (closer = better rank)
      const sorted = Array.from(expanded.entries()).sort((a, b) => a[1] - b[1]);
      sorted.slice(0, limit).forEach(([id], index) => {
        ranked.set(id, index + 1);
      });
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'KG search failed');
    }

    return ranked;
  }

  /** Fallback search using Jaccard similarity when no streams are available. */
  private fallbackSearch(query: string, memories: MemoryRecord[], limit: number): MemorySearchResult[] {
    const scored: MemorySearchResult[] = memories.map(memory => ({
      memory,
      score: ScoringEngine.jaccardSimilarity(query, memory.content),
    }));

    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, limit);
  }

  /** Reciprocal Rank Fusion to merge multiple ranked lists. */
  private rrfFuse(
    streamResults: Map<string, number>[],
    memoryMap: Map<string, MemoryRecord>,
    limit: number,
  ): MemorySearchResult[] {
    const k = this.config.rrfK;
    const fusedScores = new Map<string, number>();

    // Collect all candidate IDs
    const allIds = new Set<string>();
    for (const stream of streamResults) {
      for (const id of stream.keys()) {
        allIds.add(id);
      }
    }

    // Compute RRF score for each candidate
    for (const id of allIds) {
      let score = 0;
      for (const stream of streamResults) {
        const rank = stream.get(id);
        if (rank !== undefined) {
          score += 1 / (k + rank);
        }
      }
      fusedScores.set(id, score);
    }

    // Sort by fused score
    const sorted = Array.from(fusedScores.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit);

    return sorted
      .filter(([id]) => memoryMap.has(id))
      .map(([id, score]) => ({
        memory: { ...memoryMap.get(id)! },
        score,
      }));
  }
}
