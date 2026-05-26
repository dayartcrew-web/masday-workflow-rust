'use client';

import { AppShell } from '@/components/app-shell';
import { useSettingsStore } from '@/stores/settings-store';
import { useAuthStore } from '@/stores/auth-store';
import { Moon, Sun, CheckCircle, XCircle, Zap, RefreshCw, Plus, Trash2, Star } from 'lucide-react';
import { useEffect, useState } from 'react';

export default function SettingsPage() {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const providers = useSettingsStore((s) => s.providers);
  const fetchProviders = useSettingsStore((s) => s.fetchProviders);
  const testProvider = useSettingsStore((s) => s.testProvider);
  const testResult = useSettingsStore((s) => s.testResult);
  const isTesting = useSettingsStore((s) => s.isTesting);
  const isSaving = useSettingsStore((s) => s.isSaving);
  const saveProvider = useSettingsStore((s) => s.saveProvider);
  const deleteProviderAction = useSettingsStore((s) => s.deleteProvider);
  const setDefaultProvider = useSettingsStore((s) => s.setDefaultProvider);
  const user = useAuthStore((s) => s.user);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const [apiUrl, setApiUrl] = useState(process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000');
  const [wsUrl, setWsUrl] = useState(process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3001');
  const [testPrompt, setTestPrompt] = useState('Hello, respond with OK');
  const [testingName, setTestingName] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [formType, setFormType] = useState('openai');
  const [formName, setFormName] = useState('');
  const [formBaseUrl, setFormBaseUrl] = useState('https://api.openai.com/v1');
  const [formApiKey, setFormApiKey] = useState('');
  const [formModel, setFormModel] = useState('');

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchProviders();
  }, [isAuthenticated, fetchProviders]);

  const handleTest = async (name: string) => {
    setTestingName(name);
    await testProvider(name, { prompt: testPrompt });
    setTestingName(null);
  };

  const handleSave = async () => {
    const name = formName.trim() || formType;
    const models = formModel.split(',').map((m) => m.trim()).filter(Boolean);
    const ok = await saveProvider({
      providerName: name,
      providerType: formType,
      baseUrl: formBaseUrl,
      apiKey: formApiKey,
      models,
    });
    if (ok) {
      setShowForm(false);
      setFormName('');
      setFormApiKey('');
      setFormModel('');
    }
  };

  const handleDelete = async (name: string) => {
    await deleteProviderAction(name);
  };

  const baseUrlDefaults: Record<string, string> = {
    openai: 'https://api.openai.com/v1',
    anthropic: 'https://api.anthropic.com/v1',
    custom: '',
  };

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-4 md:space-y-6">
        <h2 className="text-lg font-semibold text-[var(--text-primary)]">Settings</h2>

        {/* User Info */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4 space-y-3">
          <h3 className="text-sm font-medium text-[var(--text-secondary)]">User Profile</h3>
          {user && (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-[var(--text-secondary)]">Name</span>
                <p className="text-[var(--text-primary)]">{user.name}</p>
              </div>
              <div>
                <span className="text-[var(--text-secondary)]">Email</span>
                <p className="text-[var(--text-primary)]">{user.email}</p>
              </div>
              <div>
                <span className="text-[var(--text-secondary)]">Role</span>
                <p className="text-[var(--text-primary)]">
                  <span className="text-xs px-1.5 py-0.5 rounded bg-brand-600/10 text-brand-400">{user.role}</span>
                </p>
              </div>
              <div>
                <span className="text-[var(--text-secondary)]">ID</span>
                <p className="text-xs font-mono text-[var(--text-primary)]">{user.id}</p>
              </div>
            </div>
          )}
        </div>

        {/* Theme */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4 space-y-3">
          <h3 className="text-sm font-medium text-[var(--text-secondary)]">Theme</h3>
          <div className="flex gap-2">
            <button
              onClick={() => setTheme('light')}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-colors ${
                theme === 'light' ? 'bg-brand-600 text-white' : 'bg-[var(--bg-card)] text-[var(--text-secondary)]'
              }`}
            >
              <Sun className="w-4 h-4" />
              Light
            </button>
            <button
              onClick={() => setTheme('dark')}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-colors ${
                theme === 'dark' ? 'bg-brand-600 text-white' : 'bg-[var(--bg-card)] text-[var(--text-secondary)]'
              }`}
            >
              <Moon className="w-4 h-4" />
              Dark
            </button>
          </div>
        </div>

        {/* Connection Settings */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4 space-y-3">
          <h3 className="text-sm font-medium text-[var(--text-secondary)]">Connection</h3>
          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1">API URL</label>
            <input
              type="text"
              value={apiUrl}
              onChange={(e) => setApiUrl(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1">WebSocket URL</label>
            <input
              type="text"
              value={wsUrl}
              onChange={(e) => setWsUrl(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
          </div>
          <p className="text-xs text-[var(--text-secondary)]">
            Note: These settings are read-only for this session. Configure via environment variables.
          </p>
        </div>

        {/* Providers */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium text-[var(--text-secondary)]">LLM Providers</h3>
            <div className="flex items-center gap-2">
              <button
                onClick={() => fetchProviders()}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)] transition-colors"
              >
                <RefreshCw className="w-3.5 h-3.5" />
              </button>
              <button
                onClick={() => { setShowForm(!showForm); setFormBaseUrl(baseUrlDefaults[formType]); }}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs bg-brand-600 text-white hover:bg-brand-700 transition-colors"
              >
                <Plus className="w-3.5 h-3.5" />
                Add
              </button>
            </div>
          </div>

          {/* Add/Edit Form */}
          {showForm && (
            <div className="space-y-2.5 p-3 rounded-lg bg-[var(--bg-secondary)]">
              <div>
                <label className="block text-xs text-[var(--text-secondary)] mb-1">Provider Type</label>
                <select
                  value={formType}
                  onChange={(e) => { setFormType(e.target.value); setFormBaseUrl(baseUrlDefaults[e.target.value] ?? ''); }}
                  className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                >
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="custom">Custom</option>
                </select>
              </div>
              <div>
                <label className="block text-xs text-[var(--text-secondary)] mb-1">Name</label>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  placeholder={formType}
                  className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs text-[var(--text-secondary)] mb-1">Base URL</label>
                <input
                  type="text"
                  value={formBaseUrl}
                  onChange={(e) => setFormBaseUrl(e.target.value)}
                  placeholder="https://api.openai.com/v1"
                  className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs text-[var(--text-secondary)] mb-1">API Key <span className="opacity-60">(optional)</span></label>
                <input
                  type="password"
                  value={formApiKey}
                  onChange={(e) => setFormApiKey(e.target.value)}
                  placeholder="Paste key or leave empty to use env default"
                  className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                />
              </div>
              <div>
                <label className="block text-xs text-[var(--text-secondary)] mb-1">Models (comma-separated)</label>
                <input
                  type="text"
                  value={formModel}
                  onChange={(e) => setFormModel(e.target.value)}
                  placeholder="gpt-4, gpt-3.5-turbo"
                  className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                />
              </div>
              <div className="flex gap-2 pt-1">
                <button
                  onClick={handleSave}
                  disabled={isSaving}
                  className="flex-1 px-3 py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors"
                >
                  {isSaving ? 'Saving...' : 'Save Provider'}
                </button>
                <button
                  onClick={() => setShowForm(false)}
                  className="px-3 py-2 rounded-lg border border-[var(--border)] text-[var(--text-secondary)] text-sm hover:bg-[var(--bg-card)] transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}

          {/* Provider List */}
          {providers.length === 0 && !showForm ? (
            <p className="text-sm text-[var(--text-secondary)]">No providers configured.</p>
          ) : (
            <div className="space-y-2">
              {providers.map((p) => (
                <div key={p.name} className="flex items-center justify-between gap-3 p-2.5 rounded-lg bg-[var(--bg-secondary)]">
                  <div className="flex items-center gap-2.5 min-w-0">
                    {p.status === 'available' ? <CheckCircle className="w-4 h-4 text-emerald-500 flex-shrink-0" /> : <XCircle className="w-4 h-4 text-red-500 flex-shrink-0" />}
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-[var(--text-primary)] truncate">
                        {p.name}
                        {p.source === 'env' && <span className="text-[10px] px-1 py-0.5 rounded bg-amber-500/10 text-amber-500 ml-1">ENV</span>}
                        {p.source === 'db' && !p.isDefault && <span className="text-[10px] px-1 py-0.5 rounded bg-slate-500/10 text-slate-400 ml-1">idle</span>}
                        {p.isDefault && <span className="text-[10px] px-1 py-0.5 rounded bg-brand-500/10 text-brand-400 ml-1">active</span>}
                      </p>
                      {(p.models?.length ?? 0) > 0 && (
                        <div className="flex flex-wrap gap-1 mt-0.5">
                          {p.models.map((m) => (
                            <span key={m} className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--bg-card)] text-[var(--text-secondary)]">{m}</span>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5 flex-shrink-0">
                    <button
                      onClick={() => setDefaultProvider(p.name)}
                      className={`p-1.5 rounded-lg transition-colors ${p.isDefault ? 'text-amber-400 bg-amber-500/10' : 'text-[var(--text-secondary)] hover:bg-[var(--bg-card)]'}`}
                      title={p.isDefault ? 'Active (default)' : 'Set as default'}
                    >
                      <Star className={`w-3.5 h-3.5 ${p.isDefault ? 'fill-current' : ''}`} />
                    </button>
                    <button
                      onClick={() => handleTest(p.name)}
                      disabled={isTesting && testingName === p.name}
                      className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-brand-600 text-white text-xs hover:bg-brand-700 disabled:opacity-50 transition-colors"
                    >
                      <Zap className="w-3 h-3" />
                      {isTesting && testingName === p.name ? '...' : 'Test'}
                    </button>
                    {p.source !== 'env' && (
                    <button
                      onClick={() => handleDelete(p.name)}
                      disabled={isSaving}
                      className="p-1.5 rounded-lg text-red-400 hover:bg-red-500/10 transition-colors"
                      title="Delete provider"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Test result */}
          {testResult && (
            <pre className="text-xs bg-[var(--bg-secondary)] rounded-lg p-3 overflow-x-auto text-[var(--text-primary)] max-h-48 overflow-y-auto">
              {JSON.stringify(testResult, null, 2)}
            </pre>
          )}
        </div>

        {/* About */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-3 md:p-4 space-y-2">
          <h3 className="text-sm font-medium text-[var(--text-secondary)]">About</h3>
          <div className="text-sm text-[var(--text-secondary)] space-y-1">
            <p>Masday Workflow Dashboard v1.0.0</p>
            <p>Autonomous AI agent platform built on MCP protocol</p>
            <p>Next.js 16 + Tailwind CSS + Zustand + Recharts + D3.js</p>
          </div>
        </div>
      </div>
    </AppShell>
  );
}
