'use client';

import React, { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { AppShell } from '@/components/app-shell';
import { WorkflowDag } from '@/components/workflow-dag';
import { DataTable } from '@/components/ui/data-table';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useWebSocketStore } from '@/stores/websocket-store';
import { ArrowLeft, Play, CheckCircle } from 'lucide-react';
import type { Task } from '@/lib/types';

export default function WorkflowDetailPage() {
  const params = useParams();
  const router = useRouter();
  const workflowId = params.id as string;
  const selectedWorkflow = useWorkflowStore((s) => s.selectedWorkflow);
  const fetchWorkflow = useWorkflowStore((s) => s.fetchWorkflow);
  const executeWorkflow = useWorkflowStore((s) => s.executeWorkflow);
  const updateTaskState = useWorkflowStore((s) => s.updateTaskState);
  const isLoading = useWorkflowStore((s) => s.isLoading);
  const latestEvent = useWebSocketStore((s) => s.latestEvent);

  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);

  useEffect(() => {
    if (workflowId) fetchWorkflow(workflowId);
  }, [workflowId, fetchWorkflow]);

  // Update on WebSocket events
  useEffect(() => {
    if (!latestEvent || !latestEvent.type.startsWith('task.') || !selectedWorkflow) return;

    const data = latestEvent.data as {
      workflowId?: string;
      taskId?: string;
      state?: string;
    };

    if (!data.taskId || !data.state) return;

    // Prefer workflow-scoped filtering when the payload provides workflowId.
    if (typeof data.workflowId === 'string' && data.workflowId.length > 0) {
      if (data.workflowId !== selectedWorkflow.id) return;
      updateTaskState(selectedWorkflow.id, data.taskId, data.state);
      return;
    }

    // Some legacy task.* events may omit workflowId. In that case, only apply the
    // update if the task belongs to the currently selected workflow.
    const belongsToSelectedWorkflow = selectedWorkflow.tasks.some((t) => t.id === data.taskId);
    if (!belongsToSelectedWorkflow) return;

    updateTaskState(selectedWorkflow.id, data.taskId, data.state);
  }, [latestEvent, selectedWorkflow, updateTaskState]);

  const handleExecute = async () => {
    if (workflowId) {
      await executeWorkflow(workflowId);
      fetchWorkflow(workflowId);
    }
  };

  const taskColumns = [
    { key: 'name', label: 'Task', sortable: true, render: (t: Task) => (
      <span className="font-medium">{t.name}</span>
    )},
    { key: 'state', label: 'State', sortable: true, render: (t: Task) => (
      <span className={`text-xs px-2 py-0.5 rounded ${
        t.state === 'done' ? 'bg-emerald-500/10 text-emerald-400' :
        t.state === 'failed' ? 'bg-red-500/10 text-red-400' :
        t.state === 'running' ? 'bg-blue-500/10 text-blue-400' :
        'bg-gray-500/10 text-gray-400'
      }`}>{t.state}</span>
    )},
    { key: 'agent', label: 'Agent', hideOnMobile: true, render: (t: Task) => (
      <span className="text-xs text-[var(--text-secondary)]">{t.agent}</span>
    )},
    { key: 'skill', label: 'Skill', hideOnMobile: true, render: (t: Task) => (
      <span className="text-xs text-[var(--text-secondary)]">{t.skill}</span>
    )},
  ];

  if (isLoading && !selectedWorkflow) {
    return (
      <AppShell>
        <div className="flex items-center justify-center py-16">
          <div className="w-8 h-8 border-2 border-brand-600 border-t-transparent rounded-full animate-spin" />
        </div>
      </AppShell>
    );
  }

  if (!selectedWorkflow) {
    return (
      <AppShell>
        <div className="text-center py-16">
          <p className="text-[var(--text-secondary)]">Workflow not found</p>
          <button onClick={() => router.push('/workflows')} className="mt-4 text-brand-400 text-sm">
            Back to workflows
          </button>
        </div>
      </AppShell>
    );
  }

  const selectedTask = selectedWorkflow.tasks.find((task) => task.id === selectedTaskId) ?? null;
  const completedTasks = selectedWorkflow.tasks.filter((t) => t.state === 'done').length;
  const totalTasks = selectedWorkflow.tasks.length;
  const progressPct = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  return (
    <AppShell>
      <div className="space-y-4">
        {/* Header */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <button onClick={() => router.push('/workflows')} className="p-2 rounded-lg hover:bg-[var(--bg-card)] flex-shrink-0">
              <ArrowLeft className="w-4 h-4 text-[var(--text-secondary)]" />
            </button>
            <div className="min-w-0">
              <h2 className="text-lg font-semibold text-[var(--text-primary)] truncate">{selectedWorkflow.name}</h2>
              <p className="text-sm text-[var(--text-secondary)] truncate">{selectedWorkflow.description}</p>
            </div>
          </div>
          <div className="flex items-center gap-2 flex-shrink-0">
            <span className={`text-xs px-2 py-1 rounded ${
              selectedWorkflow.state === 'DONE' ? 'bg-emerald-500/10 text-emerald-400' :
              selectedWorkflow.state === 'FAILED' ? 'bg-red-500/10 text-red-400' :
              'bg-blue-500/10 text-blue-400'
            }`}>{selectedWorkflow.state}</span>
            <button
              onClick={handleExecute}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 transition-colors"
            >
              <Play className="w-3 h-3" />
              Execute
            </button>
          </div>
        </div>

        {/* Progress */}
        <div className="flex items-center gap-4 text-sm">
          <div className="flex items-center gap-2">
            <CheckCircle className="w-4 h-4 text-emerald-500" />
            <span>{completedTasks}/{totalTasks} tasks</span>
          </div>
          <div className="flex-1 h-2 bg-[var(--border)] rounded-full overflow-hidden">
            <div className="h-full bg-brand-500 rounded-full transition-all" style={{ width: `${progressPct}%` }} />
          </div>
          <span className="text-xs text-[var(--text-secondary)]">{progressPct}%</span>
        </div>

        {/* DAG Visualization */}
        <div>
          <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-2">Workflow DAG</h3>
          <WorkflowDag
            tasks={selectedWorkflow.tasks}
            onTaskClick={(task) => setSelectedTaskId(task.id)}
          />
        </div>

        {/* Selected Task Detail */}
        {selectedTask && (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-medium text-[var(--text-primary)]">{selectedTask.name}</h3>
              <button onClick={() => setSelectedTaskId(null)} className="text-xs text-[var(--text-secondary)]">
                Close
              </button>
            </div>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div><span className="text-[var(--text-secondary)]">State:</span> <span className="ml-1">{selectedTask.state}</span></div>
              <div><span className="text-[var(--text-secondary)]">Agent:</span> <span className="ml-1">{selectedTask.agent}</span></div>
              <div><span className="text-[var(--text-secondary)]">Skill:</span> <span className="ml-1">{selectedTask.skill}</span></div>
              <div><span className="text-[var(--text-secondary)]">Dependencies:</span> <span className="ml-1">{selectedTask.dependencies.length}</span></div>
            </div>
            {selectedTask.output != null ? (
              <div className="mt-3">
                <span className="text-xs text-[var(--text-secondary)]">Output:</span>
                <pre className="mt-1 text-xs bg-[var(--bg-secondary)] rounded p-2 overflow-x-auto">
                  {JSON.stringify(selectedTask.output, null, 2)}
                </pre>
              </div>
            ) : null}
          </div>
        )}

        {/* Tasks Table */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)]">
          <div className="px-4 py-3 border-b border-[var(--border)]">
            <h3 className="text-sm font-medium text-[var(--text-secondary)]">Tasks</h3>
          </div>
          <DataTable
            columns={taskColumns}
            data={selectedWorkflow.tasks}
            keyField="id"
            emptyMessage="No tasks in this workflow"
            onRowClick={(item) => setSelectedTaskId(item.id)}
          />
        </div>
      </div>
    </AppShell>
  );
}
