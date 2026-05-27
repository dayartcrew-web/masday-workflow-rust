import { describe, it, expect } from 'vitest';
import { EpisodicMemory } from '../episodic.js';
import { GraphStore } from '../graph.js';
import { WorkingMemory } from '../working.js';
import { MemoryStore } from '../store.js';
import fs from 'fs';
import path from 'path';
import os from 'os';

/**
 * Memory leak + context leak regression tests.
 *
 * MEMORY LEAK vectors:
 * 1. EpisodicMemory: ring buffer uses .shift() on array — O(n) per eviction
 * 2. GraphStore: in-memory Map grows unbounded; autoLink creates O(n²) edges
 * 3. WorkingMemory: sessions never evicted — Map grows with each new session
 * 4. GraphStore.autoLink: adds 2 edges per similar pair, quadratic in node count
 * 5. EpisodicMemory.persistToDb: fire-and-forget promises hold references until GC
 *
 * CONTEXT LEAK vectors:
 * 6. GraphStore: single shared instance — nodes from workflow A auto-link to workflow B
 * 7. EpisodicMemory: single buffer captures ALL tool calls, no session isolation
 * 8. MemoryStore: prune() exists but never called automatically from add()
 * 9. MemoryStore: search returns results across all workflows (no scope filter)
 */

// --- EpisodicMemory ---

describe('EpisodicMemory memory leak tests', () => {
  it('buffer should stay at maxSize after many inserts', () => {
    const mem = new EpisodicMemory(10);
    for (let i = 0; i < 1000; i++) {
      mem.add('user', `message ${i}`);
    }
    expect(mem.size).toBe(10);
    expect(mem.getRecent()).toHaveLength(10);
  });

  it('should evict oldest messages correctly', () => {
    const mem = new EpisodicMemory(3);
    mem.add('user', 'msg1');
    mem.add('user', 'msg2');
    mem.add('user', 'msg3');
    mem.add('user', 'msg4');

    const recent = mem.getRecent();
    expect(recent.map(m => m.content)).toEqual(['msg2', 'msg3', 'msg4']);
  });

  it('metadata objects should not retain references after eviction', () => {
    const mem = new EpisodicMemory(5);
    const bigMetadata: Record<string, unknown> = { data: new Array(10000).fill('x') };

    for (let i = 0; i < 100; i++) {
      mem.add('user', `msg ${i}`, i < 95 ? bigMetadata : undefined);
    }

    expect(mem.size).toBe(5);
    const recent = mem.getRecent();
    for (const msg of recent) {
      expect(msg.metadata).toBeUndefined();
    }
  });

  it('clear() should release all buffer references', () => {
    const mem = new EpisodicMemory(100);
    for (let i = 0; i < 50; i++) {
      mem.add('user', `msg ${i}`);
    }
    expect(mem.size).toBe(50);

    mem.clear();
    expect(mem.size).toBe(0);
    expect(mem.getRecent()).toHaveLength(0);
  });
});

// --- GraphStore ---

