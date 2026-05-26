'use client';

import { useState } from 'react';
import * as Tabs from '@radix-ui/react-tabs';
import { AppShell } from '@/components/app-shell';
import { policyApi } from '@/lib/api-client';
import { Shield, AlertTriangle, CheckCircle } from 'lucide-react';
import type { DriftResult } from '@/lib/types';

export default function PolicyPage() {
  const [activeTab, setActiveTab] = useState<'readiness' | 'drift' | 'validation'>('readiness');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  // Readiness state
  const [sessionKey, setSessionKey] = useState('');
  const [readinessResult, setReadinessResult] = useState<Record<string, unknown> | null>(null);

  // Drift state
  const [driftWorkflowId, setDriftWorkflowId] = useState('');
  const [driftOriginal, setDriftOriginal] = useState('');
  const [driftCurrent, setDriftCurrent] = useState('');
  const [driftResult, setDriftResult] = useState<DriftResult | null>(null);

  // Validation state
  const [validationWorkflowId, setValidationWorkflowId] = useState('');
  const [validationTaskId, setValidationTaskId] = useState('');
  const [validationCriteria, setValidationCriteria] = useState('');
  const [validationResult, setValidationResult] = useState<Record<string, unknown> | null>(null);

  const checkReadiness = async () => {
    if (!sessionKey.trim()) return;
    setIsLoading(true);
    setError('');
    try {
      const result = await policyApi.checkReadiness(sessionKey);
      setReadinessResult(result as Record<string, unknown>);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to check readiness');
    } finally {
      setIsLoading(false);
    }
  };

  const detectDrift = async () => {
    if (!driftWorkflowId || !driftOriginal || !driftCurrent) return;
    setIsLoading(true);
    setError('');
    try {
      const result = await policyApi.detectDrift({
        workflowId: driftWorkflowId,
        originalScope: driftOriginal,
        currentInput: driftCurrent,
      });
      setDriftResult(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Drift detection failed');
    } finally {
      setIsLoading(false);
    }
  };

  const validateCompletion = async () => {
    if (!validationWorkflowId || !validationTaskId) return;
    setIsLoading(true);
    setError('');
    try {
      const result = await policyApi.validateCompletion({
        workflowId: validationWorkflowId,
        taskId: validationTaskId,
        acceptanceCriteria: validationCriteria.split('\n').filter(Boolean),
        evidence: [],
      });
      setValidationResult(result as unknown as Record<string, unknown>);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Validation failed');
    } finally {
      setIsLoading(false);
    }
  };

  const tabs = [
    { key: 'readiness' as const, label: 'Session Readiness' },
    { key: 'drift' as const, label: 'Drift Detection' },
    { key: 'validation' as const, label: 'Completion Validation' },
  ];

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-brand-400" />
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Policy Settings</h2>
        </div>

        {/* Tabs (Radix UI) */}
        <Tabs.Root value={activeTab} onValueChange={(v) => setActiveTab(v as typeof activeTab)}>
          <Tabs.List className="flex gap-1 border-b border-[var(--border)]">
            {tabs.map((tab) => (
              <Tabs.Trigger
                key={tab.key}
                value={tab.key}
                className="px-4 py-2 text-sm font-medium border-b-2 transition-colors data-[state=active]:border-brand-500 data-[state=active]:text-brand-400 border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              >
                {tab.label}
              </Tabs.Trigger>
            ))}
          </Tabs.List>

        {error && (
          <div className="text-sm text-red-500 bg-red-500/10 rounded-lg px-3 py-2">{error}</div>
        )}

          <Tabs.Content value="readiness" className="mt-4 space-y-3">
            <div className="flex gap-2">
              <input
                type="text"
                value={sessionKey}
                onChange={(e) => setSessionKey(e.target.value)}
                placeholder="Session key..."
                className="flex-1 px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
              />
              <button onClick={checkReadiness} disabled={isLoading} className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors">
                Check
              </button>
            </div>
            {readinessResult && (
              <pre className="text-xs bg-[var(--bg-secondary)] rounded-lg p-3 overflow-x-auto text-[var(--text-primary)]">
                {JSON.stringify(readinessResult, null, 2)}
              </pre>
            )}
          </Tabs.Content>

          <Tabs.Content value="drift" className="mt-4 space-y-3">
            <input
              type="text"
              value={driftWorkflowId}
              onChange={(e) => setDriftWorkflowId(e.target.value)}
              placeholder="Workflow ID"
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
            <textarea
              value={driftOriginal}
              onChange={(e) => setDriftOriginal(e.target.value)}
              placeholder="Original scope..."
              rows={3}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none"
            />
            <textarea
              value={driftCurrent}
              onChange={(e) => setDriftCurrent(e.target.value)}
              placeholder="Current input..."
              rows={3}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none"
            />
            <button onClick={detectDrift} disabled={isLoading} className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors">
              Detect Drift
            </button>
            {driftResult && (
              <div className={`rounded-lg p-4 border ${
                driftResult.drifted ? 'border-red-500/30 bg-red-500/5' : 'border-emerald-500/30 bg-emerald-500/5'
              }`}>
                <div className="flex items-center gap-2 mb-2">
                  {driftResult.drifted ? <AlertTriangle className="w-4 h-4 text-red-400" /> : <CheckCircle className="w-4 h-4 text-emerald-400" />}
                  <span className="font-medium text-[var(--text-primary)]">
                    {driftResult.drifted ? 'Drift Detected' : 'No Drift'}
                  </span>
                </div>
                <div className="text-sm space-y-1">
                  <p>Score: {driftResult.score.toFixed(3)} (threshold: {driftResult.threshold})</p>
                  <p className="text-[var(--text-secondary)]">{driftResult.recommendation}</p>
                </div>
              </div>
            )}
          </Tabs.Content>

          <Tabs.Content value="validation" className="mt-4 space-y-3">
            <input
              type="text"
              value={validationWorkflowId}
              onChange={(e) => setValidationWorkflowId(e.target.value)}
              placeholder="Workflow ID"
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
            <input
              type="text"
              value={validationTaskId}
              onChange={(e) => setValidationTaskId(e.target.value)}
              placeholder="Task ID"
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
            <textarea
              value={validationCriteria}
              onChange={(e) => setValidationCriteria(e.target.value)}
              placeholder="Acceptance criteria (one per line)..."
              rows={4}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none"
            />
            <button onClick={validateCompletion} disabled={isLoading} className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors">
              Validate
            </button>
            {validationResult && (
              <pre className="text-xs bg-[var(--bg-secondary)] rounded-lg p-3 overflow-x-auto text-[var(--text-primary)]">
                {JSON.stringify(validationResult, null, 2)}
              </pre>
            )}
          </Tabs.Content>
        </Tabs.Root>
      </div>
    </AppShell>
  );
}
