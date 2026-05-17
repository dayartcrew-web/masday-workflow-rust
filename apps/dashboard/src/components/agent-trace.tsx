'use client';

import { useWebSocketStore } from '@/stores/websocket-store';
import { useEffect, useState } from 'react';
import type { ReActStep } from '@/lib/types';

interface AgentTraceProps {
  steps: ReActStep[];
  isRunning: boolean;
}

export function AgentTrace({ steps, isRunning }: AgentTraceProps) {
  const latestEvent = useWebSocketStore((s) => s.latestEvent);

  if (steps.length === 0 && !isRunning) {
    return (
      <div className="text-center py-8 text-[var(--text-secondary)]">
        Start a ReAct agent to see the live trace
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {isRunning && (
        <div className="flex items-center gap-2 text-sm text-brand-400">
          <span className="w-2 h-2 rounded-full bg-brand-500 animate-pulse" />
          Agent running...
        </div>
      )}

      {steps.map((step, idx) => (
        <div key={idx} className="rounded-lg border border-[var(--border)] bg-[var(--bg-card)] p-3 space-y-2">
          <div className="flex items-center gap-2 text-xs">
            <span className="px-2 py-0.5 rounded bg-brand-600/10 text-brand-400 font-medium">
              Step {step.iteration}
            </span>
          </div>

          {/* Thought */}
          <div>
            <span className="text-xs font-medium text-purple-400">Thought</span>
            <p className="text-sm text-[var(--text-primary)] mt-0.5">{step.thought}</p>
          </div>

          {/* Action */}
          <div>
            <span className="text-xs font-medium text-blue-400">Action</span>
            <p className="text-sm text-[var(--text-primary)] mt-0.5 font-mono text-xs bg-[var(--bg-secondary)] p-2 rounded">
              {step.action}
            </p>
          </div>

          {/* Observation */}
          <div>
            <span className="text-xs font-medium text-emerald-400">Observation</span>
            <p className="text-sm text-[var(--text-primary)] mt-0.5">{step.observation}</p>
          </div>
        </div>
      ))}

      {latestEvent && latestEvent.type.startsWith('agent.') && (
        <div className="text-xs text-[var(--text-secondary)] text-center">
          Latest event: {latestEvent.type}
        </div>
      )}
    </div>
  );
}
