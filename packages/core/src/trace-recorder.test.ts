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

import { TraceRecorder } from './trace-recorder.js';

describe('TraceRecorder', () => {
  let eventBus: EventBus;
  let recorder: TraceRecorder;

  beforeEach(() => {
    eventBus = new EventBus();
    recorder = new TraceRecorder(eventBus);
  });

  it('creates trace on workflow.started', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1', name: 'Test' });

    const trace = recorder.getTrace('wf1');
    expect(trace).toHaveLength(1);
    expect(trace[0].operation).toBe('workflow');
    expect(trace[0].spanId).toBe('wf1');
    recorder.stop();
  });

  it('closes span on workflow.completed', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('workflow.completed', { workflowId: 'wf1' });

    const trace = recorder.getTrace('wf1');
    expect(trace[0].endTime).toBeInstanceOf(Date);
    expect(trace[0].status).toBe('ok');
    recorder.stop();
  });

  it('marks span as error on workflow.failed', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('workflow.failed', { workflowId: 'wf1' });

    expect(recorder.getTrace('wf1')[0].status).toBe('error');
    recorder.stop();
  });

  it('creates task span under workflow trace', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('task.started', { taskId: 't1', workflowId: 'wf1', skill: 'git.status' });

    const trace = recorder.getTrace('wf1');
    expect(trace).toHaveLength(2);
    expect(trace[1].operation).toBe('task');
    expect(trace[1].parentSpanId).toBe('wf1');
    recorder.stop();
  });

  it('closes task span on task.completed', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('task.started', { taskId: 't1', workflowId: 'wf1' });
    eventBus.emit('task.completed', { taskId: 't1' });

    const taskSpan = recorder.getTrace('wf1').find(s => s.spanId === 't1');
    expect(taskSpan!.endTime).toBeInstanceOf(Date);
    expect(taskSpan!.status).toBe('ok');
    recorder.stop();
  });

  it('marks task span as error on task.failed', () => {
    recorder.start();
    eventBus.emit('task.started', { taskId: 't1' });
    eventBus.emit('task.failed', { taskId: 't1' });

    const traces = recorder.getAllTraces();
    const spans = Array.from(traces.values()).flat();
    const taskSpan = spans.find(s => s.spanId === 't1');
    expect(taskSpan!.status).toBe('error');
    recorder.stop();
  });

  it('emits trace.started event', () => {
    const handler = vi.fn();
    eventBus.on('trace.started' as any, handler);
    recorder.start();

    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    expect(handler).toHaveBeenCalledTimes(1);
    recorder.stop();
  });

  it('getAllTraces returns all traces', () => {
    recorder.start();
    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    eventBus.emit('workflow.started', { workflowId: 'wf2' });

    const all = recorder.getAllTraces();
    expect(all.size).toBe(2);
    recorder.stop();
  });

  it('stop unsubscribes from events', () => {
    recorder.start();
    recorder.stop();

    eventBus.emit('workflow.started', { workflowId: 'wf1' });
    expect(recorder.getTrace('wf1')).toHaveLength(0);
  });

  it('returns empty array for unknown trace', () => {
    expect(recorder.getTrace('unknown')).toHaveLength(0);
  });
});
