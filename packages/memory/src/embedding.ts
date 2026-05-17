import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('memory:embedding');

export interface EmbeddingConfig {
  apiKey?: string;
  baseUrl?: string;
  model?: string;
  dimensions?: number;
  cacheEnabled?: boolean;
  maxCacheSize?: number;
}

const DEFAULT_CONFIG: Required<EmbeddingConfig> = {
  apiKey: '',
  baseUrl: 'https://api.openai.com/v1',
  model: 'text-embedding-3-small',
  dimensions: 1536,
  cacheEnabled: true,
  maxCacheSize: 1000,
};

/**
 * EmbeddingService - generates text embeddings via OpenAI-compatible API.
 * Includes caching and batch embedding support.
 */
export class EmbeddingService {
  private readonly config: Required<EmbeddingConfig>;
  private readonly cache: Map<string, number[]> = new Map();

  constructor(config?: EmbeddingConfig) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    logger.debug({ model: this.config.model, dimensions: this.config.dimensions }, 'EmbeddingService initialized');
  }

  /** Embed a single text string. Returns a cached result if available. */
  async embed(text: string): Promise<number[]> {
    if (this.config.cacheEnabled) {
      const cached = this.cache.get(text);
      if (cached) {
        logger.debug('Embedding cache hit');
        return [...cached];
      }
    }

    const embedding = await this.callEmbeddingApi([text]);
    const result = embedding[0];

    if (this.config.cacheEnabled) {
      if (this.cache.size >= this.config.maxCacheSize) {
        const firstKey = this.cache.keys().next().value;
        if (firstKey !== undefined) {
          this.cache.delete(firstKey);
        }
      }
      this.cache.set(text, [...result]);
    }

    return result;
  }

  /** Embed multiple texts with concurrency limit. */
  async embedBatch(texts: string[], concurrency: number = 5): Promise<number[][]> {
    const results: number[][] = [];
    for (let i = 0; i < texts.length; i += concurrency) {
      const batch = texts.slice(i, i + concurrency);
      const batchResults = await this.callEmbeddingApi(batch);
      results.push(...batchResults);
    }
    return results;
  }

  /** Compute cosine similarity between two embedding vectors. */
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

  /** Call the OpenAI-compatible embedding API. */
  private async callEmbeddingApi(texts: string[]): Promise<number[][]> {
    if (!this.config.apiKey) {
      throw new Error('Embedding API key not configured. Set apiKey in EmbeddingConfig.');
    }

    const url = `${this.config.baseUrl}/embeddings`;
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.config.apiKey}`,
      },
      body: JSON.stringify({
        model: this.config.model,
        input: texts,
        dimensions: this.config.dimensions,
      }),
    });

    if (!response.ok) {
      throw new Error(`Embedding API error: status ${response.status}`);
    }

    const data = await response.json() as {
      data: Array<{ embedding: number[]; index: number }>;
    };

    const sorted = data.data.sort((a, b) => a.index - b.index);
    return sorted.map(item => item.embedding);
  }
}

/**
 * MockEmbeddingService - deterministic embedding provider for testing.
 * Generates consistent vectors based on text content hash.
 */
export class MockEmbeddingService {
  private readonly dimensions: number;

  constructor(dimensions: number = 128) {
    this.dimensions = dimensions;
  }

  /** Generate a deterministic embedding vector from text. */
  async embed(text: string): Promise<number[]> {
    return this.textToVector(text);
  }

  /** Generate deterministic embeddings for multiple texts. */
  async embedBatch(texts: string[]): Promise<number[][]> {
    return texts.map(text => this.textToVector(text));
  }

  /** Convert text to a deterministic normalized vector. */
  private textToVector(text: string): number[] {
    const vector: number[] = [];
    let seed = this.hashString(text);

    for (let i = 0; i < this.dimensions; i++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      vector.push((seed / 0x7fffffff) * 2 - 1);
    }

    // Normalize to unit length
    const norm = Math.sqrt(vector.reduce((sum, v) => sum + v * v, 0));
    if (norm === 0) return vector;
    return vector.map(v => v / norm);
  }

  /** Simple string hash for deterministic seeding. */
  private hashString(str: string): number {
    let hash = 5381;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) + hash + str.charCodeAt(i)) & 0x7fffffff;
    }
    return hash;
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
}
