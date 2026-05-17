'use client';

import { useState } from 'react';
import { AppShell } from '@/components/app-shell';
import { ChatInterface } from '@/components/chat-interface';
import { chatApi } from '@/lib/api-client';
import { useMemoryStore } from '@/stores/memory-store';
import type { ChatMessage, MemoryEntry } from '@/lib/types';

export default function ChatPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [memoryContext, setMemoryContext] = useState<MemoryEntry[]>([]);

  const handleSend = async (message: string) => {
    const userMsg: ChatMessage = {
      role: 'user',
      content: message,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setIsLoading(true);

    try {
      const result = await chatApi.complete({ message }) as { response: string; memoryContext?: MemoryEntry[] };
      const assistantMsg: ChatMessage = {
        role: 'assistant',
        content: result.response || 'No response received',
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
      <div className="max-w-3xl mx-auto h-[calc(100vh-8rem)] flex flex-col">
        <div className="rounded-xl border border-[var(--border)] bg-[var(--bg-card)] flex-1 flex flex-col overflow-hidden">
          <div className="px-4 py-3 border-b border-[var(--border)]">
            <h2 className="text-sm font-medium text-[var(--text-primary)]">Chat with AI Agent</h2>
            <p className="text-xs text-[var(--text-secondary)]">Uses LLM providers with memory context</p>
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
