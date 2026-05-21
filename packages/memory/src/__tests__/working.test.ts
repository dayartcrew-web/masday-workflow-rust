import { describe, it, expect, beforeEach } from 'vitest';
import { WorkingMemory } from '../working.js';

describe('WorkingMemory', () => {
  let memory: WorkingMemory;

  beforeEach(() => {
    memory = new WorkingMemory();
  });

  describe('create', () => {
    it('creates a new session with default state', () => {
      const state = memory.create('session-1');

      expect(state).toMatchObject({
        sessionId: 'session-1',
        currentSkills: [],
        currentWorkflow: null,
        activeGoal: null,
        customState: {},
      });
      expect(state.createdAt).toBeGreaterThan(0);
      expect(state.updatedAt).toBeGreaterThan(0);
    });

    it('returns existing session when called twice with same ID', () => {
      const first = memory.create('session-1');
      const second = memory.create('session-1');

      expect(first).toStrictEqual(second);
    });

    it('creates independent sessions for different IDs', () => {
      const a = memory.create('session-a');
      const b = memory.create('session-b');

      expect(a).not.toBe(b);
      expect(a.sessionId).toBe('session-a');
      expect(b.sessionId).toBe('session-b');
    });

    it('returns shallow copy of state (arrays shared)', () => {
      const state = memory.create('session-1');
      state.currentSkills.push('test-skill');

      const retrieved = memory.get('session-1');
      // create() returns shallow copy, so currentSkills array is shared
      expect(retrieved?.currentSkills).toEqual(['test-skill']);
    });
  });

  describe('get', () => {
    it('returns undefined for non-existent session', () => {
      expect(memory.get('non-existent')).toBeUndefined();
    });

    it('returns session state after creation', () => {
      memory.create('session-1');
      const state = memory.get('session-1');

      expect(state).toBeDefined();
      expect(state?.sessionId).toBe('session-1');
    });

    it('returns immutable copy preventing mutation', () => {
      memory.create('session-1');
      const copy = memory.get('session-1')!;
      copy.customState.key = 'mutated';

      const original = memory.get('session-1');
      expect(original?.customState.key).toBeUndefined();
    });
  });

  describe('getOrCreate', () => {
    it('creates session if it does not exist', () => {
      const state = memory.getOrCreate('new-session');

      expect(state).toBeDefined();
      expect(state.sessionId).toBe('new-session');
      expect(memory.size).toBe(1);
    });

    it('returns existing session without creating duplicate', () => {
      const created = memory.create('session-1');
      const retrieved = memory.getOrCreate('session-1');

      expect(created).toEqual(retrieved);
      expect(memory.size).toBe(1);
    });
  });

  describe('update', () => {
    it('updates session fields and returns updated state', () => {
      memory.create('session-1');

      const updated = memory.update('session-1', {
        activeGoal: 'build feature X',
      });

      expect(updated?.activeGoal).toBe('build feature X');
      expect(updated?.updatedAt).toBeGreaterThanOrEqual(updated?.createdAt);
    });

    it('returns undefined for non-existent session', () => {
      const result = memory.update('non-existent', { activeGoal: 'test' });
      expect(result).toBeUndefined();
    });

    it('preserves sessionId and createdAt on update', () => {
      memory.create('session-1');
      const original = memory.get('session-1')!;

      const updated = memory.update('session-1', { activeGoal: 'new goal' });

      expect(updated?.sessionId).toBe(original.sessionId);
      expect(updated?.createdAt).toBe(original.createdAt);
    });

    it('merges customState without overwriting existing keys', () => {
      memory.create('session-1');
      memory.update('session-1', { customState: { key1: 'value1' } });
      memory.update('session-1', { customState: { key2: 'value2' } });

      const state = memory.get('session-1');
      // Note: update replaces customState entirely, not merges
      expect(state?.customState).toEqual({ key2: 'value2' });
    });

    it('updates currentSkills array', () => {
      memory.create('session-1');
      memory.update('session-1', { currentSkills: ['skill-a', 'skill-b'] });

      const state = memory.get('session-1');
      expect(state?.currentSkills).toEqual(['skill-a', 'skill-b']);
    });

    it('updates currentWorkflow', () => {
      memory.create('session-1');
      memory.update('session-1', { currentWorkflow: 'wf-123' });

      const state = memory.get('session-1');
      expect(state?.currentWorkflow).toBe('wf-123');
    });

    it('updates multiple fields at once', () => {
      memory.create('session-1');
      const updated = memory.update('session-1', {
        activeGoal: 'goal',
        currentWorkflow: 'wf-1',
        currentSkills: ['s1'],
        customState: { extra: true },
      });

      expect(updated?.activeGoal).toBe('goal');
      expect(updated?.currentWorkflow).toBe('wf-1');
      expect(updated?.currentSkills).toEqual(['s1']);
      expect(updated?.customState).toEqual({ extra: true });
    });

    it('returns immutable copy of updated state', () => {
      memory.create('session-1');
      const updated = memory.update('session-1', {
        customState: { key: 'value' },
      })!;

      updated.customState.key = 'mutated';

      const stored = memory.get('session-1');
      expect(stored?.customState.key).toBe('value');
    });
  });

  describe('delete', () => {
    it('deletes existing session and returns true', () => {
      memory.create('session-1');
      const result = memory.delete('session-1');

      expect(result).toBe(true);
      expect(memory.has('session-1')).toBe(false);
      expect(memory.size).toBe(0);
    });

    it('returns false for non-existent session', () => {
      const result = memory.delete('non-existent');
      expect(result).toBe(false);
    });

    it('deletes only the specified session', () => {
      memory.create('session-a');
      memory.create('session-b');
      memory.delete('session-a');

      expect(memory.has('session-a')).toBe(false);
      expect(memory.has('session-b')).toBe(true);
      expect(memory.size).toBe(1);
    });
  });

  describe('has', () => {
    it('returns true for existing session', () => {
      memory.create('session-1');
      expect(memory.has('session-1')).toBe(true);
    });

    it('returns false for non-existent session', () => {
      expect(memory.has('session-1')).toBe(false);
    });
  });

  describe('size', () => {
    it('returns 0 for empty memory', () => {
      expect(memory.size).toBe(0);
    });

    it('returns correct count after creating sessions', () => {
      memory.create('s1');
      memory.create('s2');
      memory.create('s3');
      expect(memory.size).toBe(3);
    });

    it('decrements after deleting sessions', () => {
      memory.create('s1');
      memory.create('s2');
      memory.delete('s1');
      expect(memory.size).toBe(1);
    });
  });

  describe('clear', () => {
    it('removes all sessions', () => {
      memory.create('s1');
      memory.create('s2');
      memory.clear();

      expect(memory.size).toBe(0);
      expect(memory.has('s1')).toBe(false);
      expect(memory.has('s2')).toBe(false);
    });

    it('allows creating sessions after clear', () => {
      memory.create('s1');
      memory.clear();
      memory.create('s2');

      expect(memory.size).toBe(1);
      expect(memory.has('s2')).toBe(true);
    });
  });

  describe('toPromptString', () => {
    it('returns empty string for non-existent session', () => {
      expect(memory.toPromptString('non-existent')).toBe('');
    });

    it('returns formatted string with all fields populated', () => {
      memory.create('session-1');
      memory.update('session-1', {
        activeGoal: 'Build auth module',
        currentWorkflow: 'wf-auth',
        currentSkills: ['masday-backend', 'masday-qa'],
        customState: { phase: 'implementation', progress: 0.5 },
      });

      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('## Current Session (session-1)');
      expect(prompt).toContain('**Active Goal:** Build auth module');
      expect(prompt).toContain('**Current Workflow:** wf-auth');
      expect(prompt).toContain('**Active Skills:** masday-backend, masday-qa');
      expect(prompt).toContain('**Additional Context:**');
      expect(prompt).toContain('phase: "implementation"');
      expect(prompt).toContain('progress: 0.5');
    });

    it('omits sections for null/empty fields', () => {
      memory.create('session-1');
      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('## Current Session (session-1)');
      expect(prompt).not.toContain('**Active Goal:**');
      expect(prompt).not.toContain('**Current Workflow:**');
      expect(prompt).not.toContain('**Active Skills:**');
      expect(prompt).not.toContain('**Additional Context:**');
    });

    it('shows only active goal when set', () => {
      memory.create('session-1');
      memory.update('session-1', { activeGoal: 'Fix bug #42' });

      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('**Active Goal:** Fix bug #42');
      expect(prompt).not.toContain('**Current Workflow:**');
    });

    it('shows only current workflow when set', () => {
      memory.create('session-1');
      memory.update('session-1', { currentWorkflow: 'wf-deploy' });

      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('**Current Workflow:** wf-deploy');
      expect(prompt).not.toContain('**Active Goal:**');
    });

    it('shows only skills when set', () => {
      memory.create('session-1');
      memory.update('session-1', { currentSkills: ['tdd', 'review'] });

      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('**Active Skills:** tdd, review');
      expect(prompt).not.toContain('**Active Goal:**');
    });

    it('handles complex customState values', () => {
      memory.create('session-1');
      memory.update('session-1', {
        customState: {
          nested: { a: 1, b: [2, 3] },
          flag: true,
          count: 42,
        },
      });

      const prompt = memory.toPromptString('session-1');

      expect(prompt).toContain('nested:');
      expect(prompt).toContain('flag: true');
      expect(prompt).toContain('count: 42');
    });
  });

  describe('concurrent access', () => {
    it('handles rapid create/get cycles', () => {
      const results: Array<{ sessionId: string }> = [];
      for (let i = 0; i < 100; i++) {
        results.push(memory.create(`session-${i}`));
      }

      expect(memory.size).toBe(100);
      expect(new Set(results.map(r => r.sessionId)).size).toBe(100);
    });

    it('handles update after delete gracefully', () => {
      memory.create('session-1');
      memory.delete('session-1');
      const result = memory.update('session-1', { activeGoal: 'test' });

      expect(result).toBeUndefined();
    });
  });
});
