import { describe, it, expect, beforeEach } from 'vitest';
import { TaskManager } from './task.js';

describe('TaskManager', () => {
  let tm: TaskManager;

  beforeEach(() => {
    tm = new TaskManager();
  });

  describe('create', () => {
    it('creates a task with generated id and pending state', () => {
      const task = tm.create({
        name: 'Read file',
        agent: 'backend',
        skill: 'filesystem.read',
        dependencies: [],
        input: { path: '/test.txt' },
      });

      expect(task.id).toMatch(/^task_/);
      expect(task.state).toBe('pending');
      expect(task.name).toBe('Read file');
      expect(task.createdAt).toBeInstanceOf(Date);
    });

    it('creates tasks with unique ids', () => {
      const t1 = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const t2 = tm.create({ name: 't2', agent: 'a', skill: 's', dependencies: [], input: {} });
      expect(t1.id).not.toBe(t2.id);
    });
  });

  describe('get', () => {
    it('returns task by id', () => {
      const created = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const found = tm.get(created.id);
      expect(found).toBeDefined();
      expect(found!.name).toBe('t1');
    });

    it('returns undefined for unknown id', () => {
      expect(tm.get('nonexistent')).toBeUndefined();
    });
  });

  describe('getAll', () => {
    it('returns all tasks', () => {
      tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      tm.create({ name: 't2', agent: 'a', skill: 's', dependencies: [], input: {} });
      expect(tm.getAll()).toHaveLength(2);
    });

    it('returns empty array when no tasks', () => {
      expect(tm.getAll()).toEqual([]);
    });
  });

  describe('updateState', () => {
    it('updates task state to running', () => {
      const task = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const updated = tm.updateState(task.id, 'running');
      expect(updated!.state).toBe('running');
      expect(updated!.startedAt).toBeInstanceOf(Date);
    });

    it('updates task state to done', () => {
      const task = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const updated = tm.updateState(task.id, 'done');
      expect(updated!.state).toBe('done');
      expect(updated!.completedAt).toBeInstanceOf(Date);
    });

    it('updates task state to failed', () => {
      const task = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const updated = tm.updateState(task.id, 'failed');
      expect(updated!.state).toBe('failed');
      expect(updated!.completedAt).toBeInstanceOf(Date);
    });

    it('returns undefined for unknown id', () => {
      expect(tm.updateState('unknown', 'running')).toBeUndefined();
    });
  });

  describe('setOutput', () => {
    it('sets output on a task', () => {
      const task = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      tm.setOutput(task.id, { content: 'hello' });
      expect(tm.get(task.id)!.output).toEqual({ content: 'hello' });
    });

    it('returns undefined for unknown id', () => {
      expect(tm.setOutput('unknown', {})).toBeUndefined();
    });
  });

  describe('setError', () => {
    it('sets error on a task', () => {
      const task = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      tm.setError(task.id, 'Something broke');
      expect(tm.get(task.id)!.error).toBe('Something broke');
    });

    it('returns undefined for unknown id', () => {
      expect(tm.setError('unknown', 'err')).toBeUndefined();
    });
  });

  describe('getPendingTasks', () => {
    it('returns only pending tasks', () => {
      const t1 = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const t2 = tm.create({ name: 't2', agent: 'a', skill: 's', dependencies: [], input: {} });
      tm.updateState(t1.id, 'running');
      expect(tm.getPendingTasks()).toHaveLength(1);
      expect(tm.getPendingTasks()[0].id).toBe(t2.id);
    });
  });

  describe('getReadyTasks', () => {
    it('returns tasks with all dependencies done', () => {
      const t1 = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      const t2 = tm.create({ name: 't2', agent: 'a', skill: 's', dependencies: [t1.id], input: {} });

      // t1 not done yet -> t2 not ready
      expect(tm.getReadyTasks(tm.getAll())).toHaveLength(1);

      // t1 done -> t2 ready
      tm.updateState(t1.id, 'done');
      const ready = tm.getReadyTasks(tm.getAll());
      expect(ready).toHaveLength(1);
      expect(ready[0].id).toBe(t2.id);
    });

    it('returns tasks with no dependencies immediately', () => {
      tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      expect(tm.getReadyTasks(tm.getAll())).toHaveLength(1);
    });

    it('excludes running tasks', () => {
      const t1 = tm.create({ name: 't1', agent: 'a', skill: 's', dependencies: [], input: {} });
      tm.updateState(t1.id, 'running');
      expect(tm.getReadyTasks(tm.getAll())).toHaveLength(0);
    });
  });
});