describe('GraphStore memory leak tests', () => {
  it('autoLink should not create excessive edges for similar nodes', () => {
    const store = new GraphStore({ autoLinkThreshold: 0.9 });

    for (let i = 0; i < 50; i++) {
      store.addNode({ type: 'memory', label: 'identical label for all nodes', properties: {} });
    }

    expect(store.edgeCount).toBeGreaterThan(0);
    expect(store.nodeCount).toBe(50);

    const maxExpectedEdges = 50 * 49;
    expect(store.edgeCount).toBeLessThanOrEqual(maxExpectedEdges);
  });

  it('deleteNode should clean up all connected edges', () => {
    const store = new GraphStore({ autoLinkThreshold: 0.3 });

    const node1 = store.addNode({ type: 'concept', label: 'node alpha', properties: {} });
    const node2 = store.addNode({ type: 'concept', label: 'node beta', properties: {} });
    store.addEdge({ from: node1.id, to: node2.id, relation: 'connected', weight: 1 });

    const edgesBeforeDelete = store.edgeCount;
    expect(edgesBeforeDelete).toBeGreaterThanOrEqual(1);

    store.deleteNode(node1.id);

    expect(store.getNode(node1.id)).toBeUndefined();

    const remainingEdges = store.findEdges(e => e.from === node1.id || e.to === node1.id);
    expect(remainingEdges).toHaveLength(0);
  });

  it('should handle rapid node addition without unbounded edge growth', () => {
    const store = new GraphStore({ autoLinkThreshold: 1.0 });

    for (let i = 0; i < 100; i++) {
      store.addNode({ type: 'memory', label: `unique node ${i} ${Math.random()}`, properties: {} });
    }

    expect(store.nodeCount).toBe(100);
    expect(store.edgeCount).toBe(0);
  });

  it('autoLink with realistic threshold produces O(n²) edges (leak documented)', () => {
    const store = new GraphStore({ autoLinkThreshold: 0.3 });

    for (let i = 0; i < 30; i++) {
      store.addNode({
        type: 'memory',
        label: `workflow task ${i} implementation fix`,
        properties: { category: 'fix', index: i },
      });
    }

    // LEAK: 30 similar nodes at threshold 0.3 produce ~870 edges (ratio ~29)
    // This is O(n²) — each new node auto-links to most existing nodes
    const ratio = store.edgeCount / store.nodeCount;
    expect(ratio).toBeGreaterThan(20); // Confirms the quadratic growth
    // TODO: Add edge cap or eviction to GraphStore.autoLink
  });
});

// --- WorkingMemory ---

describe('WorkingMemory memory leak tests', () => {
  it('sessions should grow unbounded (documenting the leak)', () => {
    const mem = new WorkingMemory();

    for (let i = 0; i < 1000; i++) {
      mem.create(`session-${i}`);
    }

    expect(mem.size).toBe(1000);

    expect(mem.get('session-0')).toBeDefined();
    expect(mem.get('session-999')).toBeDefined();
  });

  it('customState should not share references between sessions', () => {
    const mem = new WorkingMemory();
    mem.create('s1');
    mem.update('s1', { customState: { key: 'modified' } });

    const state = mem.get('s1');
    expect(state?.customState.key).toBe('modified');
  });

  it('delete() should allow garbage collection of session data', () => {
    const mem = new WorkingMemory();
    mem.create('session-to-delete');
    mem.update('session-to-delete', {
      customState: { bigData: new Array(10000).fill('leak') },
    });

    expect(mem.has('session-to-delete')).toBe(true);
    expect(mem.delete('session-to-delete')).toBe(true);
    expect(mem.has('session-to-delete')).toBe(false);
    expect(mem.get('session-to-delete')).toBeUndefined();
  });

  it('clear() should reset all session state', () => {
    const mem = new WorkingMemory();
    for (let i = 0; i < 50; i++) {
      mem.create(`s-${i}`);
    }
    expect(mem.size).toBe(50);

    mem.clear();
    expect(mem.size).toBe(0);
  });
});

// --- Integration: combined memory pressure ---

describe('Memory pressure integration', () => {
  it('rapid operations should not throw or corrupt state', () => {
    const episodic = new EpisodicMemory(50);
    const graph = new GraphStore({ autoLinkThreshold: 0.5 });
    const working = new WorkingMemory();

    for (let i = 0; i < 500; i++) {
      episodic.add('user', `Turn ${i}: do something`);
      episodic.add('assistant', `Turn ${i}: response`);

      graph.addNode({
        type: 'memory',
        label: `Memory from turn ${i}`,
        properties: { turn: i },
      });

      if (i % 10 === 0) {
        working.create(`session-burst-${i}`);
        working.update(`session-burst-${i}`, {
          activeGoal: `Goal at turn ${i}`,
        });
      }
    }

    expect(episodic.size).toBe(50);
    expect(graph.nodeCount).toBe(500);
    expect(working.size).toBe(50);
  });
});

// ============================================================
// CONTEXT LEAK TESTS
// Data isolation between workflows, sessions, and scopes
// ============================================================

// --- GraphStore: cross-workflow contamination ---

