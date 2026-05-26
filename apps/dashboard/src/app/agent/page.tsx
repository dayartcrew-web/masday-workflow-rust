'use client';

import { useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { AgentTrace } from '@/components/agent-trace';
import { useWebSocketStore } from '@/stores/websocket-store';
import { chatApi } from '@/lib/api-client';
import type { ReActStep } from '@/lib/types';
import { Play, RotateCcw } from 'lucide-react';

export default function AgentTracePage() {
  const [steps, setSteps] = useState<ReActStep[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [goal, setGoal] = useState('');
  const [error, setError] = useState('');
  const connected = useWebSocketStore((s) => s.connected);

  const handleStart = async () => {
    if (!goal.trim()) return;
    setIsRunning(true);
    setError('');
    setSteps([]);

    try {
      const result = await chatApi.react({ goal: goal.trim(), maxIterations: 10 });
      setSteps(result.steps || []);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Agent execution failed');
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Agent Trace</h2>
          {connected && (
            <span className="flex items-center gap-1 text-xs text-emerald-500">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
              Live via WebSocket
            </span>
          )}
        </div>

        {/* Goal input */}
        <div className="flex flex-col sm:flex-row gap-2">
          <input
            type="text"
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
            placeholder="Enter agent goal..."
            disabled={isRunning}
            className="flex-1 px-4 py-2.5 sm:py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-base sm:text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 disabled:opacity-50"
          />
          <div className="flex gap-2">
            <button
              onClick={handleStart}
              disabled={isRunning || !goal.trim()}
              className="flex-1 sm:flex-none items-center justify-center gap-2 px-4 py-2.5 sm:py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors min-h-[44px] sm:min-h-0"
            >
            {isRunning ? (
              <>
                <div className="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin" />
                Running
              </>
            ) : (
              <>
                <Play className="w-3 h-3" />
                Start Agent
              </>
            )}
          </button>
          {steps.length > 0 && !isRunning && (
            <button
              onClick={() => { setSteps([]); setGoal(''); }}
              className="px-3 py-2.5 sm:py-2 rounded-lg text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-card)] transition-colors min-w-[44px] min-h-[44px] sm:min-w-0 sm:min-h-0 flex items-center justify-center"
            >
              <RotateCcw className="w-4 h-4" />
            </button>
          )}
          </div>
        </div>

        {error && (
          <div className="text-sm text-red-500 bg-red-500/10 rounded-lg px-3 py-2">{error}</div>
        )}

        {/* Trace display */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
          <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-3">ReAct Trace</h3>
          <AgentTrace steps={steps} isRunning={isRunning} />
        </div>
      </div>
    </AppShell>
  );
}
