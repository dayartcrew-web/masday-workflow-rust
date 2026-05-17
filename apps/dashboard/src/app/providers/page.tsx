'use client';

import { useEffect, useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { useSettingsStore } from '@/stores/settings-store';
import { useAuthStore } from '@/stores/auth-store';
import { RefreshCw, Zap, CheckCircle, XCircle, AlertTriangle } from 'lucide-react';

export default function ProvidersPage() {
  const providers = useSettingsStore((s) => s.providers) ?? [];
  const fetchProviders = useSettingsStore((s) => s.fetchProviders);
  const testProvider = useSettingsStore((s) => s.testProvider);
  const testResult = useSettingsStore((s) => s.testResult);
  const isTesting = useSettingsStore((s) => s.isTesting);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const [testingName, setTestingName] = useState<string | null>(null);
  const [testPrompt, setTestPrompt] = useState('Hello, respond with OK');

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchProviders();
  }, [isAuthenticated, fetchProviders]);

  const handleTest = async (name: string) => {
    setTestingName(name);
    await testProvider(name, { prompt: testPrompt });
    setTestingName(null);
  };

  const statusIcon = (status: string) => {
    if (status === 'available') return <CheckCircle className="w-4 h-4 text-emerald-500" />;
    if (status === 'unavailable') return <XCircle className="w-4 h-4 text-red-500" />;
    return <AlertTriangle className="w-4 h-4 text-yellow-500" />;
  };

  const circuitColor = (state: string) => {
    if (state === 'closed') return 'bg-emerald-500/10 text-emerald-400';
    if (state === 'open') return 'bg-red-500/10 text-red-400';
    return 'bg-yellow-500/10 text-yellow-400';
  };

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">LLM Providers</h2>
          <button
            onClick={() => fetchProviders()}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-card)] transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {/* Test prompt input */}
        <div className="flex gap-2">
          <input
            type="text"
            value={testPrompt}
            onChange={(e) => setTestPrompt(e.target.value)}
            placeholder="Test prompt..."
            className="flex-1 px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
          />
        </div>

        {/* Providers list */}
        {providers.length === 0 ? (
          <div className="text-center py-8 text-[var(--text-secondary)]">
            No providers configured. Configure providers via environment variables.
          </div>
        ) : (
          <div className="grid gap-3">
            {providers.map((provider) => (
              <div
                key={provider.name}
                className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4"
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    {statusIcon(provider.status)}
                    <div>
                      <h3 className="font-medium text-[var(--text-primary)]">{provider.name}</h3>
                      <p className="text-xs text-[var(--text-secondary)]">{provider.type}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`text-xs px-2 py-0.5 rounded ${circuitColor(provider.circuitState)}`}>
                      Circuit: {provider.circuitState}
                    </span>
                    <button
                      onClick={() => handleTest(provider.name)}
                      disabled={isTesting && testingName === provider.name}
                      className="flex items-center gap-1 px-3 py-1 rounded-lg bg-brand-600 text-white text-xs hover:bg-brand-700 disabled:opacity-50 transition-colors"
                    >
                      <Zap className="w-3 h-3" />
                      {isTesting && testingName === provider.name ? 'Testing...' : 'Test'}
                    </button>
                  </div>
                </div>

                {/* Models */}
                {(provider.models?.length ?? 0) > 0 && (
                  <div className="flex flex-wrap gap-1 mt-2">
                    {provider.models.map((model, idx) => (
                      <span key={`${provider.name}-${model}-${idx}`} className="text-xs px-2 py-0.5 rounded bg-[var(--bg-secondary)] text-[var(--text-secondary)]">
                        {model}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Test result */}
        {testResult && (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4">
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-2">Test Result</h3>
            <pre className="text-xs bg-[var(--bg-secondary)] rounded-lg p-3 overflow-x-auto text-[var(--text-primary)]">
              {JSON.stringify(testResult, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </AppShell>
  );
}
