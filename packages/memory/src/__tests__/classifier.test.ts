import { describe, it, expect, vi } from 'vitest';
import { MemoryClassifier } from '../../src/classifier';

describe('MemoryClassifier', () => {
  describe('keyword-based classification (no LLM)', () => {
    it('classifies blocker keywords with importance 0.9', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Found a bug in the auth module');
      expect(result.type).toBe('blocker');
      expect(result.importance).toBe(0.9);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies preference keywords with importance 0.8', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('I prefer using TypeScript for all new modules');
      expect(result.type).toBe('preference');
      expect(result.importance).toBe(0.8);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies decision keywords with importance 0.8', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('We decided to use PostgreSQL for the database');
      expect(result.type).toBe('decision');
      expect(result.importance).toBe(0.8);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies learning keywords with importance 0.7', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Learned that the API rate limit is 1000 requests per minute');
      expect(result.type).toBe('learning');
      expect(result.importance).toBe(0.7);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies strategy keywords with importance 0.7', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Our approach is to use microservices for scalability');
      expect(result.type).toBe('strategy');
      expect(result.importance).toBe(0.7);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies skill keywords with importance 0.6', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The tool uses a library pattern for extensibility');
      expect(result.type).toBe('skill');
      expect(result.importance).toBe(0.6);
      expect(result.shouldSave).toBe(true);
    });

    it('classifies artifact keywords with importance 0.5', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Created a new component for the dashboard');
      expect(result.type).toBe('artifact');
      expect(result.importance).toBe(0.5);
      expect(result.shouldSave).toBe(true);
    });

    it('defaults to fact with importance 0.5 for informational text', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The system is running on port 3000 and has 4 endpoints');
      expect(result.type).toBe('fact');
      expect(result.importance).toBe(0.5);
      expect(result.shouldSave).toBe(true);
    });

    it('returns shouldSave=false for non-informational short text', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('ok');
      expect(result.shouldSave).toBe(false);
      expect(result.importance).toBe(0.2);
    });

    it('first keyword match wins (priority order)', async () => {
      const classifier = new MemoryClassifier();
      // KEYWORD_RULES order: preference, learning, decision, blocker, skill, strategy, artifact
      // "decided" matches decision (3rd), "error" matches blocker (4th)
      // decision comes first in the rules array
      const result = await classifier.classify('Found an error but we decided to ignore it');
      expect(result.type).toBe('decision');
      expect(result.importance).toBe(0.8);
    });

    it('extracts tags with stop words filtered', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('The authentication module uses JWT tokens for security');
      expect(result.tags.length).toBeLessThanOrEqual(5);
      expect(result.tags).not.toContain('that');
      expect(result.tags).not.toContain('this');
      expect(result.tags).not.toContain('with');
    });

    it('limits tags to maximum 5', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('authentication module implements JWT token validation with middleware and express routes for security');
      expect(result.tags.length).toBeLessThanOrEqual(5);
    });

    it('case-insensitive keyword matching', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('Found a BUG in the AUTH module');
      expect(result.type).toBe('blocker');
      expect(result.importance).toBe(0.9);
    });

    it('includes reason in classification result', async () => {
      const classifier = new MemoryClassifier();
      const result = await classifier.classify('We decided to use PostgreSQL');
      expect(result.reason).toContain('decision');
    });
  });

  describe('LLM-based classification', () => {
    it('uses LLM provider when available', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'decision',
            importance: 0.85,
            tags: ['database', 'postgresql'],
            reason: 'Strategic decision about infrastructure',
          }),
        }),
      };
      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('We will use PostgreSQL for our database');

      expect(mockLLM.complete).toHaveBeenCalled();
      expect(result.type).toBe('decision');
      expect(result.importance).toBe(0.85);
    });

    it('falls back to keyword when LLM returns invalid type', async () => {
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
      const result = await classifier.classify('test input');

      expect(result.type).toBe('fact');
    });

    it('clamps LLM importance to [0, 1] range', async () => {
      const mockLLM = {
        complete: vi.fn().mockResolvedValue({
          text: JSON.stringify({
            shouldSave: true,
            type: 'fact',
            importance: 1.5,
            tags: [],
            reason: 'test',
          }),
        }),
      };
      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('test');
      expect(result.importance).toBe(1.0);
    });

    it('clamps negative LLM importance to 0', async () => {
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
      const result = await classifier.classify('test');
      expect(result.importance).toBe(0);
    });

    it('falls back to keywords when LLM throws', async () => {
      const mockLLM = {
        complete: vi.fn().mockRejectedValue(new Error('Network error')),
      };
      const classifier = new MemoryClassifier(mockLLM);
      const result = await classifier.classify('Found a bug in the system');
      expect(result.type).toBe('blocker');
    });
  });

  describe('constructor', () => {
    it('creates classifier without LLM provider', () => {
      const classifier = new MemoryClassifier();
      expect(classifier).toBeDefined();
    });

    it('creates classifier with LLM provider', () => {
      const mockLLM = { complete: vi.fn() };
      const classifier = new MemoryClassifier(mockLLM);
      expect(classifier).toBeDefined();
    });
  });
});
