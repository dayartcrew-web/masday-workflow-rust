// ============================================================
// Memory Store — memories, search results, filters
// ============================================================

import { create } from 'zustand';
import { memoryApi, searchApi } from '@/lib/api-client';
import type { MemoryEntry } from '@/lib/types';

interface MemoryState {
  memories: MemoryEntry[];
  searchResults: MemoryEntry[];
  selectedMemory: MemoryEntry | null;
  searchQuery: string;
  filterType: string;
  isLoading: boolean;
  error: string | null;
  fetchMemories: (workflowId: string, limit?: number) => Promise<void>;
  fetchRecent: (workflowId: string, limit?: number) => Promise<void>;
  fetchByType: (workflowId: string, type: string) => Promise<void>;
  fetchByTask: (taskId: string) => Promise<void>;
  search: (query: string, options?: { glob?: string; type?: string; limit?: number }) => Promise<void>;
  selectMemory: (memory: MemoryEntry | null) => void;
  storeMemory: (entry: { memoryType: string; summary: string; content: string; importance?: number; taskId?: string; workflowId?: string }) => Promise<string>;
  updateMemory: (id: string, updates: Record<string, unknown>) => Promise<void>;
  deleteMemory: (id: string) => Promise<void>;
  setSearchQuery: (query: string) => void;
  setFilterType: (type: string) => void;
  clearError: () => void;
}

export const useMemoryStore = create<MemoryState>((set) => ({
  memories: [],
  searchResults: [],
  selectedMemory: null,
  searchQuery: '',
  filterType: '',
  isLoading: false,
  error: null,

  fetchMemories: async (workflowId: string, limit?: number) => {
    set({ isLoading: true });
    try {
      const result = await memoryApi.recallDocuments(workflowId, limit);
      set({ memories: result.documents as MemoryEntry[], isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch memories', isLoading: false });
    }
  },

  fetchRecent: async (workflowId: string, limit?: number) => {
    set({ isLoading: true });
    try {
      const result = await memoryApi.recallRecent(workflowId, limit);
      set({ memories: result.memories, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch recent memories', isLoading: false });
    }
  },

  fetchByType: async (workflowId: string, type: string) => {
    set({ isLoading: true, filterType: type });
    try {
      const result = await memoryApi.recallByType(workflowId, type);
      set({ memories: result.memories, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch memories by type', isLoading: false });
    }
  },

  fetchByTask: async (taskId: string) => {
    set({ isLoading: true });
    try {
      const result = await memoryApi.recallByTask(taskId);
      set({ memories: result.memories, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch task memories', isLoading: false });
    }
  },

  search: async (query: string, options?: { glob?: string; type?: string; limit?: number }) => {
    set({ isLoading: true, searchQuery: query });
    try {
      const result = await searchApi.codeSearch({ query, ...options });
      set({ searchResults: (result as { results?: MemoryEntry[] }).results || [], isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Search failed', isLoading: false });
    }
  },

  selectMemory: (memory) => set({ selectedMemory: memory }),

  storeMemory: async (entry) => {
    const result = await memoryApi.store(entry);
    return result.id;
  },

  updateMemory: async (id: string, updates: Record<string, unknown>) => {
    await memoryApi.update(id, updates);
    set((s) => ({
      memories: s.memories.map((m) => m.id === id ? { ...m, ...updates } : m),
    }));
  },

  deleteMemory: async (id: string) => {
    await memoryApi.delete(id);
    set((s) => ({
      memories: s.memories.filter((m) => m.id !== id),
    }));
  },

  setSearchQuery: (query: string) => set({ searchQuery: query }),
  setFilterType: (type: string) => set({ filterType: type }),
  clearError: () => set({ error: null }),
}));
