import type { MemoryType } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('memory:classifier');

/**
 * Minimal LLM provider interface for classification.
 * The real provider will be wired in Phase 3.
 */
export interface ILLMProvider {
  complete(prompt: string, options?: { temperature?: number; maxTokens?: number }): Promise<{ text: string }>;
}

export interface ClassificationResult {
  shouldSave: boolean;
  type: MemoryType;
  importance: number;
  tags: string[];
  reason: string;
}

const MEMORY_TYPES: MemoryType[] = ['fact', 'preference', 'skill', 'experience', 'strategy', 'decision', 'artifact', 'learning', 'blocker'];

const KEYWORD_RULES: Array<{ keywords: string[]; type: MemoryType; importance: number }> = [
  { keywords: ['prefer', 'like', 'want', 'favorite', 'always use', 'never use'], type: 'preference', importance: 0.8 },
  { keywords: ['learned', 'discovered', 'found that', 'insight', 'takeaway'], type: 'learning', importance: 0.7 },
  { keywords: ['decided', 'decision', 'chose to', 'going with', 'agreed'], type: 'decision', importance: 0.8 },
  { keywords: ['error', 'bug', 'issue', 'problem', 'fix', 'blocker', 'stuck'], type: 'blocker', importance: 0.9 },
  { keywords: ['skill', 'tool', 'framework', 'library', 'technique', 'pattern'], type: 'skill', importance: 0.6 },
  { keywords: ['strategy', 'approach', 'plan', 'method', 'workflow'], type: 'strategy', importance: 0.7 },
  { keywords: ['artifact', 'file', 'document', 'code', 'component', 'module'], type: 'artifact', importance: 0.5 },
];

/**
 * MemoryClassifier - classifies input text for memory storage.
 *
 * Uses LLM-based classification when available, with keyword-based fallback.
 */
export class MemoryClassifier {
  private readonly llmProvider: ILLMProvider | null;

  constructor(llmProvider?: ILLMProvider) {
    this.llmProvider = llmProvider ?? null;
    logger.debug({ hasLLM: !!this.llmProvider }, 'MemoryClassifier initialized');
  }

  /** Classify input text to determine if and how it should be stored. */
  async classify(input: string): Promise<ClassificationResult> {
    if (this.llmProvider) {
      try {
        return await this.llmClassify(input);
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        logger.warn({ error: message }, 'LLM classification failed, using keyword fallback');
      }
    }

    return this.keywordClassify(input);
  }

  /** LLM-based classification with structured JSON prompt. */
  private async llmClassify(input: string): Promise<ClassificationResult> {
    const prompt = `You are a memory classifier for an AI assistant. Analyze the following text and determine how it should be stored in memory.

Text to classify:
"${input}"

Respond with ONLY a JSON object with these fields:
{
  "shouldSave": boolean - whether this information is worth saving,
  "type": "${MEMORY_TYPES.join(' | ')}" - the type of memory,
  "importance": number (0.0 to 1.0) - how important this information is,
  "tags": string[] - relevant tags for categorization,
  "reason": string - brief explanation of the classification
}`;

    const response = await this.llmProvider!.complete(prompt, {
      temperature: 0.1,
      maxTokens: 300,
    });

    const cleaned = response.text.trim().replace(/```json\n?/g, '').replace(/```\n?/g, '');
    const parsed = JSON.parse(cleaned) as ClassificationResult;

    if (!MEMORY_TYPES.includes(parsed.type)) {
      logger.warn({ type: parsed.type }, 'Invalid memory type from LLM, defaulting to fact');
      parsed.type = 'fact';
    }

    parsed.importance = Math.max(0, Math.min(1, parsed.importance));
    parsed.shouldSave = Boolean(parsed.shouldSave);

    return parsed;
  }

  /** Keyword-based fallback classification. */
  private keywordClassify(input: string): ClassificationResult {
    const lower = input.toLowerCase();

    // Check against keyword rules
    for (const rule of KEYWORD_RULES) {
      if (rule.keywords.some(kw => lower.includes(kw))) {
        return {
          shouldSave: true,
          type: rule.type,
          importance: rule.importance,
          tags: this.extractTags(input),
          reason: `Keyword match for ${rule.type}`,
        };
      }
    }

    // Default: check if it seems factual/informational
    const isInformational = lower.length > 20 && (
      lower.includes(' is ') ||
      lower.includes(' are ') ||
      lower.includes(' has ') ||
      lower.includes(' can ') ||
      lower.includes(' should ') ||
      lower.includes(' note:') ||
      lower.includes(' remember:')
    );

    if (isInformational) {
      return {
        shouldSave: true,
        type: 'fact',
        importance: 0.5,
        tags: this.extractTags(input),
        reason: 'Detected factual statement',
      };
    }

    return {
      shouldSave: false,
      type: 'fact',
      importance: 0.2,
      tags: [],
      reason: 'Not enough signal to save',
    };
  }

  /** Extract simple tags from text. */
  private extractTags(text: string): string[] {
    const words = text.toLowerCase().split(/\W+/).filter(w => w.length > 3);
    const stopWords = new Set(['that', 'this', 'with', 'from', 'have', 'been', 'were', 'will', 'would', 'could', 'should', 'about', 'which', 'their', 'there', 'these', 'those']);
    return [...new Set(words.filter(w => !stopWords.has(w)))].slice(0, 5);
  }
}
