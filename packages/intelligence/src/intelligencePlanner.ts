/**
 * IntelligencePlanner - Memory-aware planning with semantic search.
 *
 * Enhances task planning by:
 * 1. Searching past solutions and patterns from memory
 * 2. Using code context from semantic search
 * 3. Learning from past execution metrics
 * 4. Generating suggestions based on relevant context
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider } from '@mcp-rebuild/llm';
import type {
  PlanSuggestion,
  IntelligencePlanResult,
  PlannerConfig,
  SearchResult,
} from './types.js';
import type { SemanticSearcher } from './semanticSearcher.js';
import type { LearningSystem } from './learningSystem.js';

const logger = createLogger('intelligence:planner');

const DEFAULT_CONFIG: Required<Pick<PlannerConfig, 'maxMemoryResults' | 'maxCodeResults' | 'minConfidence'>> = {
  maxMemoryResults: 5,
  maxCodeResults: 5,
  minConfidence: 0.3,
};

/** Provider interface for memory search in planning. */
export interface PlannerMemoryProvider {
  search(query: string, options?: { limit?: number; threshold?: number }): Promise<Array<{
    memory: { id: string; content: string; type: string; importance: number };
    score: number;
  }>>;
}

/**
 * IntelligencePlanner generates memory-aware plans for task execution.
 *
 * Instead of relying on external planner packages, it uses the LLM
 * to generate suggestions based on retrieved context from memory
 * and semantic code search.
 */
export class IntelligencePlanner {
  private readonly llm: ILLMProvider | null;
  private readonly memory: PlannerMemoryProvider | null;
  private readonly searcher: SemanticSearcher | null;
  private readonly learning: LearningSystem | null;
  private readonly config: Required<Pick<PlannerConfig, 'maxMemoryResults' | 'maxCodeResults' | 'minConfidence'>>;

  constructor(config?: {
    llm?: ILLMProvider;
    memory?: PlannerMemoryProvider;
    searcher?: SemanticSearcher;
    learning?: LearningSystem;
    maxMemoryResults?: number;
    maxCodeResults?: number;
    minConfidence?: number;
  }) {
    this.llm = config?.llm ?? null;
    this.memory = config?.memory ?? null;
    this.searcher = config?.searcher ?? null;
    this.learning = config?.learning ?? null;
    this.config = {
      maxMemoryResults: config?.maxMemoryResults ?? DEFAULT_CONFIG.maxMemoryResults,
      maxCodeResults: config?.maxCodeResults ?? DEFAULT_CONFIG.maxCodeResults,
      minConfidence: config?.minConfidence ?? DEFAULT_CONFIG.minConfidence,
    };
  }

  /**
   * Generate an intelligence plan for a goal.
   *
   * Retrieves relevant memories and code context, then optionally
   * uses the LLM to generate suggestions. Falls back to heuristic
   * suggestions when no LLM is available.
   */
  async plan(goal: string): Promise<IntelligencePlanResult> {
    // Gather context from memory
    let relevantMemories: Array<{
      memory: { id: string; content: string; type: string; importance: number };
      score: number;
    }> = [];

    if (this.memory) {
      try {
        relevantMemories = await this.memory.search(goal, {
          limit: this.config.maxMemoryResults,
          threshold: 0.1,
        });
      } catch (error: unknown) {
        logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Memory search failed in planner');
      }
    }

    // Gather code context
    let relevantCode: SearchResult[] = [];
    if (this.searcher) {
      try {
        const patterns = this.extractPatterns(goal);
        relevantCode = await this.searcher.search({
          query: goal,
          patterns,
        });
        relevantCode = relevantCode.slice(0, this.config.maxCodeResults);
      } catch (error: unknown) {
        logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Code search failed in planner');
      }
    }

    // Generate suggestions
    let suggestions: PlanSuggestion[];
    if (this.llm) {
      suggestions = await this.generateLLMSuggestions(goal, relevantMemories, relevantCode);
    } else {
      suggestions = this.generateHeuristicSuggestions(goal, relevantMemories);
    }

    // Filter by minimum confidence
    const filteredSuggestions = suggestions.filter(
      s => s.confidence >= this.config.minConfidence,
    );

    return {
      goal,
      suggestions: filteredSuggestions,
      relevantMemories,
      relevantCode,
      generatedAt: Date.now(),
    };
  }

