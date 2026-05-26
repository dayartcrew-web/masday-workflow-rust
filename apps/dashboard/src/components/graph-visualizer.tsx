'use client';

import { useEffect, useRef } from 'react';
import * as d3 from 'd3';
import { useGraphStore } from '@/stores/graph-store';
import type { GraphNode, GraphEdge } from '@/lib/types';

interface GraphVisualizerProps {
  width?: number;
  height?: number;
}

type SimulationNode = GraphNode & d3.SimulationNodeDatum;
type SimulationLink = Omit<GraphEdge, 'source' | 'target'> & d3.SimulationLinkDatum<SimulationNode>;

export function GraphVisualizer({ width = 900, height = 600 }: GraphVisualizerProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const selectNode = useGraphStore((s) => s.selectNode);

  useEffect(() => {
    if (!svgRef.current || nodes.length === 0) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    // Defs for filters and gradients
    const defs = svg.append('defs');

    // Glow filter for nodes
    const glowFilter = defs.append('filter').attr('id', 'node-glow');
    glowFilter.append('feGaussianBlur').attr('stdDeviation', '4').attr('result', 'coloredBlur');
    const feMerge = glowFilter.append('feMerge');
    feMerge.append('feMergeNode').attr('in', 'coloredBlur');
    feMerge.append('feMergeNode').attr('in', 'SourceGraphic');

    // Stronger glow for selected nodes
    const strongGlow = defs.append('filter').attr('id', 'node-glow-strong');
    strongGlow.append('feGaussianBlur').attr('stdDeviation', '8').attr('result', 'coloredBlur');
    const feMergeStrong = strongGlow.append('feMerge');
    feMergeStrong.append('feMergeNode').attr('in', 'coloredBlur');
    feMergeStrong.append('feMergeNode').attr('in', 'SourceGraphic');

    // Blur filter backdrop effect (decorative)
    const blurFilter = defs.append('filter').attr('id', 'backdrop-blur');
    blurFilter.append('feGaussianBlur').attr('stdDeviation', '20');

    const g = svg.append('g');

    // Zoom behavior with touch support
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.3, 3])
      .touchable(true)
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });
    svg.call(zoom);

    // Prevent default touch behavior for better pan/zoom on mobile
    svgRef.current.addEventListener('touchmove', (e) => {
      if (e.touches.length > 1) {
        e.preventDefault();
      }
    }, { passive: false });

    // RAG Graph node colors
    const nodeColors: Record<string, string> = {
      query: '#22c55e',
      document: '#3b82f6',
      chunk: '#6366f1',
      entity: '#f59e0b',
      agent: '#818cf8',
      concept: '#3b82f6',
      memory: '#f59e0b',
      workflow: '#8b5cf6',
      task: '#ec4899',
    };

    // Prepare simulation data
    const simNodes: SimulationNode[] = nodes.map((n) => ({ ...n }));
    const simLinks = edges
      .map((e) => ({
        ...e,
        source: simNodes.find((n) => n.id === e.source) || e.source,
        target: simNodes.find((n) => n.id === e.target) || e.target,
      }))
      .filter((link) => typeof link.source === 'object' && typeof link.target === 'object') as SimulationLink[];

    // Force simulation
    const simulation = d3.forceSimulation<SimulationNode>(simNodes)
      .force('link', d3.forceLink<SimulationNode, SimulationLink>(simLinks).id((d) => d.id).distance(140))
      .force('charge', d3.forceManyBody().strength(-350))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(50));

    // Links with edge styles: solid (strong), dashed (weak), animated (active)
    const link = g.append('g')
      .selectAll('line')
      .data(simLinks)
      .join('line')
      .attr('stroke', (d) => {
        const edge = d as GraphEdge & { confidence?: number };
        if (edge.confidence && edge.confidence > 0.8) return 'var(--color-primary)';
        return 'var(--color-text-secondary)';
      })
      .attr('stroke-opacity', (d) => {
        const edge = d as GraphEdge & { strength?: string };
        if (edge.strength === 'weak') return 0.4;
        return 0.8;
      })
      .attr('stroke-width', 1.5)
      .attr('stroke-dasharray', (d) => {
        const edge = d as GraphEdge & { strength?: string };
        if (edge.strength === 'weak') return '6 4';
        return 'none';
      });

    // Node groups with touch-friendly drag
    const node = g.append('g')
      .selectAll<SVGGElement, SimulationNode>('g')
      .data(simNodes)
      .join('g')
      .style('cursor', 'pointer')
      .call(d3.drag<SVGGElement, SimulationNode>()
        .touchable(true)
        .on('start', (event, d) => {
          if (!event.active) simulation.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
        })
        .on('drag', (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on('end', (event, d) => {
          if (!event.active) simulation.alphaTarget(0);
          d.fx = null;
          d.fy = null;
        }),
      );

    // Outer glow ring for each node
    node.append('circle')
      .attr('r', 26)
      .attr('fill', 'none')
      .attr('stroke', (d) => nodeColors[d.type] || '#64748b')
      .attr('stroke-opacity', 0.2)
      .attr('stroke-width', 2);

    // Main node circle with neon glow and blur backdrop
    node.append('circle')
      .attr('r', 20)
      .attr('fill', (d) => nodeColors[d.type] || '#64748b')
      .attr('fill-opacity', 0.25)
      .attr('stroke', (d) => nodeColors[d.type] || '#64748b')
      .attr('stroke-width', 1.5)
      .attr('stroke-opacity', 0.9)
      .attr('filter', 'url(#node-glow)')
      .attr('rx', 999);

    // Inner bright core
    node.append('circle')
      .attr('r', 8)
      .attr('fill', (d) => nodeColors[d.type] || '#64748b')
      .attr('fill-opacity', 0.8);

    // Node labels
    node.append('text')
      .text((d) => {
        const label = d.label ?? '';
        return label.length > 14 ? label.slice(0, 14) + '...' : label;
      })
      .attr('text-anchor', 'middle')
      .attr('dy', 40)
      .attr('fill', 'var(--color-text)')
      .attr('font-size', '11px')
      .attr('font-weight', '500');

    // Node type sublabel
    node.append('text')
      .text((d) => d.type)
      .attr('text-anchor', 'middle')
      .attr('dy', 52)
      .attr('fill', (d) => nodeColors[d.type] || '#64748b')
      .attr('font-size', '9px')
      .attr('font-weight', '400')
      .attr('opacity', 0.7);

    node.on('click', (_event, d) => {
      selectNode(d);
    });

    simulation.on('tick', () => {
      link
        .attr('x1', (d: unknown) => ((d as { source: { x: number } }).source.x))
        .attr('y1', (d: unknown) => ((d as { source: { y: number } }).source.y))
        .attr('x2', (d: unknown) => ((d as { target: { x: number } }).target.x))
        .attr('y2', (d: unknown) => ((d as { target: { y: number } }).target.y));

      node.attr('transform', (d) => {
        const x = (d as unknown as { x: number }).x || 0;
        const y = (d as unknown as { y: number }).y || 0;
        return `translate(${x},${y})`;
      });
    });

    return () => {
      simulation.stop();
    };
  }, [nodes, edges, width, height, selectNode]);

  if (nodes.length === 0) {
    return (
      <div className="glass-surface flex items-center justify-center h-64 md:h-96 text-[var(--color-text-secondary)] text-body">
        No graph data available. Connect to the API to load the knowledge graph.
      </div>
    );
  }

  return (
    <div ref={containerRef} className="glass-surface glow-card overflow-hidden touch-pan-x touch-pan-y">
      <svg ref={svgRef} width={width} height={height} className="w-full" style={{ background: 'var(--color-surface)', borderRadius: 'var(--radius-lg)' }} />
    </div>
  );
}
