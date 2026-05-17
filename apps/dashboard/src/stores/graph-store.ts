// ============================================================
// Graph Store — knowledge graph nodes and edges
// ============================================================

import { create } from 'zustand';
import type { GraphNode, GraphEdge } from '@/lib/types';

interface GraphState {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNode: GraphNode | null;
  isLoading: boolean;
  error: string | null;
  setNodes: (nodes: GraphNode[]) => void;
  setEdges: (edges: GraphEdge[]) => void;
  addNode: (node: GraphNode) => void;
  addEdge: (edge: GraphEdge) => void;
  selectNode: (node: GraphNode | null) => void;
  clearGraph: () => void;
  clearError: () => void;
}

export const useGraphStore = create<GraphState>((set) => ({
  nodes: [],
  edges: [],
  selectedNode: null,
  isLoading: false,
  error: null,

  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),

  addNode: (node) => set((s) => ({ nodes: [...s.nodes, node] })),
  addEdge: (edge) => set((s) => ({ edges: [...s.edges, edge] })),

  selectNode: (node) => set({ selectedNode: node }),
  clearGraph: () => set({ nodes: [], edges: [], selectedNode: null }),
  clearError: () => set({ error: null }),
}));
