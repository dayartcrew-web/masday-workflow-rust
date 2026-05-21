import { describe, it, expect, vi } from 'vitest';
import { ScoringEngine, DEFAULT_WEIGHTS } from '../scoring.js';
import { MemoryClassifier } from '../classifier.js';
import type { MemoryRecord, MemorySearchResult, MemoryType } from '@mcp-rebuild/core';

function makeMemory(overrides: Partial<MemoryRecord> = {}): MemoryRecord {
  return {
    id: 'mem-1',
    content: 'Test content about testing frameworks',
    type: 'fact',
    importance: 0.5,
    tags: [],
    source: 'user',
    version: 1,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    accessedAt: Date.now(),
    accessCount: 0,
    ...overrides,
  };
}

function makeSearchResult(memory: MemoryRecord, score: number): MemorySearchResult {
  return { memory, score };
}

// ============================================================
// ScoringEngine tests
// ============================================================
describe('ScoringEngine', () => {
  describe('DEFAULT_WEIGHTS', () => {
    it('has expected default weight values', () => {
      expect(DEFAULT_WEIGHTS.similarity).toBe(0.6);
      expect(DEFAULT_WEIGHTS.recency).toBe(0.15);
      expect(DEFAULT_WEIGHTS.importance).toBe(0.15);
      expect(DEFAULT_WEIGHTS.usage).toBe(0.1);
    });
  });

  describe('constructor', () => {
    it('uses default weights when none provided', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory({ importance: 0.5, accessCount: 0, createdAt: 0 });
      const result = engine.score(memory, 1.0);

      // With age so large recency ~ 0, usage = 0
      // score = 1.0 * 0.6 + 0 * 0.15 + 0.5 * 0.15 + 0 * 0.1 = 0.6 + 0 + 0.075 + 0 = 0.675
      const expected = 1.0 * 0.6 + 0 + 0.5 * 0.15 + 0;
      expect(result.finalScore).toBeCloseTo(expected, 5);
    });

    it('accepts custom weights override', () => {
      const engine = new ScoringEngine({ similarity: 0.4, importance: 0.4, recency: 0.1, usage: 0.1 });
      const memory = makeMemory({ importance: 1.0, accessCount: 0, createdAt: Date.now() });
      const result = engine.score(memory, 0.5);

      // recency = 1.0 (brand new), usage = 0
      // score = 0.5 * 0.4 + 1.0 * 0.1 + 1.0 * 0.4 + 0 * 0.1 = 0.2 + 0.1 + 0.4 = 0.7
      const expected = 0.5 * 0.4 + 1.0 * 0.1 + 1.0 * 0.4 + 0;
      expect(result.finalScore).toBeCloseTo(expected, 5);
    });

    it('accepts custom half-life', () => {
      const engine = new ScoringEngine(undefined, 1); // 1 day half-life
      const oneDayMs = 24 * 60 * 60 * 1000;
      const memory = makeMemory({ createdAt: Date.now() - oneDayMs });
      const recency = engine.computeRecency(memory);
      expect(recency).toBeCloseTo(0.5, 5);
    });
  });

  describe('score()', () => {
    it('computes finalScore matching the weighted formula', () => {
      const engine = new ScoringEngine();
      const past = Date.now() - 3 * 24 * 60 * 60 * 1000; // 3 days ago
      const memory = makeMemory({
        importance: 0.9,
        accessCount: 50,
        createdAt: past,
      });

      const result = engine.score(memory, 0.7);

      // Verify component calculations
      const expectedRecency = Math.pow(0.5, (Date.now() - past) / engine['halfLifeMs']);
      const expectedUsage = Math.log(1 + 50) / Math.log(1 + 100);

      expect(result.recency).toBe(expectedRecency);
      expect(result.usage).toBe(expectedUsage);
      expect(result.finalScore).toBe(
        0.7 * 0.6 + expectedRecency * 0.15 + 0.9 * 0.15 + expectedUsage * 0.1,
      );
    });

    it('uses default similarityScore of 0.5 when not provided', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory({ importance: 0.5, createdAt: Date.now() });
      const result = engine.score(memory);
      // recency ~ 1.0, usage = 0
      // score = 0.5 * 0.6 + 1.0 * 0.15 + 0.5 * 0.15 + 0 = 0.3 + 0.15 + 0.075 = 0.525
      expect(result.finalScore).toBeCloseTo(0.525, 3);
    });

    it('returns the ScoredMemory with correct shape', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory();
      const result = engine.score(memory);

      expect(result).toHaveProperty('memory');
      expect(result).toHaveProperty('recency');
      expect(result).toHaveProperty('usage');
      expect(result).toHaveProperty('finalScore');
      expect(typeof result.finalScore).toBe('number');
    });
  });

  describe('rerank()', () => {
    it('sorts results by finalScore descending', () => {
      const engine = new ScoringEngine();

      const mem1 = makeMemory({ id: 'a', importance: 0.3, createdAt: 0, accessCount: 0, content: 'alpha' });
      const mem2 = makeMemory({ id: 'b', importance: 0.5, createdAt: 0, accessCount: 0, content: 'beta' });
      const mem3 = makeMemory({ id: 'c', importance: 0.9, createdAt: 0, accessCount: 0, content: 'gamma' });

      const results: MemorySearchResult[] = [
        makeSearchResult(mem1, 0.2),
        makeSearchResult(mem2, 0.2),
        makeSearchResult(mem3, 0.2),
      ];

      const ranked = engine.rerank(results);

      expect(ranked).toHaveLength(3);
      expect(ranked[0].memory.id).toBe('c'); // highest importance
      expect(ranked[1].memory.id).toBe('b');
      expect(ranked[2].memory.id).toBe('a'); // lowest importance
      expect(ranked[0].finalScore).toBeGreaterThanOrEqual(ranked[1].finalScore);
      expect(ranked[1].finalScore).toBeGreaterThanOrEqual(ranked[2].finalScore);
    });

    it('includes similarity scores in final score ordering', () => {
      const engine = new ScoringEngine();

      const mem1 = makeMemory({ id: 'x', importance: 0.5, createdAt: 0, accessCount: 0 });
      const mem2 = makeMemory({ id: 'y', importance: 0.5, createdAt: 0, accessCount: 0 });

      const results: MemorySearchResult[] = [
        makeSearchResult(mem1, 0.9), // high similarity
        makeSearchResult(mem2, 0.1), // low similarity
      ];

      const ranked = engine.rerank(results);

      expect(ranked[0].memory.id).toBe('x'); // same importance but higher similarity wins
    });

    it('returns empty array for empty input', () => {
      const engine = new ScoringEngine();
      const ranked = engine.rerank([]);
      expect(ranked).toEqual([]);
    });
  });

  describe('cosineSimilarity()', () => {
    it('returns 1.0 for identical vectors', () => {
      const v = [1, 2, 3];
      expect(ScoringEngine.cosineSimilarity(v, v)).toBeCloseTo(1.0, 10);
    });

    it('returns -1.0 for opposite vectors', () => {
      expect(ScoringEngine.cosineSimilarity([1, 0], [-1, 0])).toBeCloseTo(-1.0, 10);
    });

    it('returns 0 for orthogonal vectors', () => {
      expect(ScoringEngine.cosineSimilarity([1, 0], [0, 1])).toBeCloseTo(0, 10);
    });

    it('returns 0 for zero-length vectors', () => {
      expect(ScoringEngine.cosineSimilarity([0, 0], [1, 2])).toBe(0);
    });

    it('returns 0 for empty vectors', () => {
      expect(ScoringEngine.cosineSimilarity([], [1, 2])).toBe(0);
    });

    it('returns 0 for unequal length vectors', () => {
      expect(ScoringEngine.cosineSimilarity([1, 2], [1, 2, 3])).toBe(0);
    });

    it('handles single-element vectors', () => {
      expect(ScoringEngine.cosineSimilarity([3], [3])).toBeCloseTo(1.0, 10);
      expect(ScoringEngine.cosineSimilarity([5], [-5])).toBeCloseTo(-1.0, 10);
    });
  });

  describe('jaccardSimilarity()', () => {
    it('returns 1.0 for identical strings', () => {
      expect(ScoringEngine.jaccardSimilarity('hello world', 'hello world')).toBeCloseTo(1.0, 10);
    });

    it('returns 0 for strings with no token overlap', () => {
      expect(ScoringEngine.jaccardSimilarity('alpha beta gamma', 'delta epsilon zeta')).toBe(0);
    });

    it('returns ~0.5 for strings with partial overlap', () => {
      const result = ScoringEngine.jaccardSimilarity('hello world testing', 'hello world coding');
      expect(result).toBeGreaterThan(0);
      expect(result).toBeLessThan(1);
    });

    it('filters out short tokens (length <= 2)', () => {
      // "is" and "a" are too short, only "test" remains in both
      const result = ScoringEngine.jaccardSimilarity('is a test', 'is a test');
      expect(result).toBeCloseTo(1.0, 10); // only "test" in both
    });

    it('returns 0 when both token sets are empty', () => {
      expect(ScoringEngine.jaccardSimilarity('is a', 'an it')).toBe(0);
    });

    it('is case-insensitive', () => {
      const result = ScoringEngine.jaccardSimilarity('Hello World', 'hello world');
      expect(result).toBeCloseTo(1.0, 10);
    });
  });

  describe('computeRecency()', () => {
    it('returns 1.0 for brand new memory (createdAt in future)', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory({ createdAt: Date.now() + 10000 });
      expect(engine.computeRecency(memory)).toBe(1.0);
    });

    it('returns ~0.5 at half-life (default 7 days)', () => {
      const engine = new ScoringEngine();
      const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
      const memory = makeMemory({ createdAt: Date.now() - sevenDaysMs });
      expect(engine.computeRecency(memory)).toBeCloseTo(0.5, 3);
    });

    it('returns ~0.25 at twice the half-life', () => {
      const engine = new ScoringEngine(undefined, 7);
      const fourteenDaysMs = 14 * 24 * 60 * 60 * 1000;
      const memory = makeMemory({ createdAt: Date.now() - fourteenDaysMs });
      expect(engine.computeRecency(memory)).toBeCloseTo(0.25, 2);
    });

    it('approaches 0 for very old memories', () => {
      const engine = new ScoringEngine(undefined, 1); // 1 day half-life
      const memory = makeMemory({ createdAt: Date.now() - 30 * 24 * 60 * 60 * 1000 }); // 30 days
      expect(engine.computeRecency(memory)).toBeCloseTo(0, 5);
    });
  });

  describe('computeUsage()', () => {
    it('returns ~0 for accessCount of 0', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory({ accessCount: 0 });
      expect(engine.computeUsage(memory)).toBeCloseTo(0, 5);
    });

    it('returns 1.0 for accessCount of 100 (reference max)', () => {
      const engine = new ScoringEngine();
      const memory = makeMemory({ accessCount: 100 });
      expect(engine.computeUsage(memory)).toBeCloseTo(1.0, 5);
    });

    it('applies log(1 + x) / log(101) formula', () => {
      const engine = new ScoringEngine();
      expect(engine.computeUsage(makeMemory({ accessCount: 1 })))
        .toBeCloseTo(Math.log(2) / Math.log(101));
      expect(engine.computeUsage(makeMemory({ accessCount: 10 })))
        .toBeCloseTo(Math.log(11) / Math.log(101));
      expect(engine.computeUsage(makeMemory({ accessCount: 100 })))
        .toBeCloseTo(1.0);
    });
  });

  describe('computeSimilarity()', () => {
    it('uses cosineSimilarity when both memories have embeddings of same length', () => {
      const a = makeMemory({ embedding: [1, 2, 3] });
      const b = makeMemory({ embedding: [1, 2, 3] });
      expect(ScoringEngine.computeSimilarity(a, b)).toBeCloseTo(1.0, 10);
    });

    it('falls back to jaccard when embeddings are missing', () => {
      const a = makeMemory({ embedding: undefined, content: 'hello world testing' });
      const b = makeMemory({ embedding: undefined, content: 'hello world coding' });
      const result = ScoringEngine.computeSimilarity(a, b);
      expect(result).toBeGreaterThan(0);
      expect(result).toBeLessThan(1);
    });

    it('falls back to jaccard when embedding lengths differ', () => {
      const a = makeMemory({ embedding: [1, 2, 3] });
      const b = makeMemory({ embedding: [1, 2] });
      const result = ScoringEngine.computeSimilarity(a, b);
      expect(result).toBeGreaterThanOrEqual(0);
    });

    it('falls back to jaccard when embeddings are empty arrays', () => {
      const a = makeMemory({ embedding: [] });
      const b = makeMemory({ embedding: [] });
      const result = ScoringEngine.computeSimilarity(a, b);
      expect(result).toBeGreaterThanOrEqual(0);
    });
  });
});

