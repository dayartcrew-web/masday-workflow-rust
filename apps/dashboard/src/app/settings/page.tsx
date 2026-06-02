'use client';

import { AppShell } from '@/components/app-shell';
import { useSettingsStore } from '@/stores/settings-store';
import { useAuthStore } from '@/stores/auth-store';
import { Moon, Sun } from 'lucide-react';
import { useState } from 'react';

export default function SettingsPage() {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const user = useAuthStore((s) => s.user);
  const [apiUrl, setApiUrl] = useState(process.env.NEXT_PUBLIC_API_URL || 'http://localhost:30101');
  const [wsUrl, setWsUrl] = useState(process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:30101');

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-6">
        <h2 className="text-lg font-semibold text-[var(--text-primary)]">Settings</h2>

        {/* User Info */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4 space-y-3">
          <h3 className="text-sm font-medium text-[var(--text-secondary)]">User Profile</h3>
          {user && (
            <div className="grid grid-cols-2 gap-3 text-sm">
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
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4 space-y-3">
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
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4 space-y-3">
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

        {/* About */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4 space-y-2">
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
