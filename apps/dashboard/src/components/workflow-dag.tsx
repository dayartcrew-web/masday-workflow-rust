'use client';

import { useMemo } from 'react';
import type { Task } from '@/lib/types';

interface WorkflowDagProps {
  tasks: Task[];
  onTaskClick?: (task: Task) => void;
}

const STATE_COLORS: Record<string, string> = {
  pending: '#475569',
  running: '#6366f1',
  done: '#22c55e',
  failed: '#ef4444',
  skipped: '#f59e0b',
  blocked: '#dc2626',
};

function buildPositions(tasks: Task[]): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  const deps = new Map<string, number>();

  // Compute depth for each task
  function getDepth(task: Task): number {
    if (deps.has(task.id)) return deps.get(task.id)!;
    if (!task.dependencies || task.dependencies.length === 0) {
      deps.set(task.id, 0);
      return 0;
    }
    const maxDep = Math.max(
      ...task.dependencies.map((depId) => {
        const dep = tasks.find((t) => t.id === depId);
        return dep ? getDepth(dep) : 0;
      }),
    );
    const depth = maxDep + 1;
    deps.set(task.id, depth);
    return depth;
  }

  tasks.forEach((t) => getDepth(t));

  // Group by depth level
  const levels = new Map<number, Task[]>();
  tasks.forEach((t) => {
    const depth = deps.get(t.id) || 0;
    if (!levels.has(depth)) levels.set(depth, []);
    levels.get(depth)!.push(t);
  });

  const xSpacing = 180;
  const ySpacing = 80;
  const maxX = 800;
  let globalY = 40;

  const sortedLevels = Array.from(levels.entries()).sort(([a], [b]) => a - b);
  for (const [, levelTasks] of sortedLevels) {
    const totalWidth = (levelTasks.length - 1) * xSpacing;
    const startX = Math.max(40, (maxX - totalWidth) / 2);
    levelTasks.forEach((t: Task, i: number) => {
      positions.set(t.id, { x: startX + i * xSpacing, y: globalY });
    });
    globalY += ySpacing;
  }

  return positions;
}

export function WorkflowDag({ tasks, onTaskClick }: WorkflowDagProps) {
  const positions = useMemo(() => buildPositions(tasks), [tasks]);

  if (tasks.length === 0) {
    return (
      <div className="glass-surface flex items-center justify-center h-64 text-[var(--color-text-secondary)] text-body">
        No tasks to display
      </div>
    );
  }

  const allX = Array.from(positions.values()).map((p) => p.x);
  const allY = Array.from(positions.values()).map((p) => p.y);
  const width = Math.max(400, Math.max(...allX) + 140);
  const height = Math.max(200, Math.max(...allY) + 80);

  return (
    <svg width="100%" viewBox={`0 0 ${width} ${height}`} className="glass-surface overflow-hidden" style={{ background: 'var(--color-surface)' }}>
      {/* SVG Defs: arrowhead marker + glow filter + pulse animation */}
      <defs>
        <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
          <polygon points="0 0, 8 3, 0 6" fill="var(--color-text-secondary)" opacity="0.5" />
        </marker>
        <filter id="dag-node-glow">
          <feGaussianBlur stdDeviation="3" result="coloredBlur" />
          <feMerge>
            <feMergeNode in="coloredBlur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter id="dag-node-glow-strong">
          <feGaussianBlur stdDeviation="6" result="coloredBlur" />
          <feMerge>
            <feMergeNode in="coloredBlur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        {/* Pulse animation for running tasks */}
        <style>{`
          @keyframes dag-pulse {
            0%, 100% { opacity: 0.15; }
            50% { opacity: 0.35; }
          }
          .dag-running-bg {
            animation: dag-pulse 2s ease-in-out infinite;
          }
        `}</style>
      </defs>
      {/* Edges - subtle gray lines */}
      {tasks.flatMap((task) =>
        (task.dependencies || []).map((depId) => {
          const from = positions.get(depId);
          const to = positions.get(task.id);
          if (!from || !to) return null;
          return (
            <line
              key={`${depId}-${task.id}`}
              x1={from.x + 50}
              y1={from.y + 20}
              x2={to.x + 50}
              y2={to.y}
              stroke="var(--color-text-secondary)"
              strokeOpacity={0.2}
              strokeWidth={1}
              markerEnd="url(#arrowhead)"
            />
          );
        }),
      )}
      {/* Nodes */}
      {tasks.map((task) => {
        const pos = positions.get(task.id);
        if (!pos) return null;
        const color = STATE_COLORS[task.state] || STATE_COLORS.pending;
        const isRunning = task.state === 'running';
        const isDone = task.state === 'done';
        const isFailed = task.state === 'failed';
        return (
          <g
            key={task.id}
            onClick={() => onTaskClick?.(task)}
            className={onTaskClick ? 'cursor-pointer' : ''}
            filter={isRunning ? 'url(#dag-node-glow)' : undefined}
          >
            {/* Background fill */}
            <rect
              x={pos.x}
              y={pos.y}
              width={100}
              height={40}
              rx={8}
              fill={color}
              opacity={isRunning ? 0.15 : 0.08}
              className={isRunning ? 'dag-running-bg' : undefined}
            />
            {/* Colored border */}
            <rect
              x={pos.x}
              y={pos.y}
              width={100}
              height={40}
              rx={8}
              fill="none"
              stroke={color}
              strokeWidth={isRunning ? 2 : 1.5}
              strokeOpacity={isDone ? 0.8 : isFailed ? 0.9 : 0.6}
            />
            {/* Task name */}
            <text
              x={pos.x + 50}
              y={pos.y + 16}
              textAnchor="middle"
              fill="var(--color-text)"
              fontSize="10px"
              fontWeight="500"
            >
              {task.name.length > 12 ? task.name.slice(0, 12) + '...' : task.name}
            </text>
            {/* Task state */}
            <text
              x={pos.x + 50}
              y={pos.y + 30}
              textAnchor="middle"
              fill={color}
              fontSize="9px"
              fontWeight="500"
            >
              {task.state}
            </text>
            {/* Status indicator dot */}
            <circle
              cx={pos.x + 8}
              cy={pos.y + 8}
              r={3}
              fill={color}
              opacity={0.9}
            />
          </g>
        );
      })}
      {/* Legend */}
      <g transform={`translate(10, ${height - 25})`}>
        {Object.entries(STATE_COLORS).map(([state, color], i) => (
          <g key={state} transform={`translate(${i * 85}, 0)`}>
            <rect width={10} height={10} rx={3} fill={color} opacity={0.6} />
            <text x={14} y={9} fill="var(--color-text-secondary)" fontSize="9px">{state}</text>
          </g>
        ))}
      </g>
    </svg>
  );
}
