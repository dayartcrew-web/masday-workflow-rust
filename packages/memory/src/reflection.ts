import { v4 as uuidv4 } from 'uuid';
import type { MemoryRecord, ReflectionResult, MemoryConflict, MemoryMerge } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider } from './classifier.js';
import { ScoringEngine } from './scoring.js';

const logger = createLogger('memory:reflection');

export interface ReflectionConfig {
  clusterThreshold?: number;
  maxAge?: number;
  minImportance?: number;
  maxMemories?: number;
}

const DEFAULT_CONFIG: Required<ReflectionConfig> = {
  clusterThreshold: 0.4,
  maxAge: 30 * 24 * 60 * 60 * 1000, // 30 days
  minImportance: 0.1,
  maxMemories: 1000,
};

/**
 * ReflectionEngine - analyzes memories for conflicts, merges, and pruning.
 *
 * Clusters memories by Jaccard similarity (>0.4 threshold by default).
 * Uses LLM-based cluster analysis when available, with rule-based fallback.
 * Prunes old, low-importance memories.
 */
export class ReflectionEngine {
  private readonly llmProvider: ILLMProvider | null;
  private readonly config: Required<ReflectionConfig>;

  constructor(llmProvider?: ILLMProvider, config?: ReflectionConfig) {
    this.llmProvider = llmProvider ?? null;
    this.config = { ...DEFAULT_CONFIG, ...config };
    logger.debug({ config: this.config, hasLLM: !!this.llmProvider }, 'ReflectionEngine initialized');
  }

  /** Run reflection over a set of memories. */
  async reflect(memories: MemoryRecord[]): Promise<ReflectionResult> {
    if (memories.length === 0) {
      return { conflicts: [], merges: [], pruned: [] };
    }

    // Cluster similar memories
    const clusters = this.clusterMemories(memories);
    logger.debug({ clusterCount: clusters.length }, 'Identified memory clusters');

    // Analyze clusters for conflicts and merges
    let conflicts: MemoryConflict[] = [];
    let merges: MemoryMerge[] = [];

    if (this.llmProvider) {
      try {
        const analysis = await this.llmAnalyzeClusters(clusters);
        conflicts = analysis.conflicts;
        merges = analysis.merges;
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        logger.warn({ error: message }, 'LLM cluster analysis failed, using rule-based fallback');
        const analysis = this.ruleBasedAnalysis(clusters);
        conflicts = analysis.conflicts;
        merges = analysis.merges;
      }
    } else {
      const analysis = this.ruleBasedAnalysis(clusters);
      conflicts = analysis.conflicts;
      merges = analysis.merges;
    }

    // Prune old/low-importance memories
    const pruned = this.pruneMemories(memories);

    return { conflicts, merges, pruned };
  }

  /** Cluster memories by Jaccard similarity. */
  private clusterMemories(memories: MemoryRecord[]): MemoryRecord[][] {
    const threshold = this.config.clusterThreshold;
    const visited = new Set<string>();
    const clusters: MemoryRecord[][] = [];

    for (const memory of memories) {
      if (visited.has(memory.id)) continue;
      visited.add(memory.id);

      const cluster: MemoryRecord[] = [memory];

      for (const other of memories) {
        if (visited.has(other.id)) continue;

        const similarity = ScoringEngine.computeSimilarity(memory, other);
        if (similarity >= threshold) {
          visited.add(other.id);
          cluster.push(other);
        }
      }

      if (cluster.length > 1) {
        clusters.push(cluster);
      }
    }

    return clusters;
  }

