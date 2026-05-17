import fs from 'fs';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import type { MemoryRecord, MemorySearchResult, ReflectionResult } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';
import { ScoringEngine } from './scoring.js';

const logger = createLogger('memory:store');

export interface MemoryStoreConfig {
  filePath: string;
  maxMemories?: number;
  embeddingService?: EmbeddingProvider;
}

export interface EmbeddingProvider {
  embed(text: string): Promise<number[]>;
}

export interface SearchOptions {
  limit?: number;
  threshold?: number;
  type?: string;
  tags?: string[];
}

export interface PruneOptions {
  maxAge?: number;
  minImportance?: number;
  maxMemories?: number;
}

/**
 * MemoryStore - file-based long-term memory storage with scoring.
 *
 * Supports:
 * - CRUD operations on memory records
 * - Scored search using similarity, recency, importance, usage weights
 * - Embedding integration for vector similarity
 * - Pruning and reflection result application
 */
export class MemoryStore {
  private memories: Map<string, MemoryRecord> = new Map();
  private readonly filePath: string;
  private readonly maxMemories: number;
  private readonly embeddingService: EmbeddingProvider | null;
  private readonly scoringEngine: ScoringEngine;
  private dirty: boolean = false;

  constructor(config: MemoryStoreConfig) {
    this.filePath = config.filePath;
    this.maxMemories = config.maxMemories ?? 1000;
    this.embeddingService = config.embeddingService ?? null;
    this.scoringEngine = new ScoringEngine();
  }

