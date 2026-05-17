import { EventBus } from '@mcp-rebuild/core';
import type { IndexedRepository, CodeGraph, DependencyEdge, FileMetadata } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('CodeGraphAnalyzer');

export class CodeGraphAnalyzer {
  private eventBus: EventBus;
  private indexedRepo: IndexedRepository | null = null;

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
  }

  setIndexedRepository(repo: IndexedRepository): void {
    this.indexedRepo = repo;
    logger.info('Indexed repository set for graph analysis');
  }

  async analyze(repositoryPath?: string): Promise<CodeGraph> {
    if (!this.indexedRepo) {
      throw new Error('No indexed repository available');
    }

    logger.info('Analyzing code graph...');

    const nodes = new Map<string, FileMetadata>();
    const edges = new Map<string, DependencyEdge[]>();

    // Analyze dependencies
    for (const [filePath, depEdges] of this.indexedRepo.dependencies) {
      for (const edge of depEdges) {
        if (!nodes.has(edge.to)) {
          const fromNode = this.indexedRepo.files.get(edge.from);
          if (fromNode) {
            nodes.set(edge.to, fromNode);
          }
        }
      }
    }

    // Build entry points (files with no incoming edges)
    const allFilePaths = Array.from(this.indexedRepo.files.keys());
    const targetSet = new Set<string>(Array.from(edges.values()).flatMap(e => e.map(edge => edge.to)));
    const entryPoints = allFilePaths.filter(path => !targetSet.has(path));

    const graph: CodeGraph = {
      nodes,
      edges,
      entryPoints,
    };

    logger.info(`Graph analysis complete: ${nodes.size} nodes, ${edges.size} edges, ${entryPoints.length} entry points`);

    this.eventBus.emit('graph.analyzed', {
      nodeCount: nodes.size,
      edgeCount: edges.size,
      entryPointsCount: entryPoints.length,
    });

    void repositoryPath;
    return graph;
  }

  getCriticalPath(graph: CodeGraph): string[] {
    const visited = new Set<string>();
    const path: string[] = [];
    let maxDistance = 0;

    // Simple BFS for critical path
    const queue = [...graph.entryPoints];
    const predecessors = new Map<string, string[]>();

    for (const entryPoint of graph.entryPoints) {
      predecessors.set(entryPoint, []);
      queue.push(entryPoint);
    }

    while (queue.length > 0) {
      const current = queue.shift()!;
      if (visited.has(current)) continue;

      visited.add(current);
      path.push(current);

      const outgoingEdges = graph.edges.get(current) || [];

      for (const edge of outgoingEdges) {
        const predList = predecessors.get(edge.to) || [];
        predList.push(current);

        if (!predList.includes(edge.from)) {
          predecessors.set(edge.to, [...predList, edge.from]);
        }
      }

      const nextNodes = outgoingEdges.map(e => e.to);
      for (const node of nextNodes) {
        if (!visited.has(node) && queue.length < 100) { // Limit queue size
          queue.push(node);
        }
      }

      // Update max distance if tracking weighted paths
      maxDistance = Math.max(maxDistance, path.length);
    }

    void maxDistance;
    return path;
  }

  findCircularDependencies(graph: CodeGraph): string[][] {
    const visited = new Set<string>();
    const path: string[] = [];
    const cycles: string[][] = [];

    const dfs = (node: string, currentPath: string[] = []) => {
      if (visited.has(node)) return;

      if (currentPath.includes(node)) {
        cycles.push([...currentPath]);
        return;
      }

      visited.add(node);
      currentPath.push(node);

      const outgoingEdges = graph.edges.get(node) || [];
      for (const edge of outgoingEdges) {
        dfs(edge.to, [...currentPath]);
      }

      currentPath.pop();
    };

    for (const startNode of graph.entryPoints) {
      dfs(startNode, []);
    }

    void path;
    return cycles;
  }

  getDependenciesForNode(node: string, graph: CodeGraph): Set<string> {
    const deps = new Set<string>();
    const incomingEdges = Array.from(graph.edges.entries())
      .filter(([_, edges]) => edges.some(e => e.to === node))
      .flatMap(([_, edges]) => edges.map(e => e.from));

    for (const dep of incomingEdges) {
      deps.add(dep);
    }

    return deps;
  }
}
