// ============================================================
// Settings Store — provider config, policy settings, preferences
// ============================================================

import { create } from 'zustand';
import { providerApi } from '@/lib/api-client';
import type { ProviderInfo } from '@/lib/types';

interface SettingsState {
  providers: ProviderInfo[];
  testResult: Record<string, unknown> | null;
  isTesting: boolean;
  isLoading: boolean;
  error: string | null;
  theme: 'light' | 'dark';
  fetchProviders: () => Promise<void>;
  testProvider: (name: string, input?: { model?: string; prompt?: string }) => Promise<void>;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleTheme: () => void;
  clearError: () => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  providers: [],
  testResult: null,
  isTesting: false,
  isLoading: false,
  error: null,
  theme: (typeof window !== 'undefined' && localStorage.getItem('theme') as 'light' | 'dark') || 'dark',

  fetchProviders: async () => {
    set({ isLoading: true });
    try {
      const result = await providerApi.list();
      const rawProviders = result.providers || [];
      // API may return string[] or ProviderInfo[] — normalize to ProviderInfo[]
      const providers: ProviderInfo[] = rawProviders.map((p) =>
        typeof p === 'string'
          ? { name: p, type: p, models: [], status: 'available' as const, circuitState: 'closed' as const }
          : p,
      );
      set({ providers, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch providers', isLoading: false });
    }
  },

  testProvider: async (name: string, input?: { model?: string; prompt?: string }) => {
    set({ isTesting: true, testResult: null });
    try {
      const result = await providerApi.test(name, input);
      set({ testResult: result as Record<string, unknown>, isTesting: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Provider test failed', isTesting: false });
    }
  },

  setTheme: (theme: 'light' | 'dark') => {
    if (typeof window !== 'undefined') {
      localStorage.setItem('theme', theme);
      document.documentElement.classList.toggle('dark', theme === 'dark');
    }
    set({ theme });
  },

  toggleTheme: () => {
    const newTheme = get().theme === 'dark' ? 'light' : 'dark';
    get().setTheme(newTheme);
  },

  clearError: () => set({ error: null }),
}));
