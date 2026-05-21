import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useMemoryStore } from '@/stores/memory-store';
import { memoryApi, searchApi } from '@/lib/api-client';

vi.mock('@/lib/api-client', () => ({
  memoryApi: {
    recallDocuments: vi.fn(),
    recallRecent: vi.fn(),
    recallByType: vi.fn(),
    recallByTask: vi.fn(),
    store: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
  },
  searchApi: {
    codeSearch: vi.fn(),
  },
}));

describe('useMemoryStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useMemoryStore.setState({
      memories: [],
      searchResults: [],
      selectedMemory: null,
      searchQuery: '',
      filterType: '',
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with empty memories and default state', () => {
    const state = useMemoryStore.getState();
    expect(state.memories).toEqual([]);
    expect(state.searchResults).toEqual([]);
    expect(state.selectedMemory).toBeNull();
    expect(state.searchQuery).toBe('');
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetchMemories loads documents', async () => {
    const mockDocs = [{ id: '1', content: 'test memory', type: 'note' }];
    (memoryApi.recallDocuments as ReturnType<typeof vi.fn>).mockResolvedValue({ documents: mockDocs });

    await useMemoryStore.getState().fetchMemories('wf-1');

    expect(useMemoryStore.getState().memories).toEqual(mockDocs);
    expect(useMemoryStore.getState().isLoading).toBe(false);
  });

  it('fetchMemories sets error on failure', async () => {
    (memoryApi.recallDocuments as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Failed to fetch memories'));

    await useMemoryStore.getState().fetchMemories('wf-1');

    expect(useMemoryStore.getState().error).toBe('Failed to fetch memories');
  });

  it('fetchRecent loads recent memories', async () => {
    const mockRecent = [{ id: '2', content: 'recent' }];
    (memoryApi.recallRecent as ReturnType<typeof vi.fn>).mockResolvedValue({ memories: mockRecent });

    await useMemoryStore.getState().fetchRecent('wf-1');

    expect(useMemoryStore.getState().memories).toEqual(mockRecent);
  });

  it('search stores results in searchResults', async () => {
    const mockResults = [{ id: '3', content: 'search result' }];
    (searchApi.codeSearch as ReturnType<typeof vi.fn>).mockResolvedValue({ results: mockResults });

    await useMemoryStore.getState().search('test query');

    expect(useMemoryStore.getState().searchResults).toEqual(mockResults);
    expect(useMemoryStore.getState().searchQuery).toBe('test query');
  });

  it('storeMemory returns memory id', async () => {
    (memoryApi.store as ReturnType<typeof vi.fn>).mockResolvedValue({ id: 'new-id' });

    const id = await useMemoryStore.getState().storeMemory({
      memoryType: 'decision',
      summary: 'test',
      content: 'stored',
    });

    expect(id).toBe('new-id');
  });

  it('deleteMemory removes from memories list', async () => {
    useMemoryStore.setState({
      memories: [{ id: '1', content: 'old' }, { id: '2', content: 'keep' }],
    });
    (memoryApi.delete as ReturnType<typeof vi.fn>).mockResolvedValue({});

    await useMemoryStore.getState().deleteMemory('1');

    const state = useMemoryStore.getState();
    expect(state.memories).not.toContainEqual({ id: '1', content: 'old' });
    expect(state.memories).toContainEqual({ id: '2', content: 'keep' });
  });

  it('selectMemory updates selected memory', () => {
    const memory = { id: '1', content: 'selected' };
    useMemoryStore.getState().selectMemory(memory);

    expect(useMemoryStore.getState().selectedMemory).toEqual(memory);
  });

  it('setSearchQuery updates search query', () => {
    useMemoryStore.getState().setSearchQuery('new query');

    expect(useMemoryStore.getState().searchQuery).toBe('new query');
  });

  it('setFilterType updates filter type', () => {
    useMemoryStore.getState().setFilterType('decision');

    expect(useMemoryStore.getState().filterType).toBe('decision');
  });

  it('clearError resets error', () => {
    useMemoryStore.setState({ error: 'Error occurred' });

    useMemoryStore.getState().clearError();

    expect(useMemoryStore.getState().error).toBeNull();
  });
});
