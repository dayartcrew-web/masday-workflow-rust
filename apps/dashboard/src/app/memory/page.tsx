'use client';

import { useEffect, useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { MemorySearch } from '@/components/memory-search';
import { SelectRoot, SelectTrigger, SelectContent, SelectItem, SelectValue } from '@/components/ui/select';
import { useMemoryStore } from '@/stores/memory-store';
import { useWorkflowStore } from '@/stores/workflow-store';
import { useAuthStore } from '@/stores/auth-store';
import { RefreshCw } from 'lucide-react';

export default function MemoryExplorerPage() {
  const memories = useMemoryStore((s) => s.memories);
  const fetchMemories = useMemoryStore((s) => s.fetchMemories);
  const search = useMemoryStore((s) => s.search);
  const searchResults = useMemoryStore((s) => s.searchResults);
  const isLoading = useMemoryStore((s) => s.isLoading);
  const workflows = useWorkflowStore((s) => s.workflows);
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const [selectedWorkflow, setSelectedWorkflow] = useState('');
  const [activeTab, setActiveTab] = useState<'browse' | 'search'>('browse');

  useEffect(() => {
    if (!isAuthenticated) return;
    fetchWorkflows();
  }, [isAuthenticated, fetchWorkflows]);

  useEffect(() => {
    if (selectedWorkflow) {
      fetchMemories(selectedWorkflow);
    }
  }, [selectedWorkflow, fetchMemories]);

  const displayMemories = activeTab === 'search' ? searchResults : memories;

  return (
    <AppShell>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Memory Explorer</h2>
          <button
            onClick={() => selectedWorkflow && fetchMemories(selectedWorkflow)}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-card)] transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {/* Workflow selector + tabs */}
        <div className="flex items-center gap-3">
          <SelectRoot value={selectedWorkflow} onValueChange={setSelectedWorkflow}>
            <SelectTrigger className="flex-1">
              <SelectValue placeholder="Select workflow..." />
            </SelectTrigger>
            <SelectContent>
              {workflows.map((w) => (
                <SelectItem key={w.id} value={w.id}>{w.name}</SelectItem>
              ))}
            </SelectContent>
          </SelectRoot>
          <div className="flex gap-1">
            <button
              onClick={() => setActiveTab('browse')}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                activeTab === 'browse' ? 'bg-brand-600 text-white' : 'bg-[var(--bg-card)] text-[var(--text-secondary)]'
              }`}
            >
              Browse
            </button>
            <button
              onClick={() => setActiveTab('search')}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                activeTab === 'search' ? 'bg-brand-600 text-white' : 'bg-[var(--bg-card)] text-[var(--text-secondary)]'
              }`}
            >
              Search
            </button>
          </div>
        </div>

        {/* Search or browse */}
        {activeTab === 'search' ? (
          <MemorySearch
            onSearch={(query) => search(query)}
            results={searchResults}
            isLoading={isLoading}
          />
        ) : (
          <div className="space-y-2">
            {displayMemories.length === 0 ? (
              <div className="text-center py-8 text-[var(--text-secondary)]">
                {selectedWorkflow ? 'No memories found for this workflow' : 'Select a workflow to browse memories'}
              </div>
            ) : (
              displayMemories.map((memory) => (
                <div
                  key={memory.id}
                  className="rounded-lg border border-[var(--border)] bg-[var(--bg-card)] p-3"
                >
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs px-1.5 py-0.5 rounded bg-brand-600/10 text-brand-400">
                      {memory.memoryType}
                    </span>
                    <span className="text-xs text-[var(--text-secondary)]">
                      Importance: {memory.importance}
                    </span>
                  </div>
                  <p className="text-sm text-[var(--text-primary)]">{memory.summary}</p>
                  <p className="text-xs text-[var(--text-secondary)] mt-1 line-clamp-2">{memory.content}</p>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </AppShell>
  );
}
