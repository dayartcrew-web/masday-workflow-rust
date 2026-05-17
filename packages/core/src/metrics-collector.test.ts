import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EventBus } from './eventBus.js';

vi.mock('./logger.js', () => ({
  createLogger: () => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  }),
}));

import { MetricsCollector } from './metrics-collector.js';

describe('MetricsCollector', () => {
  let eventBus: EventBus;
  let collector: MetricsCollector;

  beforeEach(() => {
    eventBus = new EventBus();
    collector = new MetricsCollector(eventBus);
  });

  it('records task duration on task.completed', () => {
    collector.start();
    eventBus.emit('task.started', { taskId: 't1' });

    // Advance time by emitting completed after a delay
    const start = Date.now();
    eventBus.emit('task.completed', { taskId: 't1', result: 'ok' });

    const metrics = collector.getMetric('task.duration');
    expect(metrics).toHaveLength(1);
    expect(metrics[0].tags.success).toBe('true');
    collector.stop();
  });

  it('records task duration on task.failed', () => {
    collector.start();
    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.failed', { taskId: 't1', error: 'boom' });

    const metrics = collector.getMetric('task.duration');
    expect(metrics).toHaveLength(1);
    expect(metrics[0].tags.success).toBe('false');
    collector.stop();
  });

  it('records workflow duration', () => {
    collector.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('workflow.completed', { workflowId: 'wf1' });

    const metrics = collector.getMetric('workflow.duration');
    expect(metrics).toHaveLength(1);
    expect(metrics[0].tags.success).toBe('true');
    collector.stop();
  });

  it('records failed workflow duration', () => {
    collector.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('workflow.failed', { workflowId: 'wf1', error: 'fail' });

    const metrics = collector.getMetric('workflow.duration');
    expect(metrics).toHaveLength(1);
    expect(metrics[0].tags.success).toBe('false');
    collector.stop();
  });

  it('ignores events without start', () => {
    collector.start();
    eventBus.emit('task.completed', { taskId: 'unknown' });
    expect(collector.getMetric('task.duration')).toHaveLength(0);
    collector.stop();
  });

  it('computes summary correctly', () => {
    collector.start();
    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });
    eventBus.emit('task.started', { taskId: 't2' });
    eventBus.emit('task.completed', { taskId: 't2' });

    const summary = collector.getSummary('task.duration');
    expect(summary.count).toBe(2);
    expect(summary.min).toBeGreaterThanOrEqual(0);
    expect(summary.avg).toBeGreaterThanOrEqual(0);
    collector.stop();
  });

  it('filters summary by tags', () => {
    collector.start();
    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });
    eventBus.emit('task.started', { taskId: 't2' });
    eventBus.emit('task.failed', { taskId: 't2' });

    const successOnly = collector.getSummary('task.duration', { success: 'true' });
    expect(successOnly.count).toBe(1);
    collector.stop();
  });

  it('returns empty summary for unknown metric', () => {
    const summary = collector.getSummary('nonexistent');
    expect(summary.count).toBe(0);
  });

  it('getAllMetrics returns map', () => {
    collector.start();
    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });

    const all = collector.getAllMetrics();
    expect(all.has('task.duration')).toBe(true);
    collector.stop();
  });

  it('stop unsubscribes from events', () => {
    collector.start();
    collector.stop();

    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });

    expect(collector.getMetric('task.duration')).toHaveLength(0);
  });

  it('does not double-subscribe on double start', () => {
    collector.start();
    collector.start();

    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });

    expect(collector.getMetric('task.duration')).toHaveLength(1);
    collector.stop();
  });

  it('emits metrics.recorded event', () => {
    const handler = vi.fn();
    eventBus.on('metrics.recorded' as any, handler);
    collector.start();

    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.completed', { taskId: 't1' });

    expect(handler).toHaveBeenCalledTimes(1);
    collector.stop();
  });
});
