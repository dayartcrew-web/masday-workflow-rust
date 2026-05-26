'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { AppShell } from '@/components/app-shell';
import { DataTable } from '@/components/ui/data-table';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useAuthStore } from '@/stores/auth-store';
import { Plus } from 'lucide-react';
import type { Workflow } from '@/lib/types';

export default function WorkflowsPage() {
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const isLoading = useWorkflowStore((s) => s.isLoading);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const router = useRouter();

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchWorkflows();
  }, [isAuthenticated, fetchWorkflows]);

  const columns = [
    { key: 'name', label: 'Name', sortable: true, render: (w: Workflow) => (
      <span className="font-medium">{w.name}</span>
    )},
    { key: 'state', label: 'State', sortable: true, render: (w: Workflow) => (
      <span className={`text-xs px-2 py-0.5 rounded ${
        w.state === 'DONE' ? 'bg-emerald-500/10 text-emerald-400' :
        w.state === 'FAILED' ? 'bg-red-500/10 text-red-400' :
        w.state === 'running' ? 'bg-blue-500/10 text-blue-400' :
        'bg-gray-500/10 text-gray-400'
      }`}>{w.state}</span>
    )},
    { key: 'tasks', label: 'Tasks', render: (w: Workflow) => (
      <span className="text-[var(--text-secondary)]">{w.tasks?.length || 0}</span>
    )},
    { key: 'createdAt', label: 'Created', sortable: true, hideOnMobile: true, render: (w: Workflow) => (
      <span className="text-xs text-[var(--text-secondary)]">
        {w.createdAt ? new Date(w.createdAt).toLocaleDateString() : '-'}
      </span>
    )},
  ];

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Workflows</h2>
          <Link
            href="/workflows/new"
            className="flex items-center gap-2 px-3 sm:px-4 py-2.5 sm:py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 transition-colors min-h-[44px] sm:min-h-0"
          >
            <Plus className="w-4 h-4" />
            New Workflow
          </Link>
        </div>

        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)]">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="w-6 h-6 border-2 border-brand-600 border-t-transparent rounded-full animate-spin" />
            </div>
          ) : (
            <DataTable
              columns={columns}
              data={workflows}
              keyField="id"
              emptyMessage="No workflows found. Create one to get started."
              onRowClick={(item) => router.push(`/workflows/${item.id}`)}
            />
          )}
        </div>
      </div>
    </AppShell>
  );
}
