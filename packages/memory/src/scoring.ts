import type { MemoryRecord, MemorySearchResult } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('memory:scoring');

export interface ScoringWeights {
  similarity: number;
  recency: number;
  importance: number;
  usage: number;
}

export const DEFAULT_WEIGHTS: ScoringWeights = {
  similarity: 0.6,
  recency: 0.15,
  importance: 0.15,
  usage: 0.1,
};

export interface ScoredMemory {
  memory: MemoryRecord;
  recency: number;
  usage: number;
  finalScore: number;
}

/**
 * ScoringEngine - computes relevance scores for memory records.
 *
 * Scoring formula:
 *   score = similarity * 0.6 + recency * 0.15 + importance * 0.15 + usage * 0.1
 *
 * Recency uses exponential decay with configurable half-life (default 7 days).
 * Usage uses logarithmic scaling.
 */
export class ScoringEngine {
  private readonly weights: ScoringWeights;
  private readonly halfLifeMs: number;

  constructor(weights?: Partial<ScoringWeights>, halfLifeDays: number = 7) {
    this.weights = { ...DEFAULT_WEIGHTS, ...weights };
    this.halfLifeMs = halfLifeDays * 24 * 60 * 60 * 1000;
    logger.debug({ weights: this.weights, halfLifeDays }, 'ScoringEngine initialized');
  }

  /** Compute recency score using exponential decay. */
  computeRecency(memory: MemoryRecord): number {
    const now = Date.now();
    const ageMs = now - memory.createdAt;
    if (ageMs < 0) return 1.0;
    return Math.pow(0.5, ageMs / this.halfLifeMs);
  }

  /** Compute usage score using logarithmic scaling. */
  computeUsage(memory: MemoryRecord): number {
    return Math.log(1 + memory.accessCount) / Math.log(1 + 100);
  }

  /** Score a single memory with all components. */
  score(memory: MemoryRecord, similarityScore: number = 0.5): ScoredMemory {
    const recency = this.computeRecency(memory);
    const usage = this.computeUsage(memory);

    const finalScore =
      similarityScore * this.weights.similarity +
      recency * this.weights.recency +
      memory.importance * this.weights.importance +
      usage * this.weights.usage;

    return {
      memory,
      recency,
      usage,
      finalScore,
    };
  }

  /** Re-rank an array of memories by computing full scores. */
  rerank(memories: MemorySearchResult[]): ScoredMemory[] {
    const scored = memories.map(result => this.score(result.memory, result.score));
    scored.sort((a, b) => b.finalScore - a.finalScore);
    return scored;
  }

  /** Compute cosine similarity between two vectors. */
  static cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length || a.length === 0) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denominator = Math.sqrt(normA) * Math.sqrt(normB);
    if (denominator === 0) return 0;

    return dotProduct / denominator;
  }

  /** Compute Jaccard similarity between two texts based on token overlap. */
  static jaccardSimilarity(a: string, b: string): number {
    const tokenize = (text: string): Set<string> => {
      return new Set(
        text
          .toLowerCase()
          .split(/\W+/)
          .filter(token => token.length > 2)
      );
    };

    const setA = tokenize(a);
    const setB = tokenize(b);

    if (setA.size === 0 && setB.size === 0) return 0;

    let intersection = 0;
    for (const token of setA) {
      if (setB.has(token)) {
        intersection++;
      }
    }

    const union = setA.size + setB.size - intersection;
    return union === 0 ? 0 : intersection / union;
  }

  /** Compute similarity between two memories (embedding-based or Jaccard fallback). */
  static computeSimilarity(a: MemoryRecord, b: MemoryRecord): number {
    if (a.embedding && b.embedding && a.embedding.length > 0 && a.embedding.length === b.embedding.length) {
      return ScoringEngine.cosineSimilarity(a.embedding, b.embedding);
    }
    return ScoringEngine.jaccardSimilarity(a.content, b.content);
  }
}
