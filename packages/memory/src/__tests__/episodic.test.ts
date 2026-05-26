import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { EpisodicMemory, setEpisodicDb, type ChatMessage } from '../episodic.js';

function freezeTime(ts: number) {
  vi.setSystemTime(ts);
}

function advanceTime(ms: number) {
  vi.advanceTimersByTime(ms);
}

describe('EpisodicMemory', () => {
  beforeEach(() => {
    setEpisodicDb(null);
    vi.useFakeTimers();
    freezeTime(0);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // --- Constructor ---

  describe('constructor', () => {
    it('should default maxSize to 100 when no argument is provided', () => {
      const memory = new EpisodicMemory();
      expect(memory.capacity).toBe(100);
    });

    it('should accept a custom maxSize', () => {
      const memory = new EpisodicMemory(50);
      expect(memory.capacity).toBe(50);
    });

    it('should throw if maxSize is less than 1', () => {
      expect(() => new EpisodicMemory(0)).toThrow('EpisodicMemory maxSize must be at least 1');
      expect(() => new EpisodicMemory(-5)).toThrow('EpisodicMemory maxSize must be at least 1');
    });

    it('should start with size 0', () => {
      const memory = new EpisodicMemory(10);
      expect(memory.size).toBe(0);
    });
  });

  // --- add ---

  describe('add', () => {
    it('should add a message and increase size', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'Hello');
      expect(memory.size).toBe(1);
    });

    it('should set timestamp on added messages', () => {
      freezeTime(1700000000000);
      const memory = new EpisodicMemory(10);
      memory.add('user', 'Hello');

      const messages = memory.getAll();
      expect(messages[0].timestamp).toBe(1700000000000);
    });

    it('should store optional metadata when provided', () => {
      const memory = new EpisodicMemory(10);
      memory.add('assistant', 'Sure', { confidence: 0.9, source: 'model-a' });

      const messages = memory.getAll();
      expect(messages[0].metadata).toEqual({ confidence: 0.9, source: 'model-a' });
    });

    it('should not include metadata key when metadata is omitted', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'Hello');

      const messages = memory.getAll();
      expect(messages[0].metadata).toBeUndefined();
    });

    it('should evict oldest message when buffer exceeds maxSize', () => {
      const memory = new EpisodicMemory(3);
      memory.add('user', 'msg1');
      memory.add('assistant', 'msg2');
      memory.add('user', 'msg3');
      memory.add('system', 'msg4');

      expect(memory.size).toBe(3);
      const messages = memory.getAll();
      expect(messages[0].content).toBe('msg2');
      expect(messages[1].content).toBe('msg3');
      expect(messages[2].content).toBe('msg4');
    });

    it('should handle maxSize=1 ring buffer correctly', () => {
      const memory = new EpisodicMemory(1);
      memory.add('user', 'first');
      expect(memory.size).toBe(1);
      expect(memory.getAll()[0].content).toBe('first');

      memory.add('assistant', 'second');
      expect(memory.size).toBe(1);
      expect(memory.getAll()[0].content).toBe('second');
    });
  });

  // --- getRecent ---

  describe('getRecent', () => {
    it('should return all messages when called without count', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'msg1');
      memory.add('assistant', 'msg2');
      memory.add('user', 'msg3');

      expect(memory.getRecent()).toHaveLength(3);
    });

    it('should return the last N messages when count is specified', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'msg1');
      memory.add('assistant', 'msg2');
      memory.add('user', 'msg3');

      const recent = memory.getRecent(2);
      expect(recent).toHaveLength(2);
      expect(recent[0].content).toBe('msg2');
      expect(recent[1].content).toBe('msg3');
    });

    it('should return empty array for empty buffer', () => {
      const memory = new EpisodicMemory(10);
      expect(memory.getRecent()).toEqual([]);
      expect(memory.getRecent(5)).toEqual([]);
    });

    it('should return shallow copies (mutation does not affect original)', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'original');

      const recent = memory.getRecent();
      recent[0].content = 'mutated';

      expect(memory.getAll()[0].content).toBe('original');
    });
  });

  // --- getAll ---

  describe('getAll', () => {
    it('should return all messages in insertion order', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'msg1');
      memory.add('assistant', 'msg2');
      memory.add('system', 'msg3');

      const all = memory.getAll();
      expect(all).toHaveLength(3);
      expect(all[0].content).toBe('msg1');
      expect(all[1].content).toBe('msg2');
      expect(all[2].content).toBe('msg3');
    });

    it('should return shallow copies (mutation does not affect original)', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'original');

      const all = memory.getAll();
      all[0].content = 'mutated';
      // Mutate a shallow copy's nested metadata too
      all[0].role = 'system' as const;

      const fresh = memory.getAll();
      expect(fresh[0].content).toBe('original');
      expect(fresh[0].role).toBe('user');
    });

    it('should return empty array for empty buffer', () => {
      const memory = new EpisodicMemory(10);
      expect(memory.getAll()).toEqual([]);
    });
  });

  // --- clear ---

  describe('clear', () => {
    it('should empty the buffer', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'msg1');
      memory.add('assistant', 'msg2');
      expect(memory.size).toBe(2);

      memory.clear();
      expect(memory.size).toBe(0);
      expect(memory.getAll()).toEqual([]);
    });

    it('should be idempotent', () => {
      const memory = new EpisodicMemory(10);
      memory.clear();
      memory.clear();
      expect(memory.size).toBe(0);
    });
  });

  // --- toPromptString ---

  describe('toPromptString', () => {
    it('should return empty string for empty buffer', () => {
      const memory = new EpisodicMemory(10);
      expect(memory.toPromptString()).toBe('');
    });

    it('should format user and assistant messages correctly', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'What is the weather?');
      memory.add('assistant', 'It is sunny today.');

      const result = memory.toPromptString();
      expect(result).toBe(
        '## Recent Conversation\n**User:** What is the weather?\n**Assistant:** It is sunny today.',
      );
    });

    it('should format system messages as System', () => {
      const memory = new EpisodicMemory(10);
      memory.add('system', 'You are a helpful assistant.');

      const result = memory.toPromptString();
      expect(result).toBe(
        '## Recent Conversation\n**System:** You are a helpful assistant.',
      );
    });

    it('should respect the count argument', () => {
      const memory = new EpisodicMemory(10);
      memory.add('user', 'msg1');
      memory.add('user', 'msg2');
      memory.add('user', 'msg3');

      const result = memory.toPromptString(2);
      expect(result).toBe(
        '## Recent Conversation\n**User:** msg2\n**User:** msg3',
      );
    });
  });

  // --- size / capacity getters ---

  describe('size and capacity', () => {
    it('size should reflect the number of messages', () => {
      const memory = new EpisodicMemory(5);
      expect(memory.size).toBe(0);
      memory.add('user', 'a');
      expect(memory.size).toBe(1);
      memory.add('assistant', 'b');
      expect(memory.size).toBe(2);
    });

    it('capacity should return the configured maxSize', () => {
      const memory = new EpisodicMemory(42);
      expect(memory.capacity).toBe(42);
    });

    it('size should never exceed capacity', () => {
      const memory = new EpisodicMemory(3);
      memory.add('user', 'a');
      memory.add('user', 'b');
      memory.add('user', 'c');
      memory.add('user', 'd');
      memory.add('user', 'e');
      expect(memory.size).toBe(3);
      expect(memory.size).toBeLessThanOrEqual(memory.capacity);
    });
  });
});