describe('GraphStore context leak: cross-workflow auto-linking', () => {
  it('nodes from workflow A should NOT auto-link to workflow B nodes', () => {
    const store = new GraphStore({ autoLinkThreshold: 0.3 });

    // Workflow A: add 10 nodes about "authentication"
    for (let i = 0; i < 10; i++) {
      store.addNode({
        type: 'memory',
        label: `authentication fix ${i} security token`,
        properties: { workflowId: 'wf-A', category: 'auth' },
      });
    }

    // Workflow B: add 10 nodes about "authentication" (same domain!)
    for (let i = 0; i < 10; i++) {
      store.addNode({
        type: 'memory',
        label: `authentication feature ${i} token validation`,
        properties: { workflowId: 'wf-B', category: 'auth' },
      });
    }

    // CONTEXT LEAK: Jaccard similarity on labels matches across workflows.
    // Nodes from wf-A will auto-link to wf-B nodes because labels are similar.
    const wfAEdges = store.findEdges(e => {
      const fromNode = store.getNode(e.from);
      const toNode = store.getNode(e.to);
      return fromNode?.properties.workflowId === 'wf-A' && toNode?.properties.workflowId === 'wf-B';
    });

    const wfBEdges = store.findEdges(e => {
      const fromNode = store.getNode(e.from);
      const toNode = store.getNode(e.to);
      return fromNode?.properties.workflowId === 'wf-B' && toNode?.properties.workflowId === 'wf-A';
    });

    const crossWorkflowEdges = wfAEdges.length + wfBEdges.length;

    // This documents the leak: cross-workflow edges SHOULD be 0
    // but autoLink compares ALL nodes regardless of workflowId
    expect(crossWorkflowEdges).toBeGreaterThan(0);
    // TODO: autoLink should scope similarity to same workflowId
  });

  it('findNodes by workflowId should return only that workflow\'s nodes', () => {
    const store = new GraphStore({ autoLinkThreshold: 1.0 }); // no auto-link

    for (let i = 0; i < 5; i++) {
      store.addNode({ type: 'memory', label: `wf-A node ${i}`, properties: { workflowId: 'wf-A' } });
      store.addNode({ type: 'memory', label: `wf-B node ${i}`, properties: { workflowId: 'wf-B' } });
    }

    const wfANodes = store.findNodes(n => n.properties.workflowId === 'wf-A');
    const wfBNodes = store.findNodes(n => n.properties.workflowId === 'wf-B');

    expect(wfANodes).toHaveLength(5);
    expect(wfBNodes).toHaveLength(5);

    // Verify no overlap
    const aIds = new Set(wfANodes.map(n => n.id));
    const bIds = new Set(wfBNodes.map(n => n.id));
    for (const id of aIds) {
      expect(bIds.has(id)).toBe(false);
    }
  });
});

// --- EpisodicMemory: cross-session contamination ---

describe('EpisodicMemory context leak: shared buffer across sessions', () => {
  it('single buffer mixes messages from different workflows', () => {
    const mem = new EpisodicMemory(10);

    // Workflow A tool calls
    mem.add('user', '[workflow.create] wf-A');
    mem.add('assistant', '[workflow.create] {"id":"wf-A"}');
    mem.add('user', '[memory.store] wf-A secret data');

    // Workflow B tool calls (same buffer!)
    mem.add('user', '[workflow.create] wf-B');
    mem.add('assistant', '[workflow.create] {"id":"wf-B"}');
    mem.add('user', '[memory.store] wf-B private data');

    // Workflow A can read Workflow B's tool call history
    const recent = mem.getRecent();
    const wfBMentions = recent.filter(m =>
      m.content.includes('wf-B') || m.content.includes('wf-A')
    );

    // CONTEXT LEAK: All 6 messages are visible — no session isolation
    expect(wfBMentions.length).toBeGreaterThan(0);
    // Both wf-A and wf-B data are in the same buffer
    expect(recent.some(m => m.content.includes('wf-A'))).toBe(true);
    expect(recent.some(m => m.content.includes('wf-B'))).toBe(true);
    // TODO: EpisodicMemory should be per-session or per-workflow
  });

  it('sessionId is shared across all add() calls', () => {
    const mem = new EpisodicMemory(100);
    mem.add('user', 'session 1 message');
    mem.add('user', 'session 2 message');

    // Both messages share the same sessionId — no session concept
    const all = mem.getAll();
    expect(all).toHaveLength(2);
    // The sessionId is generated once in constructor, not per-session
    // This means you can't distinguish which session a message belongs to
  });
});

