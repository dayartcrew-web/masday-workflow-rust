import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EventBus } from './eventBus.js';

describe('EventBus', () => {
  let bus: EventBus;

  beforeEach(() => {
    bus = new EventBus();
  });

  describe('emit / on', () => {
    it('emits and receives events', () => {
      const handler = vi.fn();
      bus.on('workflow.started', handler);
      bus.emit('workflow.started', { workflowId: 'w1' });
      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'workflow.started',
          data: { workflowId: 'w1' },
        })
      );
    });

    it('delivers to correct handler only', () => {
      const h1 = vi.fn();
      const h2 = vi.fn();
      bus.on('workflow.started', h1);
      bus.on('workflow.completed', h2);
      bus.emit('workflow.started', {});
      expect(h1).toHaveBeenCalledTimes(1);
      expect(h2).toHaveBeenCalledTimes(0);
    });

    it('supports multiple handlers for same event', () => {
      const h1 = vi.fn();
      const h2 = vi.fn();
      bus.on('workflow.started', h1);
      bus.on('workflow.started', h2);
      bus.emit('workflow.started', {});
      expect(h1).toHaveBeenCalledTimes(1);
      expect(h2).toHaveBeenCalledTimes(1);
    });
  });

  describe('once', () => {
    it('fires handler only once', () => {
      const handler = vi.fn();
      bus.once('workflow.started', handler);
      bus.emit('workflow.started', {});
      bus.emit('workflow.started', {});
      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  describe('getHistory', () => {
    it('records event history', () => {
      bus.emit('workflow.started', { a: 1 });
      bus.emit('workflow.completed', { b: 2 });
      const history = bus.getHistory();
      expect(history).toHaveLength(2);
      expect(history[0].type).toBe('workflow.started');
      expect(history[1].type).toBe('workflow.completed');
    });

    it('returns limited history', () => {
      bus.emit('workflow.started', {});
      bus.emit('workflow.completed', {});
      const limited = bus.getHistory(1);
      expect(limited).toHaveLength(1);
      expect(limited[0].type).toBe('workflow.completed');
    });

    it('returns empty array when no events', () => {
      expect(bus.getHistory()).toEqual([]);
    });
  });

  describe('getHistoryByType', () => {
    it('filters history by event type', () => {
      bus.emit('workflow.started', {});
      bus.emit('workflow.completed', {});
      bus.emit('workflow.started', {});
      const started = bus.getHistoryByType('workflow.started');
      expect(started).toHaveLength(2);
    });

    it('returns limited filtered history', () => {
      bus.emit('workflow.started', {});
      bus.emit('workflow.started', {});
      bus.emit('workflow.started', {});
      const limited = bus.getHistoryByType('workflow.started', 2);
      expect(limited).toHaveLength(2);
    });
  });

  describe('clearHistory', () => {
    it('clears all event history', () => {
      bus.emit('workflow.started', {});
      bus.clearHistory();
      expect(bus.getHistory()).toEqual([]);
    });
  });

  describe('event cap', () => {
    it('keeps last 1000 events', () => {
      for (let i = 0; i < 1010; i++) {
        bus.emit('workflow.started', { i });
      }
      const history = bus.getHistory();
      expect(history).toHaveLength(1000);
      expect(history[0].data.i).toBe(10);
    });
  });

  describe('event structure', () => {
    it('includes timestamp in events', () => {
      bus.emit('workflow.started', {});
      const [event] = bus.getHistory();
      expect(event.timestamp).toBeInstanceOf(Date);
    });
  });
});
