'use client';

import { useParams, useRouter } from 'next/navigation';
import { AppShell } from '@/components/app-shell';
import { useMemoryStore } from '@/stores/memory-store';
import { ArrowLeft } from 'lucide-react';

export default function MemoryDetailContent() {
  const params = useParams();
  const router = useRouter();
  const selectedMemory = useMemoryStore((s) => s.selectedMemory);

  if (!selectedMemory) {
    return (
      <AppShell>
        <div className="text-center py-16">
          <p className="text-[var(--text-secondary)]">Memory not found</p>
          <button onClick={() => router.push('/memory')} className="mt-4 text-brand-400 text-sm">
            Back to memory explorer
          </button>
        </div>
      </AppShell>
    );
  }

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-4">
        <div className="flex items-center gap-3">
          <button onClick={() => router.push('/memory')} className="p-2 rounded-lg hover:bg-[var(--bg-card)]">
            <ArrowLeft className="w-4 h-4 text-[var(--text-secondary)]" />
          </button>
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Memory Detail</h2>
        </div>

        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] p-4 space-y-4">
          <div className="flex items-center gap-2">
            <span className="text-xs px-2 py-0.5 rounded bg-brand-600/10 text-brand-400">{selectedMemory.memoryType}</span>
            <span className="text-xs text-[var(--text-secondary)]">ID: {selectedMemory.id}</span>
          </div>

          <div>
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-1">Summary</h3>
            <p className="text-[var(--text-primary)]">{selectedMemory.summary}</p>
          </div>

          <div>
            <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-1">Content</h3>
            <div className="bg-[var(--bg-secondary)] rounded-lg p-3 text-sm text-[var(--text-primary)] whitespace-pre-wrap">
              {selectedMemory.content}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-[var(--text-secondary)]">Importance:</span>{' '}
              <span className="text-[var(--text-primary)]">{selectedMemory.importance}</span>
            </div>
            <div>
              <span className="text-[var(--text-secondary)]">Created:</span>{' '}
              <span className="text-[var(--text-primary)]">
                {selectedMemory.createdAt ? new Date(selectedMemory.createdAt).toLocaleString() : '-'}
              </span>
            </div>
            {selectedMemory.taskId && (
              <div>
                <span className="text-[var(--text-secondary)]">Task:</span>{' '}
                <span className="text-brand-400">{selectedMemory.taskId}</span>
              </div>
            )}
            {selectedMemory.workflowId && (
              <div>
                <span className="text-[var(--text-secondary)]">Workflow:</span>{' '}
                <span className="text-brand-400">{selectedMemory.workflowId}</span>
              </div>
            )}
          </div>

          {selectedMemory.score && (
            <div>
              <h3 className="text-sm font-medium text-[var(--text-secondary)] mb-2">Score Breakdown</h3>
              <div className="space-y-1 text-sm">
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Similarity</span><span>{selectedMemory.score.similarity.toFixed(3)}</span></div>
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Recency</span><span>{selectedMemory.score.recency.toFixed(3)}</span></div>
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Importance</span><span>{selectedMemory.score.importance.toFixed(3)}</span></div>
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Usage</span><span>{selectedMemory.score.usage.toFixed(3)}</span></div>
                <div className="flex justify-between font-medium pt-1 border-t border-[var(--border)]">
                  <span className="text-[var(--text-secondary)]">Total</span>
                  <span className="text-brand-400">{selectedMemory.score.total.toFixed(3)}</span>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </AppShell>
  );
}
