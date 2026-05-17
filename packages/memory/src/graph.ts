import fs from 'fs';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import type { GraphNodeRecord, GraphEdgeRecord } from '@mcp-rebuild/core';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('memory:graph');

export interface GraphStoreConfig {
  filePath?: string;
  autoLinkThreshold?: number;
}

interface PathResult {
  path: string[];
  distance: number;
}

/**
 * GraphStore - knowledge graph with Dijkstra, BFS, and auto-linking.
 *
 * Supports:
 * - Node and edge CRUD
 * - Dijkstra's shortest path algorithm
 * - BFS subgraph expansion
 * - Auto-linking nodes via Jaccard similarity
 * - File persistence (JSON)
 */
export class GraphStore {
  private nodes: Map<string, GraphNodeRecord> = new Map();
  private edges: Map<string, GraphEdgeRecord> = new Map();
  private readonly filePath: string | null;
  private readonly autoLinkThreshold: number;

  constructor(config?: GraphStoreConfig) {
    this.filePath = config?.filePath ?? null;
    this.autoLinkThreshold = config?.autoLinkThreshold ?? 0.4;
  }

  /** Initialize from file if configured. */
  async init(): Promise<void> {
    if (!this.filePath) return;

    try {
      if (fs.existsSync(this.filePath)) {
        const data = JSON.parse(fs.readFileSync(this.filePath, 'utf-8')) as {
          nodes: GraphNodeRecord[];
          edges: GraphEdgeRecord[];
        };

        for (const node of data.nodes) {
          this.nodes.set(node.id, node);
        }
        for (const edge of data.edges) {
          this.edges.set(edge.id, edge);
        }
        logger.info({ nodes: this.nodes.size, edges: this.edges.size }, 'Loaded graph from file');
      } else {
        const dir = path.dirname(this.filePath);
        if (!fs.existsSync(dir)) {
          fs.mkdirSync(dir, { recursive: true });
        }
        logger.info('Initialized empty graph store');
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.error({ error: message }, 'Failed to initialize graph store');
      throw new Error(`Graph store initialization failed: ${message}`);
    }
  }

  /** Save current state to file. */
  async save(): Promise<void> {
    if (!this.filePath) return;

    try {
      const dir = path.dirname(this.filePath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      const data = {
        nodes: Array.from(this.nodes.values()),
        edges: Array.from(this.edges.values()),
      };
      fs.writeFileSync(this.filePath, JSON.stringify(data, null, 2), 'utf-8');
      logger.debug({ nodes: data.nodes.length, edges: data.edges.length }, 'Saved graph');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.error({ error: message }, 'Failed to save graph store');
      throw new Error(`Graph store save failed: ${message}`);
    }
  }

  // --- Node Operations ---

  /** Add a node to the graph. */
  addNode(node: Omit<GraphNodeRecord, 'id'>): GraphNodeRecord {
    const record: GraphNodeRecord = {
      id: uuidv4(),
      type: node.type,
      label: node.label,
      properties: { ...node.properties },
    };
    this.nodes.set(record.id, record);
    logger.debug({ id: record.id, label: record.label }, 'Added node');

    // Auto-link with similar nodes
    this.autoLink(record);

    return { ...record };
  }

  /** Get a node by ID. */
  getNode(id: string): GraphNodeRecord | undefined {
    const node = this.nodes.get(id);
    return node ? { ...node, properties: { ...node.properties } } : undefined;
  }

  /** Update a node. */
  updateNode(id: string, updates: Partial<Pick<GraphNodeRecord, 'label' | 'properties'>>): GraphNodeRecord | undefined {
    const current = this.nodes.get(id);
    if (!current) return undefined;

    const updated: GraphNodeRecord = {
      ...current,
      label: updates.label ?? current.label,
      properties: updates.properties ? { ...updates.properties } : { ...current.properties },
    };
    this.nodes.set(id, updated);
    return { ...updated, properties: { ...updated.properties } };
  }

  /** Delete a node and all connected edges. */
  deleteNode(id: string): boolean {
    // Remove connected edges
    const edgesToRemove: string[] = [];
    for (const [edgeId, edge] of this.edges) {
      if (edge.from === id || edge.to === id) {
        edgesToRemove.push(edgeId);
      }
    }
    for (const edgeId of edgesToRemove) {
      this.edges.delete(edgeId);
    }

    const existed = this.nodes.delete(id);
    if (existed) {
      logger.debug({ id, removedEdges: edgesToRemove.length }, 'Deleted node and connected edges');
    }
    return existed;
  }

  /** Find nodes matching a predicate. */
  findNodes(predicate: (node: GraphNodeRecord) => boolean): GraphNodeRecord[] {
    return Array.from(this.nodes.values())
      .filter(predicate)
      .map(n => ({ ...n, properties: { ...n.properties } }));
  }

  // --- Edge Operations ---

  /** Add an edge between two nodes. */
  addEdge(edge: Omit<GraphEdgeRecord, 'id'>): GraphEdgeRecord | undefined {
    if (!this.nodes.has(edge.from) || !this.nodes.has(edge.to)) {
      logger.warn({ from: edge.from, to: edge.to }, 'Cannot add edge: node not found');
      return undefined;
    }

    const record: GraphEdgeRecord = {
      id: uuidv4(),
      from: edge.from,
      to: edge.to,
      relation: edge.relation,
      weight: edge.weight,
    };
    this.edges.set(record.id, record);
    logger.debug({ id: record.id, relation: record.relation }, 'Added edge');
    return { ...record };
  }

  /** Get an edge by ID. */
  getEdge(id: string): GraphEdgeRecord | undefined {
    const edge = this.edges.get(id);
    return edge ? { ...edge } : undefined;
  }

  /** Delete an edge. */
  deleteEdge(id: string): boolean {
    return this.edges.delete(id);
  }

  /** Find edges matching criteria. */
  findEdges(predicate: (edge: GraphEdgeRecord) => boolean): GraphEdgeRecord[] {
    return Array.from(this.edges.values())
      .filter(predicate)
      .map(e => ({ ...e }));
  }

  /** Get neighboring nodes with optional relation and direction filters. */
  getNeighbors(nodeId: string, options?: { relation?: string; direction?: 'in' | 'out' | 'both' }): GraphNodeRecord[] {
    const direction = options?.direction ?? 'both';
    const neighborIds = new Set<string>();

    for (const edge of this.edges.values()) {
      if (options?.relation && edge.relation !== options.relation) continue;

      if ((direction === 'out' || direction === 'both') && edge.from === nodeId) {
        neighborIds.add(edge.to);
      }
      if ((direction === 'in' || direction === 'both') && edge.to === nodeId) {
        neighborIds.add(edge.from);
      }
    }

    const neighbors: GraphNodeRecord[] = [];
    for (const id of neighborIds) {
      const node = this.nodes.get(id);
      if (node) {
        neighbors.push({ ...node, properties: { ...node.properties } });
      }
    }

    return neighbors;
  }

  /** Find shortest path using Dijkstra's algorithm. */
  getShortestPath(fromId: string, toId: string): PathResult | undefined {
    if (!this.nodes.has(fromId) || !this.nodes.has(toId)) {
      return undefined;
    }

    if (fromId === toId) {
      return { path: [fromId], distance: 0 };
    }

    // Build adjacency list
    const adj = new Map<string, Array<{ to: string; weight: number }>>();
    for (const node of this.nodes.keys()) {
      adj.set(node, []);
    }
    for (const edge of this.edges.values()) {
      const list = adj.get(edge.from);
      if (list) {
        list.push({ to: edge.to, weight: edge.weight });
      }
    }

    // Dijkstra
    const distances = new Map<string, number>();
    const previous = new Map<string, string>();
    const visited = new Set<string>();

    for (const nodeId of this.nodes.keys()) {
      distances.set(nodeId, Infinity);
    }
    distances.set(fromId, 0);

    while (true) {
      let current: string | null = null;
      let minDist = Infinity;

      for (const [nodeId, dist] of distances) {
        if (!visited.has(nodeId) && dist < minDist) {
          current = nodeId;
          minDist = dist;
        }
      }

      if (current === null || current === toId) break;
      visited.add(current);

      const neighbors = adj.get(current) ?? [];
      for (const neighbor of neighbors) {
        if (visited.has(neighbor.to)) continue;

        const newDist = distances.get(current)! + neighbor.weight;
        if (newDist < distances.get(neighbor.to)!) {
          distances.set(neighbor.to, newDist);
          previous.set(neighbor.to, current);
        }
      }
    }

    const finalDist = distances.get(toId)!;
    if (finalDist === Infinity) return undefined;

    // Reconstruct path
    const pathNodes: string[] = [];
    let current: string | undefined = toId;
    while (current !== undefined) {
      pathNodes.unshift(current);
      current = previous.get(current);
    }

    return { path: pathNodes, distance: finalDist };
  }

  /** Get subgraph by BFS expansion from a root node. */
  getSubgraph(rootId: string, depth: number): { nodes: GraphNodeRecord[]; edges: GraphEdgeRecord[] } {
    if (!this.nodes.has(rootId)) {
      return { nodes: [], edges: [] };
    }

    const visitedNodes = new Set<string>();
    const visitedEdges = new Set<string>();
    const resultNodes: GraphNodeRecord[] = [];
    const resultEdges: GraphEdgeRecord[] = [];

    let frontier = new Set<string>([rootId]);

    for (let d = 0; d <= depth; d++) {
      const nextFrontier = new Set<string>();

      for (const nodeId of frontier) {
        if (visitedNodes.has(nodeId)) continue;
        visitedNodes.add(nodeId);

        const node = this.nodes.get(nodeId);
        if (node) {
          resultNodes.push({ ...node, properties: { ...node.properties } });
        }

        if (d < depth) {
          for (const edge of this.edges.values()) {
            if (edge.from === nodeId && !visitedNodes.has(edge.to)) {
              if (!visitedEdges.has(edge.id)) {
                visitedEdges.add(edge.id);
                resultEdges.push({ ...edge });
              }
              nextFrontier.add(edge.to);
            }
            if (edge.to === nodeId && !visitedNodes.has(edge.from)) {
              if (!visitedEdges.has(edge.id)) {
                visitedEdges.add(edge.id);
                resultEdges.push({ ...edge });
              }
              nextFrontier.add(edge.from);
            }
          }
        }
      }

      frontier = nextFrontier;
    }

    return { nodes: resultNodes, edges: resultEdges };
  }

  /** Get counts. */
  get nodeCount(): number {
    return this.nodes.size;
  }

  get edgeCount(): number {
    return this.edges.size;
  }

  /** Auto-link a new node with similar existing nodes using Jaccard similarity. */
  private autoLink(newNode: GraphNodeRecord): void {
    const threshold = this.autoLinkThreshold;

    for (const [existingId, existingNode] of this.nodes) {
      if (existingId === newNode.id) continue;

      const similarity = this.nodeJaccardSimilarity(newNode, existingNode);
      if (similarity >= threshold) {
        this.addEdge({
          from: newNode.id,
          to: existingId,
          relation: 'similar',
          weight: similarity,
        });

        this.addEdge({
          from: existingId,
          to: newNode.id,
          relation: 'similar',
          weight: similarity,
        });

        logger.debug(
          { from: newNode.id, to: existingId, similarity },
          'Auto-linked similar nodes'
        );
      }
    }
  }

  /** Compute Jaccard similarity between two nodes based on label tokens and properties. */
  private nodeJaccardSimilarity(a: GraphNodeRecord, b: GraphNodeRecord): number {
    const tokenize = (text: string): Set<string> =>
      new Set(
        text
          .toLowerCase()
          .split(/\W+/)
          .filter(t => t.length > 1)
      );

    const setA = tokenize(a.label);
    const setB = tokenize(b.label);

    // Include property string tokens
    for (const val of Object.values(a.properties)) {
      if (typeof val === 'string') tokenize(val).forEach(t => setA.add(t));
    }
    for (const val of Object.values(b.properties)) {
      if (typeof val === 'string') tokenize(val).forEach(t => setB.add(t));
    }

    if (setA.size === 0 && setB.size === 0) return 0;

    let intersection = 0;
    for (const token of setA) {
      if (setB.has(token)) intersection++;
    }

    const union = setA.size + setB.size - intersection;
    return union === 0 ? 0 : intersection / union;
  }
}
