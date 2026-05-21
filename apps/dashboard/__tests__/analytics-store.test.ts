import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAnalyticsStore } from '@/stores/analytics-store';
import { monitoringApi } from '@/lib/api-client';

vi.mock('@/lib/api-client', () => ({
  monitoringApi: {
    getMetrics: vi.fn(),
    getStats: vi.fn(),
    getHealth: vi.fn(),
  },
}));

describe('useAnalyticsStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAnalyticsStore.setState({
      metrics: null,
      stats: null,
      health: null,
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with null metrics, stats, and health', () => {
    const state = useAnalyticsStore.getState();
    expect(state.metrics).toBeNull();
    expect(state.stats).toBeNull();
    expect(state.health).toBeNull();
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetchMetrics loads metrics data', async () => {
    const mockMetrics = { totalWorkflows: 10, successRate: 0.95, avgDuration: 120 };
    (monitoringApi.getMetrics as ReturnType<typeof vi.fn>).mockResolvedValue(mockMetrics);

    await useAnalyticsStore.getState().fetchMetrics();

    const state = useAnalyticsStore.getState();
    expect(state.metrics).toEqual(mockMetrics);
  });

  it('fetchMetrics sets error on failure', async () => {
    (monitoringApi.getMetrics as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Failed to fetch metrics'));

    await useAnalyticsStore.getState().fetchMetrics();

    expect(useAnalyticsStore.getState().error).toBe('Failed to fetch metrics');
  });

  it('fetchStats loads stats data', async () => {
    const mockStats = { cpu: 45, memory: 60, disk: 70 };
    (monitoringApi.getStats as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);

    await useAnalyticsStore.getState().fetchStats();

    expect(useAnalyticsStore.getState().stats).toEqual(mockStats);
  });

  it('fetchHealth loads health data', async () => {
    const mockHealth = { status: 'healthy', checks: [] };
    (monitoringApi.getHealth as ReturnType<typeof vi.fn>).mockResolvedValue(mockHealth);

    await useAnalyticsStore.getState().fetchHealth();

    expect(useAnalyticsStore.getState().health).toEqual(mockHealth);
  });

  it('refreshAll loads all data in parallel', async () => {
    const mockMetrics = { totalWorkflows: 10 };
    const mockStats = { cpu: 45 };
    const mockHealth = { status: 'healthy' };

    (monitoringApi.getMetrics as ReturnType<typeof vi.fn>).mockResolvedValue(mockMetrics);
    (monitoringApi.getStats as ReturnType<typeof vi.fn>).mockResolvedValue(mockStats);
    (monitoringApi.getHealth as ReturnType<typeof vi.fn>).mockResolvedValue(mockHealth);

    await useAnalyticsStore.getState().refreshAll();

    const state = useAnalyticsStore.getState();
    expect(state.metrics).toEqual(mockMetrics);
    expect(state.stats).toEqual(mockStats);
    expect(state.health).toEqual(mockHealth);
    expect(state.isLoading).toBe(false);
  });

  it('clearError resets error state', () => {
    useAnalyticsStore.setState({ error: 'Analytics error' });

    useAnalyticsStore.getState().clearError();

    expect(useAnalyticsStore.getState().error).toBeNull();
  });
});
