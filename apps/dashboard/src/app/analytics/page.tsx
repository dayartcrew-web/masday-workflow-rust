'use client';

import { useEffect } from 'react';
import { AppShell } from '@/components/app-shell';
import { MetricCard } from '@/components/ui/metric-card';
import { useAnalyticsStore } from '@/stores/analytics-store';
import { useAuthStore } from '@/stores/auth-store';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { Activity, Zap, Database, ListTodo, Brain } from 'lucide-react';

const COLORS = ['#3b82f6', '#22c55e', '#ef4444', '#f59e0b', '#8b5cf6', '#ec4899'];

const tooltipStyle = {
  backgroundColor: 'rgba(17, 24, 39, 0.85)',
  border: '1px solid rgba(99, 102, 241, 0.15)',
  borderRadius: '12px',
  backdropFilter: 'blur(12px)',
  color: '#f1f5f9',
  fontSize: '12px',
  boxShadow: '0 8px 40px rgba(0, 0, 0, 0.45)',
};

export default function AnalyticsPage() {
  const metrics = useAnalyticsStore((s) => s.metrics);
  const stats = useAnalyticsStore((s) => s.stats);
  const health = useAnalyticsStore((s) => s.health);
  const refreshAll = useAnalyticsStore((s) => s.refreshAll);
  const isLoading = useAnalyticsStore((s) => s.isLoading);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  useEffect(() => {
    if (!isAuthenticated) return;
    refreshAll();
  }, [isAuthenticated, refreshAll]);

  const workflowData = metrics ? [
    { name: 'Active', value: metrics.workflowsActive },
    { name: 'Completed', value: metrics.workflowsCompleted },
    { name: 'Failed', value: metrics.workflowsFailed },
  ] : [];

  const taskData = metrics ? [
    { name: 'Total', value: metrics.tasksTotal },
    { name: 'Completed', value: metrics.tasksCompleted },
    { name: 'Failed', value: metrics.tasksFailed },
  ] : [];

  return (
    <AppShell>
      <div className="space-y-4 md:space-y-6">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Analytics</h2>
          <button
            onClick={refreshAll}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm text-brand-400 hover:bg-brand-600/5 transition-colors"
          >
            <Activity className="w-4 h-4" />
            {isLoading ? 'Loading...' : 'Refresh'}
          </button>
        </div>

        {/* Top metrics */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 md:gap-4">
          <MetricCard
            title="Total Workflows"
            value={metrics?.workflowsTotal ?? 0}
            icon={<Database className="w-4 h-4" />}
          />
          <MetricCard
            title="Total Tasks"
            value={metrics?.tasksTotal ?? 0}
            icon={<ListTodo className="w-4 h-4 text-[var(--color-neon-blue)]" />}
          />
          <MetricCard
            title="Memories"
            value={metrics?.memoriesTotal ?? 0}
            icon={<Brain className="w-4 h-4 text-[var(--color-secondary)]" />}
          />
          <MetricCard
            title="Tokens Used"
            value={(metrics?.tokensUsed ?? 0).toLocaleString()}
            icon={<Zap className="w-4 h-4" />}
          />
        </div>

        {/* Charts */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 md:gap-4">
          {/* Workflow status chart */}
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4">
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-4">Workflow Status</h3>
            {workflowData.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <BarChart data={workflowData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                  <XAxis dataKey="name" tick={{ fill: 'var(--text-secondary)', fontSize: 12 }} />
                  <YAxis tick={{ fill: 'var(--text-secondary)', fontSize: 12 }} />
                  <Tooltip contentStyle={tooltipStyle} cursor={{ fill: 'rgba(99, 102, 241, 0.06)' }} />
                  <Bar dataKey="value" fill="#3b82f6" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-48 text-[var(--text-secondary)] text-sm">No data</div>
            )}
          </div>

          {/* Task performance chart */}
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4">
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-4">Task Performance</h3>
            {taskData.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <BarChart data={taskData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                  <XAxis dataKey="name" tick={{ fill: 'var(--text-secondary)', fontSize: 12 }} />
                  <YAxis tick={{ fill: 'var(--text-secondary)', fontSize: 12 }} />
                  <Tooltip contentStyle={tooltipStyle} cursor={{ fill: 'rgba(99, 102, 241, 0.06)' }} />
                  <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                    {taskData.map((_, idx) => (
                      <Cell key={idx} fill={COLORS[idx]} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-48 text-[var(--text-secondary)] text-sm">No data</div>
            )}
          </div>
        </div>

        {/* Server stats */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4">
          <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-3 md:mb-4">Server Performance</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4 text-sm">
            <div>
              <span className="text-[var(--text-secondary)]">Uptime</span>
              <p className="font-medium text-[var(--text-primary)]">
                {stats ? `${Math.round((stats.uptimeMs / 1000 / 60))}m` : '-'}
              </p>
            </div>
            <div>
              <span className="text-[var(--text-secondary)]">Total Requests</span>
              <p className="font-medium text-[var(--text-primary)]">{stats?.requestsTotal ?? 0}</p>
            </div>
            <div>
              <span className="text-[var(--text-secondary)]">Error Rate</span>
              <p className="font-medium text-[var(--text-primary)]">
                {stats && stats.requestsTotal > 0
                  ? `${((stats.errorsTotal / stats.requestsTotal) * 100).toFixed(1)}%`
                  : '0%'}
              </p>
            </div>
            <div>
              <span className="text-[var(--text-secondary)]">Avg Latency</span>
              <p className="font-medium text-[var(--text-primary)]">{stats?.avgLatencyMs ?? 0}ms</p>
            </div>
          </div>
        </div>

        {/* Health status */}
        {health && (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4">
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-3">Health Status</h3>
            <div className="flex items-center gap-3 mb-3">
              <span className={`w-3 h-3 rounded-full ${
                health.status === 'healthy' ? 'bg-emerald-500' :
                health.status === 'degraded' ? 'bg-yellow-500' : 'bg-red-500'
              }`} />
              <span className="text-sm font-medium text-[var(--text-primary)] capitalize">{health.status}</span>
            </div>
            {Object.entries(health.checks || {}).map(([name, check]) => (
              <div key={name} className="flex items-center justify-between text-sm py-1 gap-2">
                <span className="text-[var(--text-secondary)] truncate">{name}</span>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-[var(--text-secondary)]">{check.latencyMs}ms</span>
                  <span className={`text-xs px-2 py-0.5 rounded ${
                    check.status === 'healthy' ? 'bg-emerald-500/10 text-emerald-400' : 'bg-red-500/10 text-red-400'
                  }`}>{check.status}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </AppShell>
  );
}
