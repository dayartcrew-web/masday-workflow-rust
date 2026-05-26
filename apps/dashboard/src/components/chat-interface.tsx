'use client';

import { useState, useRef, useEffect } from 'react';
import { Send } from 'lucide-react';
import type { ChatMessage, MemoryEntry } from '@/lib/types';

interface ChatInterfaceProps {
  onSend: (message: string) => Promise<void>;
  messages: ChatMessage[];
  isLoading: boolean;
  memoryContext?: MemoryEntry[];
}

export function ChatInterface({ onSend, messages, isLoading, memoryContext }: ChatInterfaceProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const [showContext, setShowContext] = useState(false);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;
    const msg = input.trim();
    setInput('');
    await onSend(msg);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Memory context toggle */}
      {memoryContext && memoryContext.length > 0 && (
        <div className="border-b border-[var(--border)] px-3 md:px-4 py-2">
          <button
            onClick={() => setShowContext(!showContext)}
            className="text-xs text-brand-400 hover:text-brand-300"
          >
            {showContext ? 'Hide' : 'Show'} memory context ({memoryContext.length} entries)
          </button>
          {showContext && (
            <div className="mt-2 space-y-1 max-h-32 overflow-y-auto">
              {memoryContext.map((m) => (
                <div key={m.id} className="text-xs bg-[var(--bg-secondary)] rounded p-2">
                  <span className="text-brand-400 font-medium">{m.memoryType}</span>: {m.summary}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-3 md:p-4 space-y-3 md:space-y-4">
        {messages.length === 0 && (
          <div className="text-center text-[var(--text-secondary)] text-sm py-8">
            Send a message to start chatting
          </div>
        )}
        {messages.map((msg, idx) => (
          <div
            key={idx}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[85%] md:max-w-[80%] rounded-lg px-3 md:px-4 py-2 text-sm ${
                msg.role === 'user'
                  ? 'bg-brand-600 text-white'
                  : 'bg-[var(--bg-card)] border border-[var(--border)] text-[var(--text-primary)]'
              }`}
            >
              <p className="whitespace-pre-wrap">{msg.content}</p>
              <span className={`text-[10px] mt-1 block ${msg.role === 'user' ? 'text-brand-200' : 'text-[var(--text-secondary)]'}`}>
                {new Date(msg.timestamp).toLocaleTimeString()}
              </span>
            </div>
          </div>
        ))}
        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-[var(--bg-card)] border border-[var(--border)] rounded-lg px-4 py-2">
              <div className="flex gap-1">
                <span className="w-2 h-2 bg-[var(--text-secondary)] rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                <span className="w-2 h-2 bg-[var(--text-secondary)] rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                <span className="w-2 h-2 bg-[var(--text-secondary)] rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input - sticky at bottom on mobile */}
      <div className="border-t border-[var(--border)] p-3 md:p-4 sticky bottom-0 bg-[var(--bg-card)]">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && handleSend()}
            placeholder="Type a message..."
            disabled={isLoading}
            className="flex-1 px-3 md:px-4 py-2.5 md:py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-base md:text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 disabled:opacity-50"
          />
          <button
            onClick={handleSend}
            disabled={isLoading || !input.trim()}
            className="px-3 md:px-4 py-2.5 md:py-2 rounded-lg bg-brand-600 text-white text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