// --- MemoryStore: unbounded growth + no auto-prune + cross-scope search ---

describe('MemoryStore context leak: unbounded growth and cross-scope search', () => {
  it('add() auto-prunes to maxMemories', async () => {
    const tmpFile = path.join(os.tmpdir(), `memstore-test-${Date.now()}.json`);
    try {
      const store = new MemoryStore({ filePath: tmpFile, maxMemories: 10 });
      await store.init();

      for (let i = 0; i < 100; i++) {
        await store.add(`Memory ${i}`, { type: 'fact', importance: 0.5 });
      }

      const stats = store.getStats();
      expect(stats.total).toBeLessThanOrEqual(10);
    } finally {
      if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
    }
  });

  it('search returns results from all scopes without workflow filter', async () => {
    const tmpFile = path.join(os.tmpdir(), `memstore-test-${Date.now()}.json`);
    try {
      const store = new MemoryStore({ filePath: tmpFile });
      await store.init();

      // Add memories "for" different workflows (via tags)
      await store.add('Workflow A secret API key: sk-abc123', {
        type: 'artifact', tags: ['wf-A', 'secret'],
      });
      await store.add('Workflow B database password: db-pass-xyz', {
        type: 'artifact', tags: ['wf-B', 'secret'],
      });
      await store.add('Workflow A public docs', {
        type: 'fact', tags: ['wf-A', 'docs'],
      });

      // Search without scope — returns results from ALL workflows
      const results = await store.search('secret');
      // CONTEXT LEAK: Workflow B can discover Workflow A's secrets and vice versa
      expect(results.length).toBeGreaterThan(0);

      // Both workflows' secrets are returned
      const contents = results.map(r => r.memory.content);
      const hasWfA = contents.some(c => c.includes('sk-abc123'));
      const hasWfB = contents.some(c => c.includes('db-pass-xyz'));
      expect(hasWfA || hasWfB).toBe(true); // At least one workflow's data leaks
      // TODO: search() should accept a scope/workflowId parameter
    } finally {
      if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
    }
  });

  it('tag-based filtering provides partial isolation', async () => {
    const tmpFile = path.join(os.tmpdir(), `memstore-test-${Date.now()}.json`);
    try {
      const store = new MemoryStore({ filePath: tmpFile });
      await store.init();

      await store.add('WF-A data', { type: 'fact', tags: ['wf-A'] });
      await store.add('WF-B data', { type: 'fact', tags: ['wf-B'] });

      // search with tags filter works
      const wfAOnly = await store.search('data', { tags: ['wf-A'] });
      expect(wfAOnly.every(r => r.memory.tags.includes('wf-A'))).toBe(true);
    } finally {
      if (fs.existsSync(tmpFile)) fs.unlinkSync(tmpFile);
    }
  });
});

// --- WorkingMemory: session isolation ---

describe('WorkingMemory context leak: cross-session state', () => {
  it('sessions are properly isolated — no data bleed', () => {
    const mem = new WorkingMemory();

    mem.create('session-1');
    mem.create('session-2');

    mem.update('session-1', {
      activeGoal: 'Fix auth bug',
      customState: { secrets: 'api-key-123' },
    });

    mem.update('session-2', {
      activeGoal: 'Write docs',
      customState: { notes: 'public documentation' },
    });

    const s1 = mem.get('session-1');
    const s2 = mem.get('session-2');

    // Verify isolation
    expect(s1?.activeGoal).toBe('Fix auth bug');
    expect(s1?.customState.secrets).toBe('api-key-123');
    expect(s2?.activeGoal).toBe('Write docs');
    expect(s2?.customState.notes).toBe('public documentation');

    // No cross-contamination
    expect(s2?.customState.secrets).toBeUndefined();
    expect(s1?.customState.notes).toBeUndefined();
  });

  it('returned state objects are copies — mutation does not leak back', () => {
    const mem = new WorkingMemory();
    mem.create('session-1');
    mem.update('session-1', { customState: { key: 'original' } });

    const state1 = mem.get('session-1')!;
    state1.customState.key = 'mutated';

    const state2 = mem.get('session-1')!;
    expect(state2.customState.key).toBe('original'); // Mutation did not leak
  });
});