// ============================================================
// Classifier tests
// ============================================================
describe('MemoryClassifier', () => {
  describe('keywordClassify (via classify without LLM)', () => {
    it('detects blocker keywords and assigns importance 0.9', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('I found a bug in the memory system');
      expect(result.type).toBe('blocker');
      expect(result.importance).toBe(0.9);
      expect(result.shouldSave).toBe(true);
    });

    it('detects "error" keyword as blocker with importance 0.9', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('There is an error in the deployment pipeline');
      expect(result.type).toBe('blocker');
      expect(result.importance).toBe(0.9);
    });

    it('detects "stuck" keyword as blocker', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('I am stuck on this implementation');
      expect(result.type).toBe('blocker');
      expect(result.importance).toBe(0.9);
    });

    it('detects preference keywords with importance 0.8', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('I prefer using TypeScript over JavaScript');
      expect(result.type).toBe('preference');
      expect(result.importance).toBe(0.8);
    });

    it('detects "favorite" as preference', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('My favorite framework is Next.js');
      expect(result.type).toBe('preference');
      expect(result.importance).toBe(0.8);
    });

    it('detects decision keywords with importance 0.8', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('We decided to use PostgreSQL');
      expect(result.type).toBe('decision');
      expect(result.importance).toBe(0.8);
    });

    it('detects learning keywords with importance 0.7', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('I learned that Vitest mocking works well');
      expect(result.type).toBe('learning');
      expect(result.importance).toBe(0.7);
    });

    it('detects "insight" as learning', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Key insight: caching reduces latency by 10x');
      expect(result.type).toBe('learning');
      expect(result.importance).toBe(0.7);
    });

    it('detects strategy keywords with importance 0.7', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Our strategy is to build incrementally');
      expect(result.type).toBe('strategy');
      expect(result.importance).toBe(0.7);
    });

    it('detects "workflow" as strategy', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The workflow should include code review');
      expect(result.type).toBe('strategy');
      expect(result.importance).toBe(0.7);
    });

    it('detects skill keywords with importance 0.6', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('This tool helps with code generation');
      expect(result.type).toBe('skill');
      expect(result.importance).toBe(0.6);
    });

    it('detects "framework" as skill', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('React is a popular framework');
      expect(result.type).toBe('skill');
      expect(result.importance).toBe(0.6);
    });

    it('detects artifact keywords with importance 0.5', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The component file is in the src directory');
      expect(result.type).toBe('artifact');
      expect(result.importance).toBe(0.5);
    });

    it('defaults to fact with importance 0.5 for informational text', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The project is using TypeScript strict mode');
      expect(result.type).toBe('fact');
      expect(result.importance).toBe(0.5);
      expect(result.shouldSave).toBe(true);
    });

    it('defaults to shouldSave=false with importance 0.2 for non-informational text', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('ok');
      expect(result.shouldSave).toBe(false);
      expect(result.importance).toBe(0.2);
      expect(result.type).toBe('fact');
      expect(result.tags).toEqual([]);
    });

    it('detects first keyword match (preference over artifact for "I always use that library")', async () => {
      const classifier = new MemoryClassifier();
      // "always use" is a preference keyword; "library" is a skill keyword
      // preference comes first in KEYWORD_RULES array
      const result = await classifier.classify('I always use that library');
      expect(result.type).toBe('preference');
      expect(result.importance).toBe(0.8);
    });

    it('extracts tags from classified text', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('We decided to use PostgreSQL for the database');
      expect(result.tags.length).toBeGreaterThan(0);
      expect(result.tags).toContain('postgresql');
      expect(result.tags).toContain('database');
      expect(result.tags.length).toBeLessThanOrEqual(5);
    });

    it('filters stop words from tags', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('This is a note: remember that testing is important');
      // 'this' and 'that' are stop words, should not appear
      expect(result.tags).not.toContain('this');
      expect(result.tags).not.toContain('that');
      expect(result.tags).toContain('testing');
      expect(result.tags).toContain('important');
    });
  });

  describe('LLM integration', () => {
    it('uses LLM provider when available', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'strategy',
            importance: 0.85,
            tags: ['architecture', 'design'],
            reason: 'Architectural design decision',
          }),
        }),
      };

      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('We should use microservices architecture');
      expect(result.type).toBe('strategy');
      expect(result.importance).toBe(0.85);
      expect(result.shouldSave).toBe(true);
      expect(result.tags).toEqual(['architecture', 'design']);
      expect(mockLLM.complete).toHaveBeenCalledOnce();
    });

    it('falls back to keyword classification on LLM error', async () => {
      const mockLLM = {
        complete: vi.fn().mockRejectedValue(new Error('LLM unavailable')),
      };

      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('I prefer using Rust');
      // Should fallback to keyword match → preference
      expect(result.type).toBe('preference');
      expect(result.importance).toBe(0.8);
      expect(result.shouldSave).toBe(true);
    });

    it('clamps importance to 0-1 range from LLM', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'fact',
            importance: 2.5, // out of range
            tags: [],
            reason: 'test',
          }),
        }),
      };

      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('some text for testing');
      expect(result.importance).toBe(1.0);
    });

    it('clamps negative importance from LLM to 0', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'fact',
            importance: -0.5,
            tags: [],
            reason: 'test',
          }),
        }),
      };

      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('some text');
      expect(result.importance).toBe(0);
    });

    it('defaults invalid LLM memory type to fact', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'invalid_type',
            importance: 0.5,
            tags: [],
            reason: 'test',
          }),
        }),
      };

      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('any text');
      expect(result.type).toBe('fact');
    });
  });

  describe('constructor', () => {
    it('creates classifier without LLM provider', () => {
      const classifier = new MemoryClassifier();
      expect(classifier).toBeInstanceOf(MemoryClassifier);
    });

    it('creates classifier with LLM provider', () => {
      const mockLLM = {
        complete: vi.fn(),
      };
      const classifier = new MemoryClassifier(mockLLM);
      expect(classifier).toBeInstanceOf(MemoryClassifier);
    });
  });
});
