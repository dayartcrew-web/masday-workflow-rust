// ============================================================
// Analytics Store — metrics, chart data
// ============================================================

import { create } from 'zustand';
import { monitoringApi } from '@/lib/api-client';
import type { Metrics, SystemStats, HealthStatus } from '@/lib/types';

interface AnalyticsState {
  metrics: Metrics | null;
  stats: SystemStats | null;
  health: HealthStatus | null;
  isLoading: boolean;
  error: string | null;
  fetchMetrics: () => Promise<void>;
  fetchStats: () => Promise<void>;
  fetchHealth: () => Promise<void>;
  refreshAll: () => Promise<void>;
  clearError: () => void;
}

export const useAnalyticsStore = create<AnalyticsState>((set) => ({
  metrics: null,
  stats: null,
  health: null,
  isLoading: false,
  error: null,

  fetchMetrics: async () => {
    try {
      const result = await monitoringApi.getMetrics();
      set({ metrics: result });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch metrics' });
    }
  },

  fetchStats: async () => {
    try {
      const result = await monitoringApi.getStats();
      set({ stats: result });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch stats' });
    }
  },

  fetchHealth: async () => {
    try {
      const result = await monitoringApi.getHealth();
      set({ health: result });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch health' });
    }
  },

  refreshAll: async () => {
    set({ isLoading: true });
    try {
      const [metrics, stats, health] = await Promise.all([
        monitoringApi.getMetrics(),
        monitoringApi.getStats(),
        monitoringApi.getHealth(),
      ]);
      set({ metrics, stats, health, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to refresh analytics', isLoading: false });
    }
  },

  clearError: () => set({ error: null }),
}));