  /** Rule-based cluster analysis for conflicts and merges. */
  private ruleBasedAnalysis(clusters: MemoryRecord[][]): { conflicts: MemoryConflict[]; merges: MemoryMerge[] } {
    const conflicts: MemoryConflict[] = [];
    const merges: MemoryMerge[] = [];

    for (const cluster of clusters) {
      // Check for contradictions (different type tags on similar content)
      const types = new Set(cluster.map(m => m.type));
      if (types.size > 1) {
        conflicts.push({
          memoryIds: cluster.map(m => m.id),
          description: `Cluster contains memories with different types: ${[...types].join(', ')}`,
          resolution: 'Merge into a single memory with the most important type',
        });
      }

      // Merge similar memories
      if (cluster.length >= 2) {
        const sortedByImportance = [...cluster].sort((a, b) => b.importance - a.importance);
        const combinedContent = sortedByImportance.map(m => m.content).join(' ');
        const maxImportance = sortedByImportance[0].importance;

        // Only merge if content is not too long
        if (combinedContent.length <= 2000) {
          merges.push({
            sourceIds: cluster.map(m => m.id),
            mergedContent: combinedContent,
            newImportance: Math.min(1.0, maxImportance + 0.1),
          });
        }
      }
    }

    return { conflicts, merges };
  }

  /** LLM-based cluster analysis for conflicts and merges. */
  private async llmAnalyzeClusters(clusters: MemoryRecord[][]): Promise<{ conflicts: MemoryConflict[]; merges: MemoryMerge[] }> {
    const conflicts: MemoryConflict[] = [];
    const merges: MemoryMerge[] = [];

    for (const cluster of clusters) {
      const contents = cluster.map(m => `ID: ${m.id}\nType: ${m.type}\nContent: ${m.content}\nImportance: ${m.importance}`).join('\n\n');

      const prompt = `Analyze these similar memories for conflicts and potential merges:

${contents}

Respond with ONLY a JSON object:
{
  "hasConflict": boolean,
  "conflictDescription": string (if hasConflict),
  "conflictResolution": string (if hasConflict),
  "shouldMerge": boolean,
  "mergedContent": string (if shouldMerge, combine into one clear statement),
  "newImportance": number (0.0 to 1.0, if shouldMerge)
}`;

      const response = await this.llmProvider!.complete(prompt, {
        temperature: 0.1,
        maxTokens: 500,
      });

      try {
        const cleaned = response.text.trim().replace(/```json\n?/g, '').replace(/```\n?/g, '');
        const parsed = JSON.parse(cleaned) as {
          hasConflict: boolean;
          conflictDescription?: string;
          conflictResolution?: string;
          shouldMerge: boolean;
          mergedContent?: string;
          newImportance?: number;
        };

        if (parsed.hasConflict && parsed.conflictDescription) {
          conflicts.push({
            memoryIds: cluster.map(m => m.id),
            description: parsed.conflictDescription,
            resolution: parsed.conflictResolution ?? 'Review manually',
          });
        }

        if (parsed.shouldMerge && parsed.mergedContent) {
          merges.push({
            sourceIds: cluster.map(m => m.id),
            mergedContent: parsed.mergedContent,
            newImportance: Math.max(0, Math.min(1, parsed.newImportance ?? 0.7)),
          });
        }
      } catch (parseError: unknown) {
        logger.warn({ parseError }, 'Failed to parse LLM reflection response');
      }
    }

    return { conflicts, merges };
  }

  /** Prune memories based on age, importance, and count limits. */
  private pruneMemories(memories: MemoryRecord[]): string[] {
    const now = Date.now();
    const pruned: string[] = [];

    for (const memory of memories) {
      const age = now - memory.createdAt;
      if (age > this.config.maxAge && memory.importance < this.config.minImportance) {
        pruned.push(memory.id);
      }
    }

    if (memories.length - pruned.length > this.config.maxMemories) {
      const remaining = memories
        .filter(m => !pruned.includes(m.id))
        .sort((a, b) => a.importance - b.importance);

      const excess = remaining.length - this.config.maxMemories;
      for (let i = 0; i < excess; i++) {
        pruned.push(remaining[i].id);
      }
    }

    logger.debug({ pruned: pruned.length }, 'Pruned memories');
    return pruned;
  }
}
