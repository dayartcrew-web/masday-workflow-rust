'use client';

import { useEffect } from 'react';
import { AppShell } from '@/components/app-shell';
import { GraphVisualizer } from '@/components/graph-visualizer';
import { useGraphStore } from '@/stores/graph-store';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useAuthStore } from '@/stores/auth-store';
import type { GraphNode, GraphEdge } from '@/lib/types';

export default function GraphPage() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const selectedNode = useGraphStore((s) => s.selectedNode);
  const setNodes = useGraphStore((s) => s.setNodes);
  const setEdges = useGraphStore((s) => s.setEdges);
  const clearGraph = useGraphStore((s) => s.clearGraph);
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchWorkflows();
  }, [isAuthenticated, fetchWorkflows]);

  // Generate sample graph data from workflows when available
  useEffect(() => {
    if (workflows.length > 0 && nodes.length === 0) {
      const graphNodes: GraphNode[] = [];
      const graphEdges: GraphEdge[] = [];

      workflows.forEach((w) => {
        graphNodes.push({
          id: `wf-${w.id}`,
          label: w.name,
          type: 'workflow',
          properties: { state: w.state },
        });

        (w.tasks || []).forEach((t) => {
          graphNodes.push({
            id: `task-${t.id}`,
            label: t.name,
            type: 'task',
            properties: { state: t.state, agent: t.agent },
          });
          graphEdges.push({
            id: `edge-${w.id}-${t.id}`,
            source: `wf-${w.id}`,
            target: `task-${t.id}`,
            type: 'contains',
            weight: 1,
          });
        });
      });

      setNodes(graphNodes);
      setEdges(graphEdges);
    }
  }, [workflows, nodes.length, setNodes, setEdges]);

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Knowledge Graph</h2>
          <div className="flex items-center gap-2">
            <span className="text-xs text-[var(--text-secondary)]">
              {nodes.length} nodes, {edges.length} edges
            </span>
            <button
              onClick={clearGraph}
              className="px-3 py-1.5 rounded-lg text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-card)] transition-colors"
            >
              Clear
            </button>
          </div>
        </div>

        {/* Graph type legend */}
        <div className="flex gap-4 text-xs text-[var(--text-secondary)]">
          <div className="flex items-center gap-1"><span className="w-3 h-3 rounded-full bg-blue-500" /> Workflow</div>
          <div className="flex items-center gap-1"><span className="w-3 h-3 rounded-full bg-pink-500" /> Task</div>
          <div className="flex items-center gap-1"><span className="w-3 h-3 rounded-full bg-green-500" /> Entity</div>
          <div className="flex items-center gap-1"><span className="w-3 h-3 rounded-full bg-yellow-500" /> Memory</div>
        </div>

        {/* Graph visualization */}
        <GraphVisualizer width={900} height={600} />

        {/* Selected node detail */}
        {selectedNode && (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
            <h3 className="font-medium text-[var(--text-primary)] mb-2">{selectedNode.label}</h3>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div><span className="text-[var(--text-secondary)]">Type:</span> <span className="ml-1">{selectedNode.type}</span></div>
              <div><span className="text-[var(--text-secondary)]">ID:</span> <span className="ml-1 text-xs font-mono">{selectedNode.id}</span></div>
              {Object.entries(selectedNode.properties).map(([key, val]) => (
                <div key={key}>
                  <span className="text-[var(--text-secondary)]">{key}:</span> <span className="ml-1">{String(val)}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </AppShell>
  );
}
