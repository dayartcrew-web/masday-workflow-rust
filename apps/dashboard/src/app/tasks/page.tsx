'use client';

import { useEffect, useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { DataTable } from '@/components/ui/data-table';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useAuthStore } from '@/stores/auth-store';
import type { Task } from '@/lib/types';
import { Filter } from 'lucide-react';

export default function TasksPage() {
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const [filter, setFilter] = useState<string>('all');

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchWorkflows();
  }, [isAuthenticated, fetchWorkflows]);

  const allTasks = workflows.flatMap((w) =>
    (w.tasks || []).map((t) => ({ ...t, workflowId: w.id, workflowName: w.name })),
  );

  const filtered = filter === 'all' ? allTasks : allTasks.filter((t) => t.state === filter);
  const states = ['all', 'pending', 'running', 'done', 'failed'];

  const columns = [
    { key: 'name', label: 'Task', sortable: true, render: (t: Task & { workflowName: string }) => (
      <div>
        <span className="font-medium">{t.name}</span>
        <span className="block text-xs text-[var(--text-secondary)]">{t.workflowName}</span>
      </div>
    )},
    { key: 'state', label: 'State', sortable: true, render: (t: Task) => (
      <span className={`text-xs px-2 py-0.5 rounded ${
        t.state === 'done' ? 'bg-emerald-500/10 text-emerald-400' :
        t.state === 'failed' ? 'bg-red-500/10 text-red-400' :
        t.state === 'running' ? 'bg-blue-500/10 text-blue-400' :
        'bg-gray-500/10 text-gray-400'
      }`}>{t.state}</span>
    )},
    { key: 'agent', label: 'Agent', render: (t: Task) => (
      <span className="text-sm text-[var(--text-secondary)]">{t.agent}</span>
    )},
    { key: 'skill', label: 'Skill', render: (t: Task) => (
      <span className="text-sm text-[var(--text-secondary)]">{t.skill}</span>
    )},
  ];

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Tasks</h2>
          <div className="flex items-center gap-2">
            <Filter className="w-4 h-4 text-[var(--text-secondary)]" />
            <div className="flex gap-1">
              {states.map((s) => (
                <button
                  key={s}
                  onClick={() => setFilter(s)}
                  className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                    filter === s
                      ? 'bg-brand-600 text-white'
                      : 'bg-[var(--bg-card)] text-[var(--text-secondary)] hover:bg-[var(--border)]'
                  }`}
                >
                  {s === 'all' ? 'All' : s}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)]">
          <DataTable
            columns={columns}
            data={filtered}
            keyField="id"
            emptyMessage="No tasks found"
          />
        </div>
      </div>
    </AppShell>
  );
}
