import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useGraphStore } from '@/stores/graph-store';

describe('useGraphStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useGraphStore.setState({
      nodes: [],
      edges: [],
      selectedNode: null,
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with empty graph', () => {
    const state = useGraphStore.getState();
    expect(state.nodes).toEqual([]);
    expect(state.edges).toEqual([]);
    expect(state.selectedNode).toBeNull();
  });

  it('setNodes updates the node list', () => {
    const nodes = [
      { id: '1', label: 'Node 1', type: 'task' },
      { id: '2', label: 'Node 2', type: 'task' },
    ];

    useGraphStore.getState().setNodes(nodes);

    expect(useGraphStore.getState().nodes).toEqual(nodes);
  });

  it('setEdges updates the edge list', () => {
    const edges = [
      { id: 'e1', source: '1', target: '2' },
    ];

    useGraphStore.getState().setEdges(edges);

    expect(useGraphStore.getState().edges).toEqual(edges);
  });

  it('selectNode updates selected node', () => {
    const node = { id: '1', label: 'Selected' };
    useGraphStore.getState().selectNode(node);

    expect(useGraphStore.getState().selectedNode).toEqual(node);
  });

  it('clearGraph resets all graph data', () => {
    useGraphStore.setState({
      nodes: [{ id: '1', label: 'Node' }],
      edges: [{ id: 'e1', source: '1', target: '2' }],
      selectedNode: { id: '1', label: 'Node' },
    });

    useGraphStore.getState().clearGraph();

    const state = useGraphStore.getState();
    expect(state.nodes).toEqual([]);
    expect(state.edges).toEqual([]);
    expect(state.selectedNode).toBeNull();
  });

  it('addNode appends to existing nodes', () => {
    useGraphStore.setState({ nodes: [{ id: '1', label: 'Existing' }] });

    useGraphStore.getState().addNode({ id: '2', label: 'New' });

    expect(useGraphStore.getState().nodes).toHaveLength(2);
    expect(useGraphStore.getState().nodes[1]).toEqual({ id: '2', label: 'New' });
  });

  it('addEdge appends to existing edges', () => {
    useGraphStore.setState({ edges: [{ id: 'e1', source: '1', target: '2' }] });

    useGraphStore.getState().addEdge({ id: 'e2', source: '2', target: '3' });

    expect(useGraphStore.getState().edges).toHaveLength(2);
    expect(useGraphStore.getState().edges[1]).toEqual({ id: 'e2', source: '2', target: '3' });
  });

  it('clearError resets error', () => {
    useGraphStore.setState({ error: 'Some error' });

    useGraphStore.getState().clearError();

    expect(useGraphStore.getState().error).toBeNull();
  });
});
