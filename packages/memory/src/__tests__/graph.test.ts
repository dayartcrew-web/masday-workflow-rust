import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { GraphStore, setGraphDb } from '../graph.js';
import type { GraphNodeRecord, GraphEdgeRecord } from '@mcp-rebuild/core';
import fs from 'fs';
import path from 'path';
import os from 'os';

function tempFilePath(prefix: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  return path.join(dir, 'graph.json');
}

function createNode(overrides?: Partial<Omit<GraphNodeRecord, 'id'>>): Omit<GraphNodeRecord, 'id'> {
  return { type: 'concept', label: 'test node', properties: { key: 'value' }, ...overrides };
}

function createEdge(from: string, to: string, overrides?: Partial<Omit<GraphEdgeRecord, 'id'>>): Omit<GraphEdgeRecord, 'id'> {
  return { from, to, relation: 'related', weight: 1, ...overrides };
}

describe('GraphStore', () => {
  describe('constructor', () => {
    it('defaults autoLinkThreshold to 0.4', () => {
      const store = new GraphStore();
      store.addNode(createNode({ label: 'machine learning model' }));
      store.addNode(createNode({ label: 'deep learning model training' }));
      // Jaccard of "machine learning model" vs "deep learning model training" = 2/5 = 0.4
      const edges = store.findEdges(() => true);
      expect(edges.length).toBeGreaterThan(0);
    });

    it('defaults filePath to null', async () => {
      const store = new GraphStore();
      await expect(store.init()).resolves.toBeUndefined();
      await expect(store.save()).resolves.toBeUndefined();
    });

    it('accepts custom autoLinkThreshold', () => {
      const store = new GraphStore({ autoLinkThreshold: 0.9 });
      store.addNode(createNode({ label: 'machine learning model' }));
      store.addNode(createNode({ label: 'deep learning model training' }));
      expect(store.findEdges(() => true).length).toBe(0);
    });

    it('accepts custom filePath', () => {
      const fp = tempFilePath('graph-test-constructor');
      const store = new GraphStore({ filePath: fp });
      expect(store).toBeDefined();
      fs.rmSync(path.dirname(fp), { recursive: true, force: true });
    });
  });

  describe('addNode', () => {
    let store: GraphStore;

    beforeEach(() => { store = new GraphStore({ autoLinkThreshold: 999 }); });

    it('adds a node with an auto-generated UUID', () => {
      const node = store.addNode(createNode());
      expect(node.id).toBeDefined();
      expect(typeof node.id).toBe('string');
      expect(node.id.length).toBeGreaterThan(0);
    });

    it('generates unique IDs for multiple nodes', () => {
      const ids = new Set([
        store.addNode(createNode()).id,
        store.addNode(createNode()).id,
        store.addNode(createNode()).id,
      ]);
      expect(ids.size).toBe(3);
    });

    it('preserves node type, label, and properties', () => {
      const node = store.addNode(createNode({
        type: 'skill', label: 'vitest', properties: { version: '4.x', coverage: true },
      }));
      expect(node.type).toBe('skill');
      expect(node.label).toBe('vitest');
      expect(node.properties).toEqual({ version: '4.x', coverage: true });
    });

    it('returns a shallow clone (label is copied, properties share reference)', () => {
      const node = store.addNode(createNode({ properties: { a: 1 } }));
      // label is spread-copied, so mutation does not leak
      node.label = 'changed';
      // properties is spread at addNode input level, but returned object shallow-copies record
      node.properties.a = 99;
      const retrieved = store.getNode(node.id);
      expect(retrieved!.label).toBe('test node');
      // properties object IS shared (shallow spread on record, returned via { ...record })
      expect(retrieved!.properties.a).toBe(99);
    });

    it('returns a deep copy of properties object', () => {
      const inputProps = { nested: { deep: true } };
      const node = store.addNode(createNode({ properties: inputProps }));
      (inputProps as Record<string, unknown>).nested = { deep: false };
      const retrieved = store.getNode(node.id);
      expect(retrieved!.properties).toEqual({ nested: { deep: true } });
    });

    it('increments nodeCount', () => {
      expect(store.nodeCount).toBe(0);
      store.addNode(createNode());
      expect(store.nodeCount).toBe(1);
      store.addNode(createNode());
      store.addNode(createNode());
      expect(store.nodeCount).toBe(3);
    });

    it('triggers autoLink for similar nodes by Jaccard similarity', () => {
      const astore = new GraphStore();
      astore.addNode(createNode({ label: 'machine learning model', properties: {} }));
      astore.addNode(createNode({ label: 'deep learning model training', properties: {} }));
      const edges = astore.findEdges(e => e.relation === 'similar');
      expect(edges.length).toBe(2);
    });

    it('does not autoLink nodes below similarity threshold', () => {
      const astore = new GraphStore();
      astore.addNode(createNode({ label: 'apple fruit', properties: {} }));
      astore.addNode(createNode({ label: 'quantum physics theory', properties: {} }));
      expect(astore.findEdges(() => true).length).toBe(0);
    });

    it('autoLink creates edges with correct weight equal to similarity', () => {
      const astore = new GraphStore();
      astore.addNode(createNode({ label: 'machine learning model', properties: {} }));
      astore.addNode(createNode({ label: 'deep learning model training', properties: {} }));
      const edges = astore.findEdges(e => e.relation === 'similar');
      // Jaccard: {machine,learning,model} vs {deep,learning,model,training} = 2/5 = 0.4
      expect(edges[0].weight).toBeCloseTo(0.4);
      expect(edges[1].weight).toBeCloseTo(0.4);
    });

    it('autoLink considers property string values in Jaccard calculation', () => {
      const astore = new GraphStore({ autoLinkThreshold: 0.3 });
      astore.addNode(createNode({ label: 'node a', properties: { desc: 'typescript testing framework' } }));
      astore.addNode(createNode({ label: 'node b', properties: { desc: 'typescript testing library' } }));
      const edges = astore.findEdges(e => e.relation === 'similar');
      expect(edges.length).toBe(2);
    });

    it('does not autoLink node with itself', () => {
      const astore = new GraphStore();
      const anode = astore.addNode(createNode({ label: 'test', properties: {} }));
      const edges = astore.findEdges(e => e.from === anode.id && e.to === anode.id);
      expect(edges.length).toBe(0);
    });

    it('handles empty properties', () => {
      const node = store.addNode(createNode({ properties: {} }));
      expect(node.properties).toEqual({});
      expect(store.getNode(node.id)!.properties).toEqual({});
    });
  });

  describe('getNode', () => {
    let store: GraphStore;
    let node: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      node = store.addNode(createNode({ label: 'my node', properties: { x: 10 } }));
    });

    it('retrieves a node by id', () => {
      const retrieved = store.getNode(node.id);
      expect(retrieved).toBeDefined();
      expect(retrieved!.id).toBe(node.id);
      expect(retrieved!.label).toBe('my node');
      expect(retrieved!.type).toBe('concept');
    });

    it('returns a deep copy (modifying return value does not mutate stored node)', () => {
      const retrieved = store.getNode(node.id)!;
      retrieved.label = 'mutated';
      (retrieved.properties as Record<string, number>).x = 999;
      const again = store.getNode(node.id)!;
      expect(again.label).toBe('my node');
      expect(again.properties).toEqual({ x: 10 });
    });

    it('returns deep copy of properties', () => {
      const retrieved = store.getNode(node.id)!;
      expect(retrieved.properties).not.toBe(node.properties);
      expect(retrieved.properties).toEqual(node.properties);
    });

    it('returns undefined for unknown id', () => {
      expect(store.getNode('nonexistent')).toBeUndefined();
    });

    it('returns undefined for empty string id', () => {
      expect(store.getNode('')).toBeUndefined();
    });
  });

  describe('updateNode', () => {
    let store: GraphStore;
    let node: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      node = store.addNode(createNode({ label: 'original', properties: { a: 1, b: 2 } }));
    });

    it('updates label', () => {
      const updated = store.updateNode(node.id, { label: 'new label' });
      expect(updated!.label).toBe('new label');
      expect(store.getNode(node.id)!.label).toBe('new label');
    });

    it('updates properties (replaces entirely)', () => {
      const updated = store.updateNode(node.id, { properties: { c: 3 } });
      expect(updated!.properties).toEqual({ c: 3 });
      expect(store.getNode(node.id)!.properties).toEqual({ c: 3 });
    });

    it('preserves unchanged fields', () => {
      store.updateNode(node.id, { label: 'new label' });
      const retrieved = store.getNode(node.id)!;
      expect(retrieved.type).toBe('concept');
      expect(retrieved.properties).toEqual({ a: 1, b: 2 });
    });

    it('returns a deep copy', () => {
      const updated = store.updateNode(node.id, { properties: { x: 1 } })!;
      (updated.properties as Record<string, number>).x = 99;
      expect(store.getNode(node.id)!.properties).toEqual({ x: 1 });
    });

    it('returns undefined for unknown id', () => {
      expect(store.updateNode('ghost', { label: 'nope' })).toBeUndefined();
    });

    it('can update both label and properties simultaneously', () => {
      const updated = store.updateNode(node.id, { label: 'renamed', properties: { key: 'value' } });
      expect(updated!.label).toBe('renamed');
      expect(updated!.properties).toEqual({ key: 'value' });
    });

    it('handles partial update gracefully (unchanged properties)', () => {
      const updated = store.updateNode(node.id, { label: 'partial' });
      expect(updated!.label).toBe('partial');
      expect(updated!.properties).toEqual({ a: 1, b: 2 });
    });
  });

  describe('deleteNode', () => {
    let store: GraphStore;
    let nodeA: GraphNodeRecord;
    let nodeB: GraphNodeRecord;
    let nodeC: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      nodeA = store.addNode(createNode({ label: 'A' }));
      nodeB = store.addNode(createNode({ label: 'B' }));
      nodeC = store.addNode(createNode({ label: 'C' }));
      store.addEdge(createEdge(nodeA.id, nodeB.id));
      store.addEdge(createEdge(nodeB.id, nodeC.id));
      store.addEdge(createEdge(nodeA.id, nodeC.id));
    });

    it('returns true when node existed', () => {
      expect(store.deleteNode(nodeA.id)).toBe(true);
    });

    it('returns false when node did not exist', () => {
      expect(store.deleteNode('ghost-id')).toBe(false);
    });

    it('decrements nodeCount', () => {
      expect(store.nodeCount).toBe(3);
      store.deleteNode(nodeA.id);
      expect(store.nodeCount).toBe(2);
    });

    it('removes connected edges where node is source', () => {
      const edgeCountBefore = store.edgeCount;
      store.deleteNode(nodeA.id);
      expect(store.edgeCount).toBeLessThan(edgeCountBefore);
    });

    it('does not remove edges between remaining nodes', () => {
      store.deleteNode(nodeA.id);
      const edges = store.findEdges(e => e.from === nodeB.id && e.to === nodeC.id);
      expect(edges.length).toBe(1);
    });

    it('node is no longer retrievable after deletion', () => {
      store.deleteNode(nodeA.id);
      expect(store.getNode(nodeA.id)).toBeUndefined();
    });

    it('removes all edges where node is source or target', () => {
      const e1 = store.addEdge(createEdge(nodeA.id, nodeB.id, { relation: 'from-a' }));
      const e2 = store.addEdge(createEdge(nodeB.id, nodeA.id, { relation: 'to-a' }));
      store.deleteNode(nodeA.id);
      expect(store.getEdge(e1!.id)).toBeUndefined();
      expect(store.getEdge(e2!.id)).toBeUndefined();
    });
  });

  describe('findNodes', () => {
    let store: GraphStore;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      store.addNode(createNode({ type: 'skill', label: 'vitest', properties: { lang: 'ts' } }));
      store.addNode(createNode({ type: 'tool', label: 'eslint', properties: { lang: 'ts' } }));
      store.addNode(createNode({ type: 'skill', label: 'pytest', properties: { lang: 'python' } }));
    });

    it('filters by type', () => {
      const skills = store.findNodes(n => n.type === 'skill');
      expect(skills.length).toBe(2);
      expect(skills.every(n => n.type === 'skill')).toBe(true);
    });

    it('filters by label', () => {
      const results = store.findNodes(n => n.label === 'vitest');
      expect(results.length).toBe(1);
      expect(results[0].label).toBe('vitest');
    });

    it('filters by properties', () => {
      const results = store.findNodes(n => n.properties.lang === 'ts');
      expect(results.length).toBe(2);
    });

    it('returns empty array when no match', () => {
      expect(store.findNodes(n => n.type === 'nonexistent' as never)).toEqual([]);
    });

    it('returns deep copies', () => {
      const results = store.findNodes(n => n.label === 'vitest');
      results[0].label = 'mutated';
      (results[0].properties as Record<string, string>).lang = 'rust';
      const again = store.findNodes(n => n.label === 'vitest');
      expect(again[0].label).toBe('vitest');
      expect(again[0].properties.lang).toBe('ts');
    });

    it('handles complex predicate with multiple conditions', () => {
      const results = store.findNodes(
        n => n.type === 'skill' && n.properties.lang === 'ts'
      );
      expect(results.length).toBe(1);
      expect(results[0].label).toBe('vitest');
    });
  });

  describe('addEdge', () => {
    let store: GraphStore;
    let nodeA: GraphNodeRecord;
    let nodeB: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      nodeA = store.addNode(createNode({ label: 'A' }));
      nodeB = store.addNode(createNode({ label: 'B' }));
    });

    it('adds an edge with auto-generated UUID', () => {
      const edge = store.addEdge(createEdge(nodeA.id, nodeB.id));
      expect(edge).toBeDefined();
      expect(edge!.id).toBeDefined();
      expect(typeof edge!.id).toBe('string');
    });

    it('preserves edge relation and weight', () => {
      const edge = store.addEdge(createEdge(nodeA.id, nodeB.id, { relation: 'depends_on', weight: 5 }));
      expect(edge!.relation).toBe('depends_on');
      expect(edge!.weight).toBe(5);
    });

    it('returns deep copy', () => {
      const edge = store.addEdge(createEdge(nodeA.id, nodeB.id, { weight: 1 }))!;
      edge.weight = 999;
      expect(store.getEdge(edge.id)!.weight).toBe(1);
    });

    it('returns undefined when from node is missing', () => {
      expect(store.addEdge(createEdge('ghost', nodeB.id))).toBeUndefined();
    });

    it('returns undefined when to node is missing', () => {
      expect(store.addEdge(createEdge(nodeA.id, 'ghost'))).toBeUndefined();
    });

    it('returns undefined when both nodes are missing', () => {
      expect(store.addEdge(createEdge('ghost1', 'ghost2'))).toBeUndefined();
    });

    it('increments edgeCount', () => {
      expect(store.edgeCount).toBe(0);
      store.addEdge(createEdge(nodeA.id, nodeB.id));
      expect(store.edgeCount).toBe(1);
    });
  });

  describe('getEdge', () => {
    let store: GraphStore;
    let edge: GraphEdgeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      const a = store.addNode(createNode({ label: 'A' }));
      const b = store.addNode(createNode({ label: 'B' }));
      edge = store.addEdge(createEdge(a.id, b.id, { relation: 'tested', weight: 3 }))!;
    });

    it('retrieves edge by id', () => {
      const retrieved = store.getEdge(edge.id);
      expect(retrieved).toBeDefined();
      expect(retrieved!.relation).toBe('tested');
      expect(retrieved!.weight).toBe(3);
    });

    it('returns deep copy', () => {
      const retrieved = store.getEdge(edge.id)!;
      retrieved.relation = 'mutated';
      retrieved.weight = 999;
      expect(store.getEdge(edge.id)!.relation).toBe('tested');
      expect(store.getEdge(edge.id)!.weight).toBe(3);
    });

    it('returns undefined for unknown id', () => {
      expect(store.getEdge('nonexistent')).toBeUndefined();
    });

    it('returns undefined after edge deletion', () => {
      store.deleteEdge(edge.id);
      expect(store.getEdge(edge.id)).toBeUndefined();
    });
  });

  describe('deleteEdge', () => {
    let store: GraphStore;
    let edge: GraphEdgeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      const a = store.addNode(createNode({ label: 'A' }));
      const b = store.addNode(createNode({ label: 'B' }));
      edge = store.addEdge(createEdge(a.id, b.id))!;
    });

    it('returns true when edge existed', () => {
      expect(store.deleteEdge(edge.id)).toBe(true);
    });

    it('returns false when edge did not exist', () => {
      expect(store.deleteEdge('ghost')).toBe(false);
    });

    it('decrements edgeCount', () => {
      expect(store.edgeCount).toBe(1);
      store.deleteEdge(edge.id);
      expect(store.edgeCount).toBe(0);
    });

    it('edge is no longer retrievable after deletion', () => {
      store.deleteEdge(edge.id);
      expect(store.getEdge(edge.id)).toBeUndefined();
    });

    it('does not affect nodes', () => {
      store.deleteEdge(edge.id);
      expect(store.getNode(edge.from)).toBeDefined();
      expect(store.getNode(edge.to)).toBeDefined();
    });
  });

  describe('findEdges', () => {
    let store: GraphStore;
    let a: string;
    let b: string;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      a = store.addNode(createNode({ label: 'A' })).id;
      b = store.addNode(createNode({ label: 'B' })).id;
      const c = store.addNode(createNode({ label: 'C' })).id;
      store.addEdge(createEdge(a, b, { relation: 'depends_on', weight: 5 }));
      store.addEdge(createEdge(b, c, { relation: 'similar', weight: 0.8 }));
      store.addEdge(createEdge(a, c, { relation: 'depends_on', weight: 3 }));
    });

    it('filters by relation', () => {
      const deps = store.findEdges(e => e.relation === 'depends_on');
      expect(deps.length).toBe(2);
      expect(deps.every(e => e.relation === 'depends_on')).toBe(true);
    });

    it('filters by weight threshold', () => {
      expect(store.findEdges(e => e.weight >= 3).length).toBe(2);
    });

    it('filters by source node', () => {
      expect(store.findEdges(e => e.from === a).length).toBe(2);
    });

    it('returns empty array when no match', () => {
      expect(store.findEdges(e => e.relation === 'nonexistent')).toEqual([]);
    });

    it('returns deep copies', () => {
      const results = store.findEdges(e => e.relation === 'similar');
      results[0].weight = 999;
      const again = store.findEdges(e => e.relation === 'similar');
      expect(again[0].weight).toBe(0.8);
    });
  });

  describe('getNeighbors', () => {
    let store: GraphStore;
    let A: GraphNodeRecord;
    let B: GraphNodeRecord;
    let C: GraphNodeRecord;
    let D: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      A = store.addNode(createNode({ label: 'A' }));
      B = store.addNode(createNode({ label: 'B' }));
      C = store.addNode(createNode({ label: 'C' }));
      D = store.addNode(createNode({ label: 'D' }));
      store.addEdge(createEdge(A.id, B.id, { relation: 'depends_on' }));
      store.addEdge(createEdge(B.id, C.id, { relation: 'depends_on' }));
      store.addEdge(createEdge(D.id, A.id, { relation: 'calls' }));
    });

    it('returns outgoing neighbors (direction out)', () => {
      const neighbors = store.getNeighbors(B.id, { direction: 'out' });
      expect(neighbors.length).toBe(1);
      expect(neighbors[0].label).toBe('C');
    });

    it('returns incoming neighbors (direction in)', () => {
      const neighbors = store.getNeighbors(B.id, { direction: 'in' });
      expect(neighbors.length).toBe(1);
      expect(neighbors[0].label).toBe('A');
    });

    it('returns both directions when direction not specified (default)', () => {
      expect(store.getNeighbors(B.id).length).toBe(2);
    });

    it('filters by relation', () => {
      const neighbors = store.getNeighbors(A.id, { relation: 'depends_on' });
      expect(neighbors.length).toBe(1);
      expect(neighbors[0].label).toBe('B');
    });

    it('returns empty array when no edges match', () => {
      expect(store.getNeighbors(C.id, { direction: 'out' }).length).toBe(0);
    });

    it('returns deep copies of neighbor nodes', () => {
      const neighbors = store.getNeighbors(B.id);
      neighbors[0].label = 'mutated';
      const again = store.getNeighbors(B.id);
      expect(again.some(n => n.label === 'A')).toBe(true);
    });

    it('handles node with no edges at all', () => {
      const isolated = store.addNode(createNode({ label: 'E' }));
      expect(store.getNeighbors(isolated.id)).toEqual([]);
    });

    it('both direction includes outgoing and incoming', () => {
      const neighbors = store.getNeighbors(A.id, { direction: 'both' });
      expect(neighbors.length).toBe(2);
      const labels = new Set(neighbors.map(n => n.label));
      expect(labels.has('B')).toBe(true);
      expect(labels.has('D')).toBe(true);
    });
  });

  describe('getShortestPath', () => {
    let store: GraphStore;
    let A: GraphNodeRecord;
    let B: GraphNodeRecord;
    let C: GraphNodeRecord;
    let D: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      A = store.addNode(createNode({ label: 'A' }));
      B = store.addNode(createNode({ label: 'B' }));
      C = store.addNode(createNode({ label: 'C' }));
      D = store.addNode(createNode({ label: 'D' }));
      store.addEdge(createEdge(A.id, B.id, { weight: 1 }));
      store.addEdge(createEdge(A.id, C.id, { weight: 5 }));
      store.addEdge(createEdge(B.id, C.id, { weight: 1 }));
      store.addEdge(createEdge(C.id, D.id, { weight: 2 }));
    });

    it('returns path with lowest total weight using Dijkstra', () => {
      const result = store.getShortestPath(A.id, C.id);
      expect(result).toBeDefined();
      expect(result!.path).toEqual([A.id, B.id, C.id]);
      expect(result!.distance).toBe(2);
    });

    it('returns { path: [id], distance: 0 } for same node', () => {
      expect(store.getShortestPath(A.id, A.id)).toEqual({ path: [A.id], distance: 0 });
    });

    it('returns undefined when from node is missing', () => {
      expect(store.getShortestPath('ghost', A.id)).toBeUndefined();
    });

    it('returns undefined when to node is missing', () => {
      expect(store.getShortestPath(A.id, 'ghost')).toBeUndefined();
    });

    it('returns undefined when nodes are unreachable', () => {
      const isolated = store.addNode(createNode({ label: 'isolated' }));
      expect(store.getShortestPath(A.id, isolated.id)).toBeUndefined();
    });

    it('finds direct path when single edge exists', () => {
      const result = store.getShortestPath(A.id, B.id);
      expect(result!.path).toEqual([A.id, B.id]);
      expect(result!.distance).toBe(1);
    });

    it('handles chain of multiple edges', () => {
      const result = store.getShortestPath(A.id, D.id);
      expect(result!.path).toEqual([A.id, B.id, C.id, D.id]);
      expect(result!.distance).toBe(4);
    });

    it('picks path with lower sum even if more edges', () => {
      const E = store.addNode(createNode({ label: 'E' }));
      store.addEdge(createEdge(A.id, E.id, { weight: 0.5 }));
      store.addEdge(createEdge(E.id, D.id, { weight: 0.5 }));
      const result = store.getShortestPath(A.id, D.id);
      expect(result!.distance).toBeCloseTo(1);
    });
  });

  describe('getSubgraph', () => {
    let store: GraphStore;
    let root: GraphNodeRecord;
    let child1: GraphNodeRecord;
    let child2: GraphNodeRecord;
    let grandchild: GraphNodeRecord;

    beforeEach(() => {
      store = new GraphStore({ autoLinkThreshold: 999 });
      root = store.addNode(createNode({ label: 'root' }));
      child1 = store.addNode(createNode({ label: 'child1' }));
      child2 = store.addNode(createNode({ label: 'child2' }));
      grandchild = store.addNode(createNode({ label: 'grandchild' }));
      store.addEdge(createEdge(root.id, child1.id));
      store.addEdge(createEdge(root.id, child2.id));
      store.addEdge(createEdge(child1.id, grandchild.id));
    });

    it('returns only root node at depth 0', () => {
      const sub = store.getSubgraph(root.id, 0);
      expect(sub.nodes.length).toBe(1);
      expect(sub.nodes[0].id).toBe(root.id);
      expect(sub.edges.length).toBe(0);
    });

    it('returns root and direct neighbors at depth 1', () => {
      const sub = store.getSubgraph(root.id, 1);
      const nodeIds = new Set(sub.nodes.map(n => n.id));
      expect(nodeIds.has(root.id)).toBe(true);
      expect(nodeIds.has(child1.id)).toBe(true);
      expect(nodeIds.has(child2.id)).toBe(true);
      expect(nodeIds.has(grandchild.id)).toBe(false);
      expect(sub.edges.length).toBe(2);
    });

    it('includes grandchildren at depth 2', () => {
      const sub = store.getSubgraph(root.id, 2);
      const nodeIds = new Set(sub.nodes.map(n => n.id));
      expect(nodeIds.has(grandchild.id)).toBe(true);
    });

    it('returns empty { nodes: [], edges: [] } for unknown root', () => {
      expect(store.getSubgraph('ghost', 5)).toEqual({ nodes: [], edges: [] });
    });

    it('handles cycles without infinite loop', () => {
      store.addEdge(createEdge(grandchild.id, root.id));
      const sub = store.getSubgraph(root.id, 5);
      expect(sub.nodes.length).toBe(4);
    });

    it('returns deep copies of nodes', () => {
      const sub = store.getSubgraph(root.id, 1);
      sub.nodes[0].label = 'mutated';
      expect(store.getSubgraph(root.id, 1).nodes[0].label).toBe('root');
    });

    it('returns deep copies of edges', () => {
      const sub = store.getSubgraph(root.id, 1);
      sub.edges[0].weight = 999;
      expect(store.getSubgraph(root.id, 1).edges[0].weight).toBe(1);
    });

    it('includes incoming edges in subgraph expansion', () => {
      const incoming = store.addNode(createNode({ label: 'incoming' }));
      store.addEdge(createEdge(incoming.id, root.id));
      const sub = store.getSubgraph(root.id, 1);
      const nodeIds = new Set(sub.nodes.map(n => n.id));
      expect(nodeIds.has(incoming.id)).toBe(true);
    });
  });

  describe('nodeCount / edgeCount getters', () => {
    it('nodeCount returns 0 for empty store', () => {
      expect(new GraphStore({ autoLinkThreshold: 999 }).nodeCount).toBe(0);
    });

    it('edgeCount returns 0 for empty store', () => {
      expect(new GraphStore({ autoLinkThreshold: 999 }).edgeCount).toBe(0);
    });

    it('nodeCount reflects add and delete operations', () => {
      const store = new GraphStore({ autoLinkThreshold: 999 });
      const n1 = store.addNode(createNode());
      const n2 = store.addNode(createNode());
      expect(store.nodeCount).toBe(2);
      store.deleteNode(n1.id);
      expect(store.nodeCount).toBe(1);
      store.addNode(createNode());
      expect(store.nodeCount).toBe(2);
    });

    it('edgeCount reflects add and delete operations', () => {
      const store = new GraphStore({ autoLinkThreshold: 999 });
      const a = store.addNode(createNode({ label: 'A' }));
      const b = store.addNode(createNode({ label: 'B' }));
      const c = store.addNode(createNode({ label: 'C' }));
      const e1 = store.addEdge(createEdge(a.id, b.id))!;
      const e2 = store.addEdge(createEdge(b.id, c.id))!;
      expect(store.edgeCount).toBe(2);
      store.deleteEdge(e1.id);
      expect(store.edgeCount).toBe(1);
      store.deleteNode(b.id);
      expect(store.edgeCount).toBe(0);
    });
  });

  describe('init / save (file persistence)', () => {
    let filePath: string;
    let dirPath: string;

    beforeEach(() => {
      dirPath = fs.mkdtempSync(path.join(os.tmpdir(), 'graph-test-'));
      filePath = path.join(dirPath, 'graph.json');
    });

    afterEach(() => {
      fs.rmSync(dirPath, { recursive: true, force: true });
    });

    it('init creates directory if it does not exist', async () => {
      const nestedPath = path.join(dirPath, 'nested', 'deep', 'graph.json');
      const store = new GraphStore({ filePath: nestedPath });
      await store.init();
      expect(fs.existsSync(path.dirname(nestedPath))).toBe(true);
    });

    it('init loads nodes from file', async () => {
      const data = {
        nodes: [
          { id: 'n1', type: 'concept', label: 'loaded-node', properties: { p: 1 } },
        ],
        edges: [],
      };
      fs.writeFileSync(filePath, JSON.stringify(data));
      const store = new GraphStore({ filePath });
      await store.init();
      expect(store.nodeCount).toBe(1);
      const node = store.getNode('n1');
      expect(node!.label).toBe('loaded-node');
      expect(node!.properties).toEqual({ p: 1 });
    });

    it('init loads edges from file', async () => {
      const data = {
        nodes: [
          { id: 'n1', type: 'concept', label: 'N1', properties: {} },
          { id: 'n2', type: 'tool', label: 'N2', properties: {} },
        ],
        edges: [
          { id: 'e1', from: 'n1', to: 'n2', relation: 'links', weight: 0.5 },
        ],
      };
      fs.writeFileSync(filePath, JSON.stringify(data));
      const store = new GraphStore({ filePath });
      await store.init();
      expect(store.edgeCount).toBe(1);
      const edge = store.getEdge('e1');
      expect(edge!.relation).toBe('links');
      expect(edge!.weight).toBe(0.5);
    });

    it('save writes nodes and edges to file', async () => {
      const store = new GraphStore({ filePath, autoLinkThreshold: 999 });
      const a = store.addNode(createNode({ label: 'AA' }));
      const b = store.addNode(createNode({ label: 'BB' }));
      store.addEdge(createEdge(a.id, b.id, { relation: 'saved', weight: 7 }));
      await store.save();
      expect(fs.existsSync(filePath)).toBe(true);
      const raw = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
      expect(raw.nodes.length).toBe(2);
      expect(raw.edges.length).toBe(1);
      expect(raw.edges[0].relation).toBe('saved');
    });

    it('save creates directory if needed', async () => {
      const nestedPath = path.join(dirPath, 'sub', 'nested', 'g.json');
      const store = new GraphStore({ filePath: nestedPath });
      store.addNode(createNode());
      await store.save();
      expect(fs.existsSync(nestedPath)).toBe(true);
    });

    it('round-trip: save then init loads same data', async () => {
      const store1 = new GraphStore({ filePath });
      store1.addNode(createNode({ label: 'roundtrip', type: 'memory', properties: { x: 42 } }));
      const node = store1.addNode(createNode({ label: 'target', type: 'tool', properties: {} }));
      store1.addEdge(createEdge(node.id, store1.addNode(createNode({ label: 'd' })).id));
      await store1.save();

      const store2 = new GraphStore({ filePath });
      await store2.init();
      expect(store2.nodeCount).toBe(store1.nodeCount);
      expect(store2.edgeCount).toBe(store1.edgeCount);
      expect(store2.findNodes(n => n.label === 'roundtrip').length).toBe(1);
    });

    it('init handles missing file gracefully (creates empty store)', async () => {
      const store = new GraphStore({ filePath: path.join(dirPath, 'nope.json') });
      await store.init();
      expect(store.nodeCount).toBe(0);
      expect(store.edgeCount).toBe(0);
    });
  });

  describe('integration scenarios', () => {
    it('full CRUD cycle: add, read, update, delete node', () => {
      const store = new GraphStore({ autoLinkThreshold: 999 });
      const node = store.addNode(createNode({ label: 'crud-test' }));
      expect(store.getNode(node.id)!.label).toBe('crud-test');
      store.updateNode(node.id, { label: 'updated' });
      expect(store.getNode(node.id)!.label).toBe('updated');
      store.deleteNode(node.id);
      expect(store.getNode(node.id)).toBeUndefined();
    });

    it('graph traversal: build chain and walk via getNeighbors', () => {
      const store = new GraphStore({ autoLinkThreshold: 999 });
      const n1 = store.addNode(createNode({ label: 'start' }));
      const n2 = store.addNode(createNode({ label: 'middle' }));
      const n3 = store.addNode(createNode({ label: 'end' }));
      store.addEdge(createEdge(n1.id, n2.id, { relation: 'next' }));
      store.addEdge(createEdge(n2.id, n3.id, { relation: 'next' }));

      const neighbors = store.getNeighbors(n1.id, { direction: 'out', relation: 'next' });
      expect(neighbors.length).toBe(1);
      expect(neighbors[0].label).toBe('middle');

      const nextNeighbors = store.getNeighbors(neighbors[0].id, { direction: 'out', relation: 'next' });
      expect(nextNeighbors.length).toBe(1);
      expect(nextNeighbors[0].label).toBe('end');
    });

    it('graph analysis: shortest path through a weighted graph', () => {
      const store = new GraphStore({ autoLinkThreshold: 999 });
      const s = store.addNode(createNode({ label: 'S' }));
      const a = store.addNode(createNode({ label: 'A' }));
      const b = store.addNode(createNode({ label: 'B' }));
      const t = store.addNode(createNode({ label: 'T' }));
      store.addEdge(createEdge(s.id, a.id, { weight: 1 }));
      store.addEdge(createEdge(s.id, b.id, { weight: 100 }));
      store.addEdge(createEdge(a.id, t.id, { weight: 1 }));
      store.addEdge(createEdge(b.id, t.id, { weight: 1 }));

      const result = store.getShortestPath(s.id, t.id);
      expect(result!.path).toEqual([s.id, a.id, t.id]);
      expect(result!.distance).toBe(2);
    });
  });

  // --- setGraphDb & persistNode/persistEdge ---

  describe('setGraphDb and persistence', () => {
    afterEach(() => {
      setGraphDb(null);
    });

    it('setGraphDb enables node persistence to PostgreSQL', () => {
      const execute = vi.fn().mockResolvedValue(undefined);
      setGraphDb({ execute });

      const store = new GraphStore({ autoLinkThreshold: 999 });
      store.addNode(createNode({ label: 'persist-test' }));

      expect(execute).toHaveBeenCalledTimes(1);
    });

    it('setGraphDb enables edge persistence to PostgreSQL', () => {
      const execute = vi.fn().mockResolvedValue(undefined);
      setGraphDb({ execute });

      const store = new GraphStore({ autoLinkThreshold: 999 });
      const n1 = store.addNode(createNode({ label: 'from' }));
      const n2 = store.addNode(createNode({ label: 'to' }));
      execute.mockClear();

      store.addEdge(createEdge(n1.id, n2.id, { relation: 'linked', weight: 0.8 }));

      expect(execute).toHaveBeenCalledTimes(1);
    });

    it('persistNode is a no-op when drizzleDb is null', () => {
      setGraphDb(null);
      const store = new GraphStore({ autoLinkThreshold: 999 });
      expect(() => store.addNode(createNode({ label: 'no db' }))).not.toThrow();
    });

    it('persistNode catches and logs errors without throwing', async () => {
      const execute = vi.fn().mockRejectedValue(new Error('db error'));
      setGraphDb({ execute });

      const store = new GraphStore({ autoLinkThreshold: 999 });
      expect(() => store.addNode(createNode({ label: 'fail gracefully' }))).not.toThrow();

      await new Promise(r => setTimeout(r, 50));
      expect(execute).toHaveBeenCalledTimes(1);
    });

    it('persistEdge catches and logs errors without throwing', async () => {
      const execute = vi.fn()
        .mockResolvedValue(undefined)
        .mockRejectedValueOnce(new Error('node persist fail'))
        .mockRejectedValueOnce(new Error('edge persist fail'));
      setGraphDb({ execute });

      const store = new GraphStore({ autoLinkThreshold: 999 });
      const n1 = store.addNode(createNode({ label: 'a' }));
      const n2 = store.addNode(createNode({ label: 'b' }));
      execute.mockClear();
      execute.mockRejectedValue(new Error('edge fail'));

      expect(() => store.addEdge(createEdge(n1.id, n2.id))).not.toThrow();

      await new Promise(r => setTimeout(r, 50));
      expect(execute).toHaveBeenCalled();
    });

    it('auto-link triggers persistence for auto-generated edges', () => {
      const execute = vi.fn().mockResolvedValue(undefined);
      setGraphDb({ execute });

      const store = new GraphStore({ autoLinkThreshold: 0.1 });
      store.addNode(createNode({ label: 'machine learning model' }));
      execute.mockClear();

      store.addNode(createNode({ label: 'deep learning model training' }));

      const edges = store.findEdges(e => e.relation === 'similar');
      expect(edges.length).toBeGreaterThan(0);
      expect(execute).toHaveBeenCalled();
    });
  });
});
