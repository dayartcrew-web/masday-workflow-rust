'use client';

import { useState } from 'react';
import { Search, Filter } from 'lucide-react';
import type { MemoryEntry, ScoreBreakdown } from '@/lib/types';

interface MemorySearchProps {
  onSearch: (query: string) => void;
  results: MemoryEntry[];
  isLoading: boolean;
}

function ScoreBar({ label, value, max = 1 }: { label: string; value: number; max?: number }) {
  const pct = Math.round((value / max) * 100);
  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="w-16 md:w-20 text-[var(--text-secondary)] flex-shrink-0">{label}</span>
      <div className="flex-1 h-1.5 bg-[var(--border)] rounded-full overflow-hidden">
        <div className="h-full bg-brand-500 rounded-full" style={{ width: `${pct}%` }} />
      </div>
      <span className="w-10 text-right text-[var(--text-secondary)]">{(value).toFixed(2)}</span>
    </div>
  );
}

export function MemorySearch({ onSearch, results, isLoading }: MemorySearchProps) {
  const [query, setQuery] = useState('');
  const [expanded, setExpanded] = useState<string | null>(null);

  const handleSearch = () => {
    if (query.trim()) onSearch(query.trim());
  };

  return (
    <div className="space-y-4">
      {/* Search input */}
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-secondary)]" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Search memories..."
            className="w-full pl-10 pr-4 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
          />
        </div>
        <button
          onClick={handleSearch}
          disabled={isLoading}
          className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors"
        >
          {isLoading ? 'Searching...' : 'Search'}
        </button>
      </div>

      {/* Results */}
      <div className="space-y-2">
        {results.length === 0 && query && !isLoading && (
          <p className="text-sm text-[var(--text-secondary)] text-center py-4">No results found</p>
        )}
        {results.map((memory) => (
          <div
            key={memory.id}
            className="rounded-lg border border-[var(--border)] bg-[var(--bg-card)] p-3 cursor-pointer hover:border-brand-600/30 transition-colors"
            onClick={() => setExpanded(expanded === memory.id ? null : memory.id)}
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs px-1.5 py-0.5 rounded bg-brand-600/10 text-brand-400">{memory.memoryType}</span>
                  {memory.score && (
                    <span className="text-xs text-[var(--text-secondary)]">
                      Score: {memory.score.total.toFixed(3)}
                    </span>
                  )}
                </div>
                <p className="text-sm text-[var(--text-primary)] truncate">{memory.summary}</p>
              </div>
              <Filter className="w-3 h-3 text-[var(--text-secondary)] flex-shrink-0 mt-1" />
            </div>

            {/* Expanded score breakdown */}
            {expanded === memory.id && (
              <div className="mt-3 pt-3 border-t border-[var(--border)] space-y-3">
                <p className="text-xs text-[var(--text-primary)] whitespace-pre-wrap">{memory.content}</p>
                {memory.score && (
                  <div className="space-y-1">
                    <p className="text-xs font-medium text-[var(--text-secondary)]">Score Breakdown</p>
                    <ScoreBar label="Similarity" value={memory.score.similarity} />
                    <ScoreBar label="Recency" value={memory.score.recency} />
                    <ScoreBar label="Importance" value={memory.score.importance} />
                    <ScoreBar label="Usage" value={memory.score.usage} />
                    <div className="flex items-center gap-2 text-xs pt-1">
                      <span className="w-20 font-medium text-[var(--text-primary)]">Total</span>
                      <span className="text-brand-400 font-medium">{memory.score.total.toFixed(3)}</span>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
