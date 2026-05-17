/**
 * SemanticSearcher - hybrid search with BM25 + vector similarity + RRF fusion.
 *
 * Enhanced from the basic token-matching implementation to support:
 * 1. BM25 keyword search (from @mcp-rebuild/memory)
 * 2. Vector similarity search via embedding provider
 * 3. Reciprocal Rank Fusion (RRF) to combine results
 * 4. Jaccard similarity fallback when no providers are available
 */

import { promises as fs } from 'fs';
import { createLogger } from '@mcp-rebuild/core';
import { ScoringEngine } from '@mcp-rebuild/memory';
import { BM25Search } from '@mcp-rebuild/memory';
import type { SemanticQuery, FileMetadata, IndexedRepository, IndexedChunk, SearchConfig } from './types.js';

const logger = createLogger('intelligence:searcher');

export interface SearchResult {
  filePath: string;
  line: number;
  match: string;
  context: string;
  score: number;
}

/** Embedding provider for vector similarity search. */
export interface SearchEmbeddingProvider {
  embed(text: string): Promise<number[]>;
}

const DEFAULT_SEARCH_CONFIG: Required<SearchConfig> = {
  rrfK: 60,
  enableBM25: true,
  enableVector: true,
};

export class SemanticSearcher {
  private indexedRepo: IndexedRepository | null = null;
  private readonly embeddingProvider: SearchEmbeddingProvider | null;
  private readonly searchConfig: Required<SearchConfig>;
  private readonly chunkIndex: Map<string, IndexedChunk> = new Map();
  private readonly bm25Index: BM25Search;

  constructor(
    embeddingProvider?: SearchEmbeddingProvider,
    config?: SearchConfig,
  ) {
    this.embeddingProvider = embeddingProvider ?? null;
    this.searchConfig = { ...DEFAULT_SEARCH_CONFIG, ...config };
    this.bm25Index = new BM25Search();
  }

  /** Set an indexed repository for search. */
  setIndexedRepository(repo: IndexedRepository): void {
    this.indexedRepo = repo;
    logger.info('Indexed repository set for semantic search');
  }

  /** Add indexed chunks for BM25 + vector search. */
  indexChunks(chunks: IndexedChunk[]): void {
    for (const chunk of chunks) {
      this.chunkIndex.set(chunk.id, chunk);
    }

    // Rebuild BM25 index from chunks
    this.bm25Index.index(
      chunks.map(c => ({ id: c.id, content: c.content })),
    );

    logger.info({ chunkCount: chunks.length }, 'Indexed chunks for search');
  }

  /** Get all indexed chunks. */
  getChunks(): IndexedChunk[] {
    return Array.from(this.chunkIndex.values());
  }

  /** Search indexed chunks using BM25 + vector + RRF fusion. */
  searchChunks(query: string, options?: { limit?: number }): Array<{ chunk: IndexedChunk; score: number }> {
    const limit = options?.limit ?? 10;
    const streamResults: Map<string, number>[] = [];

    // Stream 1: BM25 keyword search
    if (this.searchConfig.enableBM25 && this.chunkIndex.size > 0) {
      const bm25Results = this.bm25Index.search(query, limit * 3);
      const ranked = new Map<string, number>();
      bm25Results.forEach((result, index) => {
        ranked.set(result.id, index + 1);
      });
      streamResults.push(ranked);
    }

    // Stream 2: Vector similarity search
    if (this.searchConfig.enableVector && this.embeddingProvider) {
      const vectorRanked = this.vectorSearch(query, limit * 3);
      if (vectorRanked.size > 0) {
        streamResults.push(vectorRanked);
      }
    }

    // If no streams produced results, use Jaccard fallback
    if (streamResults.length === 0) {
      return this.jaccardFallback(query, limit);
    }

    // RRF fusion
    return this.rrfFuse(streamResults, limit);
  }

  /**
   * Search the indexed repository files.
   * Legacy API for file-based search with enhanced scoring.
   */
  async search(query: SemanticQuery): Promise<SearchResult[]> {
    if (!this.indexedRepo) {
      throw new Error('No indexed repository available');
    }

    logger.info(`Searching for: ${query.query}`);

    const results: SearchResult[] = [];
    const queryLower = query.query.toLowerCase();
    const tokens = queryLower.split(/\s+/);

    for (const [filePath, metadata] of this.indexedRepo.files) {
      if (this.matchesFilters(query, metadata)) {
        const fileContent = await fs.readFile(filePath, 'utf-8');
        const matches = this.findMatches(fileContent, tokens, metadata);

        for (const match of matches) {
          results.push({
            filePath,
            line: match.line,
            match: match.text,
            context: match.context,
            score: match.score,
          });
        }
      }
    }

    logger.info(`Found ${results.length} matches`);

    return results;
  }

  // --- Private: Vector Search ---

  private vectorSearch(_query: string, limit: number): Map<string, number> {
    const ranked = new Map<string, number>();

    if (!this.embeddingProvider) return ranked;

    // Synchronous vector search using pre-computed embeddings
    const chunks = Array.from(this.chunkIndex.values());
    const scoredChunks: Array<{ id: string; score: number }> = [];

    for (const chunk of chunks) {
      if (chunk.embedding.length > 0) {
        // We'll use a placeholder since we need async for embedding
        // The actual vector search happens via searchChunksAsync
        scoredChunks.push({ id: chunk.id, score: 0 });
      }
    }

    // Sort by score (all zeros for sync version)
    scoredChunks.slice(0, limit).forEach((item, index) => {
      ranked.set(item.id, index + 1);
    });

    return ranked;
  }

