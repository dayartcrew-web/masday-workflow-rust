'use client';

import { useEffect } from 'react';
import { AppShell } from '@/components/app-shell';
import { MetricCard } from '@/components/ui/metric-card';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useAnalyticsStore } from '@/stores/analytics-store';
import { useAuthStore } from '@/stores/auth-store';
import { useWebSocketStore } from '@/stores/websocket-store';
import { GitBranch, CheckCircle, XCircle, Clock, Activity, Brain, ListTodo, CheckCheck, Zap } from 'lucide-react';
import Link from 'next/link';

export default function DashboardPage() {
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const fetchActive = useWorkflowStore((s) => s.fetchActive);
  const activeWorkflow = useWorkflowStore((s) => s.activeWorkflow);
  const metrics = useAnalyticsStore((s) => s.metrics);
  const refreshAll = useAnalyticsStore((s) => s.refreshAll);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const latestEvent = useWebSocketStore((s) => s.latestEvent);
  const eventCount = useWebSocketStore((s) => s.eventCount);

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchWorkflows();
    fetchActive();
    refreshAll();
  }, [isAuthenticated, fetchWorkflows, fetchActive, refreshAll]);

  const activeCount = workflows.filter((w) => w.state !== 'DONE' && w.state !== 'FAILED').length;
  const completedCount = workflows.filter((w) => w.state === 'DONE').length;
  const failedCount = workflows.filter((w) => w.state === 'FAILED').length;

  return (
    <AppShell>
      <div className="space-y-8">
        {/* KPI Stats Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
          <MetricCard
            title="Active Workflows"
            value={activeCount}
            icon={<Activity className="w-4 h-4 text-[var(--color-neon-blue)]" />}
          />
          <MetricCard
            title="Completed"
            value={completedCount}
            icon={<CheckCircle className="w-4 h-4 text-[var(--color-neon-green)]" />}
          />
          <MetricCard
            title="Failed"
            value={failedCount}
            icon={<XCircle className="w-4 h-4 text-[var(--color-error)]" />}
          />
          <MetricCard
            title="WebSocket Events"
            value={eventCount}
            icon={<Clock className="w-4 h-4 text-[var(--color-primary)]" />}
          />
        </div>

        {/* Analytics Metrics Row */}
        {metrics && (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
            <MetricCard title="Total Tasks" value={metrics.tasksTotal ?? 0} icon={<ListTodo className="w-4 h-4 text-[var(--color-neon-blue)]" />} />
            <MetricCard title="Tasks Completed" value={metrics.tasksCompleted ?? 0} icon={<CheckCheck className="w-4 h-4 text-[var(--color-neon-green)]" />} />
            <MetricCard title="Total Memories" value={metrics.memoriesTotal ?? 0} icon={<Brain className="w-4 h-4 text-[var(--color-secondary)]" />} />
            <MetricCard title="Tokens Used" value={(metrics.tokensUsed ?? 0).toLocaleString()} icon={<Zap className="w-4 h-4 text-[var(--color-warning)]" />} />
          </div>
        )}

        {/* Active Workflow + Live Events - Two-column glassmorphism grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
          {/* Active Workflow */}
          <div className="rounded-[var(--radius-lg,16px)] border border-[var(--color-border-subtle)] bg-[var(--color-surface)] p-6 shadow-[var(--shadow-card-depth,0_8px_40px_rgba(0,0,0,0.45))] backdrop-blur-[20px]">
            <h3 className="text-[14px] font-medium text-[var(--color-text-secondary)] mb-4 uppercase tracking-wider">Active Workflow</h3>
            {activeWorkflow ? (
              <div className="space-y-3">
                <Link
                  href={`/workflows/${activeWorkflow.id}`}
                  className="block p-4 rounded-[var(--radius-md,12px)] bg-[var(--color-surface-elevated)] border border-[var(--color-border-subtle)] transition-[background-color,box-shadow,border-color] duration-250 ease-in-out hover:bg-[rgba(99,102,241,0.06)] hover:border-[rgba(99,102,241,0.25)] hover:shadow-[0_0_20px_rgba(99,102,241,0.15)]"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-[var(--color-text)]">{activeWorkflow.name}</span>
                    <span className="text-xs px-3 py-1 rounded-full bg-[rgba(59,130,246,0.15)] text-[var(--color-neon-blue)] font-medium">
                      {activeWorkflow.state}
                    </span>
                  </div>
                  <p className="text-xs text-[var(--color-text-secondary)] mt-2">{activeWorkflow.description}</p>
                  <div className="flex items-center gap-2 mt-3 text-xs text-[var(--color-text-secondary)]">
                    <GitBranch className="w-3 h-3" />
                    <span>{activeWorkflow.tasks.length} tasks</span>
                  </div>
                </Link>
              </div>
            ) : (
              <p className="text-sm text-[var(--color-text-secondary)] text-center py-6">No active workflow</p>
            )}
          </div>

          {/* Live Events */}
          <div className="rounded-[var(--radius-lg,16px)] border border-[var(--color-border-subtle)] bg-[var(--color-surface)] p-6 shadow-[var(--shadow-card-depth,0_8px_40px_rgba(0,0,0,0.45))] backdrop-blur-[20px]">
            <h3 className="text-[14px] font-medium text-[var(--color-text-secondary)] mb-4 uppercase tracking-wider">Live Events</h3>
            <div className="space-y-2 max-h-52 overflow-y-auto scrollbar-thin">
              {latestEvent ? (
                <div className="text-xs p-3 rounded-lg bg-[var(--color-surface-elevated)] border-l-[3px] border-l-[var(--color-primary)]">
                  <span className="text-[var(--color-secondary)] font-medium">{latestEvent.type}</span>
                  <span className="text-[var(--color-text-secondary)] ml-3">
                    {new Date(latestEvent.timestamp).toLocaleTimeString()}
                  </span>
                </div>
              ) : (
                <p className="text-sm text-[var(--color-text-secondary)] text-center py-6">
                  Waiting for events...
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Recent Workflows */}
        <div className="rounded-[var(--radius-lg,16px)] border border-[var(--color-border-subtle)] bg-[var(--color-surface)] p-6 shadow-[var(--shadow-card-depth,0_8px_40px_rgba(0,0,0,0.45))] backdrop-blur-[20px]">
          <h3 className="text-[14px] font-medium text-[var(--color-text-secondary)] mb-4 uppercase tracking-wider">Recent Workflows</h3>
          {workflows.length === 0 ? (
            <p className="text-sm text-[var(--color-text-secondary)] text-center py-6">
              No workflows yet. Create one to get started.
            </p>
          ) : (
            <div className="space-y-2">
              {workflows.slice(0, 5).map((w) => (
                <Link
                  key={w.id}
                  href={`/workflows/${w.id}`}
                  className="flex items-center justify-between p-3 rounded-[var(--radius-md,12px)] transition-[background-color,box-shadow,border-color] duration-250 ease-in-out hover:bg-[var(--color-surface-elevated)] hover:border-[rgba(99,102,241,0.2)]"
                >
                  <div className="flex items-center gap-3">
                    <GitBranch className="w-4 h-4 text-[var(--color-text-secondary)]" />
                    <span className="text-sm text-[var(--color-text)]">{w.name}</span>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-[var(--color-text-secondary)]">{w.tasks.length} tasks</span>
                    <span className={`text-xs px-3 py-1 rounded-full font-medium ${
                      w.state === 'DONE' ? 'bg-[rgba(34,197,94,0.15)] text-[var(--color-neon-green)]' :
                      w.state === 'FAILED' ? 'bg-[rgba(239,68,68,0.15)] text-[var(--color-error)]' :
                      'bg-[rgba(59,130,246,0.15)] text-[var(--color-neon-blue)]'
                    }`}>
                      {w.state}
                    </span>
                  </div>
                </Link>
              ))}
            </div>
          )}
        </div>
      </div>
    </AppShell>
  );
}
