import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSettingsStore } from '@/stores/settings-store';
import { providerApi } from '@/lib/api-client';

vi.mock('@/lib/api-client', () => ({
  providerApi: {
    list: vi.fn(),
    test: vi.fn(),
  },
}));

describe('useSettingsStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      providers: [],
      testResult: null,
      isTesting: false,
      isLoading: false,
      error: null,
      theme: 'dark',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with default settings', () => {
    const state = useSettingsStore.getState();
    expect(state.providers).toEqual([]);
    expect(state.testResult).toBeNull();
    expect(state.isTesting).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.theme).toBe('dark');
  });

  it('fetchProviders loads provider list', async () => {
    const mockProviders = [
      { name: 'openai', type: 'openai', models: ['gpt-4'], status: 'available', circuitState: 'closed' },
      { name: 'anthropic', type: 'anthropic', models: ['claude'], status: 'available', circuitState: 'closed' },
    ];
    (providerApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({ providers: mockProviders });

    await useSettingsStore.getState().fetchProviders();

    expect(useSettingsStore.getState().providers).toEqual(mockProviders);
    expect(useSettingsStore.getState().isLoading).toBe(false);
  });

  it('fetchProviders sets error on failure', async () => {
    (providerApi.list as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Failed to fetch providers'));

    await useSettingsStore.getState().fetchProviders();

    expect(useSettingsStore.getState().error).toBe('Failed to fetch providers');
  });

  it('testProvider runs provider test', async () => {
    const mockResult = { status: 'success', latency: 120 };
    (providerApi.test as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult);

    await useSettingsStore.getState().testProvider('openai', { model: 'gpt-4' });

    expect(useSettingsStore.getState().testResult).toEqual(mockResult);
    expect(useSettingsStore.getState().isTesting).toBe(false);
  });

  it('setTheme updates theme', () => {
    useSettingsStore.getState().setTheme('light');

    expect(useSettingsStore.getState().theme).toBe('light');
  });

  it('toggleTheme switches between light and dark', () => {
    useSettingsStore.setState({ theme: 'dark' });
    useSettingsStore.getState().toggleTheme();
    expect(useSettingsStore.getState().theme).toBe('light');

    useSettingsStore.setState({ theme: 'light' });
    useSettingsStore.getState().toggleTheme();
    expect(useSettingsStore.getState().theme).toBe('dark');
  });

  it('clearError resets error', () => {
    useSettingsStore.setState({ error: 'Settings error' });

    useSettingsStore.getState().clearError();

    expect(useSettingsStore.getState().error).toBeNull();
  });
});