  /** Initialize the store, loading from file if it exists. */
  async init(): Promise<void> {
    try {
      if (fs.existsSync(this.filePath)) {
        const data = fs.readFileSync(this.filePath, 'utf-8');
        const parsed = JSON.parse(data) as MemoryRecord[];
        for (const record of parsed) {
          this.memories.set(record.id, record);
        }
        logger.info({ count: this.memories.size }, 'Loaded memories from file');
      } else {
        const dir = path.dirname(this.filePath);
        if (!fs.existsSync(dir)) {
          fs.mkdirSync(dir, { recursive: true });
        }
        logger.info('Initialized empty memory store');
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.error({ error: message }, 'Failed to initialize memory store');
      throw new Error(`Memory store initialization failed: ${message}`);
    }
  }

  /** Save current state to file. */
  async save(): Promise<void> {
    try {
      const dir = path.dirname(this.filePath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      const data = Array.from(this.memories.values());
      fs.writeFileSync(this.filePath, JSON.stringify(data, null, 2), 'utf-8');
      this.dirty = false;
      logger.debug({ count: data.length }, 'Saved memories to file');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.error({ error: message }, 'Failed to save memory store');
      throw new Error(`Memory store save failed: ${message}`);
    }
  }

  /** Add a new memory record. */
  async add(content: string, options?: { type?: string; importance?: number; tags?: string[]; source?: string }): Promise<MemoryRecord> {
    const now = Date.now();
    const record: MemoryRecord = {
      id: uuidv4(),
      content,
      type: (options?.type as MemoryRecord['type']) ?? 'fact',
      importance: options?.importance ?? 0.5,
      tags: options?.tags ?? [],
      source: options?.source ?? 'user',
      version: 1,
      createdAt: now,
      updatedAt: now,
      accessedAt: now,
      accessCount: 0,
    };

    // Generate embedding if service is available
    if (this.embeddingService) {
      try {
        record.embedding = await this.embeddingService.embed(content);
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        logger.warn({ error: message }, 'Failed to generate embedding for new memory');
      }
    }

    this.memories.set(record.id, record);
    this.dirty = true;
    logger.debug({ id: record.id, type: record.type }, 'Added new memory');
    return { ...record };
  }

  /** Get a memory by ID. */
  get(id: string): MemoryRecord | undefined {
    const record = this.memories.get(id);
    if (!record) return undefined;

    // Update access tracking
    const updated: MemoryRecord = {
      ...record,
      accessedAt: Date.now(),
      accessCount: record.accessCount + 1,
    };
    this.memories.set(id, updated);
    this.dirty = true;

    return { ...updated };
  }

  /** Update a memory record. */
  update(id: string, updates: Partial<Pick<MemoryRecord, 'content' | 'type' | 'importance' | 'tags'>>): MemoryRecord | undefined {
    const current = this.memories.get(id);
    if (!current) {
      logger.warn({ id }, 'Cannot update non-existent memory');
      return undefined;
    }

    const updated: MemoryRecord = {
      ...current,
      ...updates,
      updatedAt: Date.now(),
      version: current.version + 1,
    };

    this.memories.set(id, updated);
    this.dirty = true;
    logger.debug({ id, version: updated.version }, 'Updated memory');
    return { ...updated };
  }

  /** Delete a memory by ID. */
  delete(id: string): boolean {
    const existed = this.memories.delete(id);
    if (existed) {
      this.dirty = true;
      logger.debug({ id }, 'Deleted memory');
    }
    return existed;
  }

  /** Get all memories. */
  getAll(): MemoryRecord[] {
    return Array.from(this.memories.values()).map(m => ({ ...m }));
  }

  /** Search memories with scoring. */
  async search(query: string, options?: SearchOptions): Promise<MemorySearchResult[]> {
    const limit = options?.limit ?? 10;
    const threshold = options?.threshold ?? 0.1;
    const candidates = this.getFilteredMemories(options);

    if (candidates.length === 0) return [];

    // Generate query embedding if available
    let queryEmbedding: number[] | undefined;
    if (this.embeddingService) {
      try {
        queryEmbedding = await this.embeddingService.embed(query);
      } catch (error: unknown) {
        logger.warn('Failed to generate query embedding, using Jaccard fallback');
      }
    }

    const scored: MemorySearchResult[] = [];

    for (const memory of candidates) {
      let similarityScore: number;

      if (queryEmbedding && memory.embedding && memory.embedding.length > 0) {
        similarityScore = ScoringEngine.cosineSimilarity(queryEmbedding, memory.embedding);
      } else {
        similarityScore = ScoringEngine.jaccardSimilarity(query, memory.content);
      }

      const scoredMemory = this.scoringEngine.score(memory, similarityScore);
      if (scoredMemory.finalScore >= threshold) {
        scored.push({
          memory: { ...memory },
          score: scoredMemory.finalScore,
        });
      }
    }

    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, limit);
  }

  /** Prune memories based on age, importance, and count limits. */
  prune(options?: PruneOptions): string[] {
    const maxAge = options?.maxAge ?? 30 * 24 * 60 * 60 * 1000; // 30 days
    const minImportance = options?.minImportance ?? 0.1;
    const maxMemories = options?.maxMemories ?? this.maxMemories;

    const now = Date.now();
    const pruned: string[] = [];

    // Prune by age and importance
    for (const [id, memory] of this.memories) {
      const age = now - memory.createdAt;
      if (age > maxAge && memory.importance < minImportance) {
        this.memories.delete(id);
        pruned.push(id);
      }
    }

    // If still over limit, remove lowest importance
    if (this.memories.size > maxMemories) {
      const sorted = Array.from(this.memories.values())
        .sort((a, b) => a.importance - b.importance);

      const toRemove = sorted.slice(0, this.memories.size - maxMemories);
      for (const memory of toRemove) {
        this.memories.delete(memory.id);
        pruned.push(memory.id);
      }
    }

    if (pruned.length > 0) {
      this.dirty = true;
      logger.info({ count: pruned.length }, 'Pruned memories');
    }

    return pruned;
  }

  /** Apply reflection results (merges and deletions). */
  applyReflection(result: ReflectionResult): void {
    // Remove pruned memories
    for (const id of result.pruned) {
      this.memories.delete(id);
    }

    // Apply merges - remove sources, add merged
    for (const merge of result.merges) {
      for (const sourceId of merge.sourceIds) {
        this.memories.delete(sourceId);
      }

      const now = Date.now();
      const mergedRecord: MemoryRecord = {
        id: uuidv4(),
        content: merge.mergedContent,
        type: 'fact',
        importance: merge.newImportance,
        tags: [],
        source: 'reflection',
        version: 1,
        createdAt: now,
        updatedAt: now,
        accessedAt: now,
        accessCount: 0,
      };
      this.memories.set(mergedRecord.id, mergedRecord);
    }

    this.dirty = true;
    logger.info({ pruned: result.pruned.length, merges: result.merges.length }, 'Applied reflection');
  }

  /** Get store statistics. */
  getStats(): { total: number; byType: Record<string, number>; avgImportance: number; oldestMs: number } {
    const records = Array.from(this.memories.values());
    const byType: Record<string, number> = {};

    let totalImportance = 0;
    let oldest = Date.now();

    for (const record of records) {
      byType[record.type] = (byType[record.type] ?? 0) + 1;
      totalImportance += record.importance;
      if (record.createdAt < oldest) {
        oldest = record.createdAt;
      }
    }

    return {
      total: records.length,
      byType,
      avgImportance: records.length > 0 ? totalImportance / records.length : 0,
      oldestMs: Date.now() - oldest,
    };
  }

  /** Filter memories by search options. */
  private getFilteredMemories(options?: SearchOptions): MemoryRecord[] {
    let candidates = Array.from(this.memories.values());

    if (options?.type) {
      candidates = candidates.filter(m => m.type === options.type);
    }

    if (options?.tags && options.tags.length > 0) {
      candidates = candidates.filter(m =>
        options.tags!.some(tag => m.tags.includes(tag))
      );
    }

    return candidates;
  }
}