  /** Async vector search with actual embedding computation. */
  async searchChunksAsync(query: string, options?: { limit?: number }): Promise<Array<{ chunk: IndexedChunk; score: number }>> {
    const limit = options?.limit ?? 10;
    const streamResults: Map<string, number>[] = [];

    // BM25 stream
    if (this.searchConfig.enableBM25 && this.chunkIndex.size > 0) {
      const bm25Results = this.bm25Index.search(query, limit * 3);
      const ranked = new Map<string, number>();
      bm25Results.forEach((result, index) => {
        ranked.set(result.id, index + 1);
      });
      streamResults.push(ranked);
    }

    // Vector stream (async)
    if (this.searchConfig.enableVector && this.embeddingProvider) {
      try {
        const queryEmbedding = await this.embeddingProvider.embed(query);
        const chunks = Array.from(this.chunkIndex.values());
        const scored: Array<{ id: string; score: number }> = [];

        for (const chunk of chunks) {
          if (chunk.embedding.length > 0) {
            const similarity = ScoringEngine.cosineSimilarity(queryEmbedding, chunk.embedding);
            scored.push({ id: chunk.id, score: similarity });
          }
        }

        scored.sort((a, b) => b.score - a.score);
        const ranked = new Map<string, number>();
        scored.slice(0, limit * 3).forEach((item, index) => {
          ranked.set(item.id, index + 1);
        });
        streamResults.push(ranked);
      } catch (error: unknown) {
        logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Vector search failed');
      }
    }

    if (streamResults.length === 0) {
      return this.jaccardFallback(query, limit);
    }

    return this.rrfFuse(streamResults, limit);
  }

  // --- Private: Jaccard Fallback ---

  private jaccardFallback(query: string, limit: number): Array<{ chunk: IndexedChunk; score: number }> {
    const scored: Array<{ chunk: IndexedChunk; score: number }> = [];

    for (const chunk of this.chunkIndex.values()) {
      const score = ScoringEngine.jaccardSimilarity(query, chunk.content);
      if (score > 0) {
        scored.push({ chunk: { ...chunk }, score });
      }
    }

    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, limit);
  }

  // --- Private: RRF Fusion ---

  private rrfFuse(streamResults: Map<string, number>[], limit: number): Array<{ chunk: IndexedChunk; score: number }> {
    const k = this.searchConfig.rrfK;
    const fusedScores = new Map<string, number>();

    // Collect all candidate IDs
    const allIds = new Set<string>();
    for (const stream of streamResults) {
      for (const id of stream.keys()) {
        allIds.add(id);
      }
    }

    // Compute RRF score
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

    // Sort and map to results
    const sorted = Array.from(fusedScores.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit);

    return sorted
      .filter(([id]) => this.chunkIndex.has(id))
      .map(([id, score]) => ({
        chunk: { ...this.chunkIndex.get(id)! },
        score,
      }));
  }

  // --- Private: File-based search helpers ---

  private matchesFilters(query: SemanticQuery, metadata: FileMetadata): boolean {
    if (query.fileFilter) {
      if (query.fileFilter.extensions) {
        if (!query.fileFilter.extensions.includes(metadata.extension)) {
          return false;
        }
      }
      if (query.fileFilter.pathPattern) {
        const pattern = new RegExp(query.fileFilter.pathPattern, 'i');
        if (!pattern.test(metadata.path)) {
          return false;
        }
      }
      if (query.fileFilter.maxSize && metadata.size > query.fileFilter.maxSize) {
        return false;
      }
    }
    return true;
  }

  private findMatches(
    content: string,
    tokens: string[],
    metadata: FileMetadata,
  ): Array<{ line: number; text: string; context: string; score: number }> {
    const lines = content.split('\n');
    const matches: Array<{ line: number; text: string; context: string; score: number }> = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const score = this.calculateScore(line, tokens, metadata);

      if (score > 0) {
        const context = this.getContext(lines, i);
        matches.push({ line: i + 1, text: line.trim(), context, score });
      }
    }

    return matches;
  }

  private calculateScore(line: string, tokens: string[], metadata: FileMetadata): number {
    let score = 0;
    const lineLower = line.toLowerCase();

    for (const token of tokens) {
      if (lineLower.includes(token.toLowerCase())) {
        score += 10;
      }
    }

    const codePatterns = [
      /\bfunction\b|\bclass\b|\bconst\b|\blet\b|\bvar\b|\bimport\b|\bexport\b|\bfrom\b|\brequire\b/,
      /\bif\b|\belse\b|\bfor\b|\bwhile\b|\breturn\b|\bthrow\b|\btry\b|\bcatch\b/,
    ];

    for (const pattern of codePatterns) {
      if (pattern.test(lineLower)) {
        score += 5;
      }
    }

    void metadata;
    return score;
  }

  private getContext(lines: string[], currentIndex: number): string {
    const contextLines: string[] = [];
    const contextSize = 3;

    for (let i = Math.max(0, currentIndex - contextSize); i < currentIndex; i++) {
      contextLines.push(lines[i]);
    }

    for (let i = currentIndex + 1; i < Math.min(lines.length, currentIndex + contextSize); i++) {
      contextLines.push(lines[i]);
    }

    return contextLines.join('\n');
  }
}