  /**
   * Get learning-based suggestions for a skill.
   */
  getSkillInsights(skill: string): Array<{ type: string; message: string }> {
    if (!this.learning) return [];
    return this.learning.getOptimizationSuggestions(skill, '');
  }

  // --- Private Methods ---

  private async generateLLMSuggestions(
    goal: string,
    memories: Array<{ memory: { id: string; content: string; type: string; importance: number }; score: number }>,
    code: SearchResult[],
  ): Promise<PlanSuggestion[]> {
    if (!this.llm) return [];

    const memoryContext = memories.length > 0
      ? memories.map(m => `[${m.memory.type}, score: ${m.score.toFixed(2)}] ${m.memory.content}`).join('\n')
      : 'No relevant memories found.';

    const codeContext = code.length > 0
      ? code.map(c => `[${c.filePath}:${c.line}] ${c.match}`).join('\n')
      : 'No relevant code found.';

    const prompt = `Analyze the following goal and generate planning suggestions.

Goal: ${goal}

Relevant Memories:
${memoryContext}

Relevant Code:
${codeContext}

Generate 3-5 suggestions in JSON array format:
[{"type": "approach|pattern|risk|dependency", "content": "suggestion text", "confidence": 0.0-1.0, "source": "memory|code|inference"}]

Output ONLY the JSON array, no other text.`;

    try {
      const response = await this.llm.complete(prompt, { temperature: 0.5 });
      const jsonMatch = response.text.match(/\[[\s\S]*\]/);

      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0]) as Array<Record<string, unknown>>;
        return parsed.map(item => ({
          type: (item.type as PlanSuggestion['type']) || 'approach',
          content: (item.content as string) || '',
          confidence: typeof item.confidence === 'number' ? item.confidence : 0.5,
          source: (item.source as string) || 'inference',
        }));
      }
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'LLM suggestion generation failed');
    }

    // Fallback to heuristic suggestions
    return this.generateHeuristicSuggestions(goal, memories);
  }

  private generateHeuristicSuggestions(
    goal: string,
    memories: Array<{ memory: { id: string; content: string; type: string; importance: number }; score: number }>,
  ): PlanSuggestion[] {
    const suggestions: PlanSuggestion[] = [];

    // Memory-based suggestions
    for (const mem of memories) {
      if (mem.score > 0.5) {
        suggestions.push({
          type: 'pattern',
          content: `Past solution: ${mem.memory.content.substring(0, 200)}`,
          confidence: Math.min(mem.score, 1.0),
          source: 'memory',
        });
      }
    }

    // Goal-based heuristics
    const lowerGoal = goal.toLowerCase();

    if (lowerGoal.includes('implement') || lowerGoal.includes('create') || lowerGoal.includes('add')) {
      suggestions.push({
        type: 'approach',
        content: 'Start with TDD: write tests first, then implement to pass them',
        confidence: 0.7,
        source: 'inference',
      });
    }

    if (lowerGoal.includes('fix') || lowerGoal.includes('bug')) {
      suggestions.push({
        type: 'approach',
        content: 'Use systematic debugging: reproduce, trace root cause, fix, verify',
        confidence: 0.8,
        source: 'inference',
      });
    }

    if (lowerGoal.includes('refactor')) {
      suggestions.push({
        type: 'risk',
        content: 'Ensure all existing tests pass before refactoring',
        confidence: 0.9,
        source: 'inference',
      });
    }

    if (memories.length === 0) {
      suggestions.push({
        type: 'dependency',
        content: 'No relevant past context found. Consider researching existing patterns first.',
        confidence: 0.6,
        source: 'inference',
      });
    }

    return suggestions;
  }

  private extractPatterns(requirements: string): string[] {
    const patterns: string[] = [];
    const lowerReqs = requirements.toLowerCase();

    const actionPatterns = [
      /read|load|get|fetch/,
      /write|create|save|update/,
      /delete|remove|clean/,
      /list|dir|scan/,
      /search|find|grep/,
      /build|compile|transpile/,
      /test|spec|verify/,
      /lint|format|check/,
    ];

    for (const pattern of actionPatterns) {
      if (pattern.test(lowerReqs)) {
        patterns.push(pattern.source.replace(/\\/g, ''));
      }
    }

    return patterns;
  }
}
