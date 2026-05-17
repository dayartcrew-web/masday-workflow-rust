'use client';

import { useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { SelectRoot, SelectTrigger, SelectContent, SelectItem, SelectValue } from '@/components/ui/select';
import { policyApi } from '@/lib/api-client';
import { useWorkflowStore } from '@/stores/workflow-store';
import { ClipboardCheck, AlertTriangle, Search, Clock, FileText } from 'lucide-react';
import type { AuditResult } from '@/lib/types';

export default function AuditPage() {
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const [selectedWorkflow, setSelectedWorkflow] = useState('');
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const runAudit = async () => {
    if (!selectedWorkflow) return;
    setIsLoading(true);
    setError('');
    try {
      const result = await policyApi.auditWorkflow(selectedWorkflow);
      setAuditResult(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Audit failed');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <ClipboardCheck className="w-5 h-5 text-brand-400" />
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Workflow Audit</h2>
        </div>

        {/* Workflow selector */}
        <div className="flex gap-2">
          <SelectRoot value={selectedWorkflow} onValueChange={setSelectedWorkflow}>
            <SelectTrigger className="flex-1">
              <SelectValue placeholder="Select workflow to audit..." />
            </SelectTrigger>
            <SelectContent>
              {workflows.map((w) => (
                <SelectItem key={w.id} value={w.id}>{w.name}</SelectItem>
              ))}
            </SelectContent>
          </SelectRoot>
          <button
            onClick={runAudit}
            disabled={isLoading || !selectedWorkflow}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors"
          >
            <Search className="w-4 h-4" />
            {isLoading ? 'Auditing...' : 'Run Audit'}
          </button>
        </div>

        {error && (
          <div className="text-sm text-red-500 bg-red-500/10 rounded-lg px-3 py-2">{error}</div>
        )}

        {/* Audit Results */}
        {auditResult && (
          <div className="space-y-4">
            {/* Summary */}
            <div className={`rounded-xl border p-4 ${
              auditResult.totalIssues > 0
                ? 'border-yellow-500/30 bg-yellow-500/5'
                : 'border-emerald-500/30 bg-emerald-500/5'
            }`}>
              <div className="flex items-center gap-2 mb-2">
                {auditResult.totalIssues > 0 ? (
                  <AlertTriangle className="w-4 h-4 text-yellow-500" />
                ) : (
                  <ClipboardCheck className="w-4 h-4 text-emerald-500" />
                )}
                <span className="font-medium text-[var(--text-primary)]">
                  {auditResult.totalIssues > 0
                    ? `${auditResult.totalIssues} issues found`
                    : 'No issues found'
                  }
                </span>
              </div>
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div>
                  <span className="text-[var(--text-secondary)]">Stuck Tasks</span>
                  <p className="font-medium text-[var(--text-primary)]">{auditResult.stuckTasks.length}</p>
                </div>
                <div>
                  <span className="text-[var(--text-secondary)]">Missing Reviews</span>
                  <p className="font-medium text-[var(--text-primary)]">{auditResult.missingReviews.length}</p>
                </div>
                <div>
                  <span className="text-[var(--text-secondary)]">Incomplete Progress</span>
                  <p className="font-medium text-[var(--text-primary)]">{auditResult.incompleteProgress.length}</p>
                </div>
              </div>
            </div>

            {/* Stuck Tasks */}
            {auditResult.stuckTasks.length > 0 && (
              <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
                <div className="flex items-center gap-2 mb-3">
                  <Clock className="w-4 h-4 text-yellow-500" />
                  <h3 className="text-sm font-medium text-[var(--text-primary)]">Stuck Tasks</h3>
                </div>
                <div className="space-y-2">
                  {auditResult.stuckTasks.map((task) => (
                    <div key={task.id} className="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-secondary)]">
                      <div>
                        <span className="text-sm font-medium text-[var(--text-primary)]">{task.name}</span>
                        <span className="ml-2 text-xs text-[var(--text-secondary)]">{task.agent}</span>
                      </div>
                      <span className="text-xs px-2 py-0.5 rounded bg-yellow-500/10 text-yellow-400">{task.state}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Missing Reviews */}
            {auditResult.missingReviews.length > 0 && (
              <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
                <div className="flex items-center gap-2 mb-3">
                  <FileText className="w-4 h-4 text-red-500" />
                  <h3 className="text-sm font-medium text-[var(--text-primary)]">Missing Reviews</h3>
                </div>
                <div className="space-y-1">
                  {auditResult.missingReviews.map((review, idx) => (
                    <div key={idx} className="p-2 text-sm text-[var(--text-secondary)] bg-[var(--bg-secondary)] rounded-lg">
                      {review}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Incomplete Progress */}
            {auditResult.incompleteProgress.length > 0 && (
              <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
                <div className="flex items-center gap-2 mb-3">
                  <AlertTriangle className="w-4 h-4 text-orange-500" />
                  <h3 className="text-sm font-medium text-[var(--text-primary)]">Incomplete Progress</h3>
                </div>
                <div className="space-y-1">
                  {auditResult.incompleteProgress.map((item, idx) => (
                    <div key={idx} className="p-2 text-sm text-[var(--text-secondary)] bg-[var(--bg-secondary)] rounded-lg">
                      {item}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </AppShell>
  );
}
