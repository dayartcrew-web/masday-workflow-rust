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
  isSaving: boolean;
  isLoading: boolean;
  error: string | null;
  theme: 'light' | 'dark';
  fetchProviders: () => Promise<void>;
  testProvider: (name: string, input?: { model?: string; prompt?: string }) => Promise<void>;
  saveProvider: (input: { providerName: string; providerType: string; baseUrl: string; apiKey: string; models: string[]; isDefault?: boolean }) => Promise<boolean>;
  deleteProvider: (providerName: string) => Promise<boolean>;
  setDefaultProvider: (providerName: string) => Promise<boolean>;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleTheme: () => void;
  clearError: () => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  providers: [],
  testResult: null,
  isTesting: false,
  isSaving: false,
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

  saveProvider: async (input) => {
    set({ isSaving: true });
    try {
      await providerApi.save(input);
      await get().fetchProviders();
      set({ isSaving: false });
      return true;
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to save provider', isSaving: false });
      return false;
    }
  },

  deleteProvider: async (providerName) => {
    set({ isSaving: true });
    try {
      await providerApi.delete(providerName);
      await get().fetchProviders();
      set({ isSaving: false });
      return true;
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to delete provider', isSaving: false });
      return false;
    }
  },

  setDefaultProvider: async (providerName) => {
    try {
      await providerApi.setDefault(providerName);
      await get().fetchProviders();
      return true;
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to set default' });
      return false;
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
