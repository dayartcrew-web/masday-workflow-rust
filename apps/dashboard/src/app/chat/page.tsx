'use client';

import { useState, useEffect } from 'react';
import { AppShell } from '@/components/app-shell';
import { ChatInterface } from '@/components/chat-interface';
import { chatApi, providerApi } from '@/lib/api-client';
import { useAuthStore } from '@/stores/auth-store';
import type { ChatMessage, MemoryEntry, ProviderInfo } from '@/lib/types';

export default function ChatPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [memoryContext, setMemoryContext] = useState<MemoryEntry[]>([]);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState('');
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  useEffect(() => {
    if (!isAuthenticated) return;
    providerApi.list().then((result) => {
      const list = result.providers || [];
      setProviders(list);
      const def = list.find((p) => p.isDefault);
      if (def?.models?.length) {
        setSelectedModel(def.models[0]);
      }
    }).catch(() => {});
  }, [isAuthenticated]);

  const modelOptions = providers.flatMap((p) =>
    (p.models || []).map((m) => ({ provider: p.name, model: m, isDefault: p.isDefault }))
  );

  const handleSend = async (message: string) => {
    const userMsg: ChatMessage = {
      role: 'user',
      content: message,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setIsLoading(true);

    try {
      const result = await chatApi.complete({ message, model: selectedModel || undefined }) as { text?: string; response?: string; error?: string; ok?: boolean; memoryContext?: MemoryEntry[] };
      const assistantMsg: ChatMessage = {
        role: 'assistant',
        content: result.error ? `Error: ${result.error}` : (result.text || result.response || 'No response received'),
        timestamp: new Date().toISOString(),
        memoryContext: result.memoryContext,
      };
      setMessages((prev) => [...prev, assistantMsg]);
      if (result.memoryContext) {
        setMemoryContext(result.memoryContext);
      }
    } catch (err: unknown) {
      const errorMsg: ChatMessage = {
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : 'Failed to get response'}`,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, errorMsg]);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell>
      <div className="max-w-3xl mx-auto h-[calc(100vh-6rem)] md:h-[calc(100vh-8rem)] flex flex-col">
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] flex-1 flex flex-col overflow-hidden">
          <div className="px-3 md:px-4 py-3 border-b border-[var(--border)] flex items-center justify-between gap-3">
            <div>
              <h2 className="text-sm font-medium text-[var(--text-primary)]">Chat with AI Agent</h2>
              <p className="text-xs text-[var(--text-secondary)]">Uses default LLM provider with memory context</p>
            </div>
            {modelOptions.length > 0 && (
              <select
                title="Select model"
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                className="px-2.5 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-xs focus:outline-none focus:ring-2 focus:ring-brand-500 max-w-[200px]"
              >
                {modelOptions.map((opt) => (
                  <option key={`${opt.provider}-${opt.model}`} value={opt.model}>
                    {opt.model} ({opt.provider}{opt.isDefault ? ' *' : ''})
                  </option>
                ))}
              </select>
            )}
          </div>
          <ChatInterface
            onSend={handleSend}
            messages={messages}
            isLoading={isLoading}
            memoryContext={memoryContext}
          />
        </div>
      </div>
    </AppShell>
  );
}
