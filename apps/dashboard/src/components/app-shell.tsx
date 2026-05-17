'use client';

import { useEffect, useState, type ReactNode } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/stores/auth-store';
import { useWebSocketStore } from '@/stores/websocket-store';
import { Sidebar } from '@/components/sidebar';
import { Header } from '@/components/header';

export function AppShell({ children }: { children: ReactNode }) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const checkAuth = useAuthStore((s) => s.checkAuth);
  const token = useAuthStore((s) => s.token);
  const connectWs = useWebSocketStore((s) => s.connect);
  const disconnectWs = useWebSocketStore((s) => s.disconnect);
  const router = useRouter();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    checkAuth().finally(() => setReady(true));
  }, [checkAuth]);

  useEffect(() => {
    if (isAuthenticated && token) {
      connectWs(token);
    }
    return () => {
      disconnectWs();
    };
  }, [isAuthenticated, token, connectWs, disconnectWs]);

  useEffect(() => {
    if (ready && !isAuthenticated) {
      router.replace('/login');
    }
  }, [ready, isAuthenticated, router]);

  if (!ready) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--color-bg)]">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-full animate-spin shell-loader-icon" />
          <span className="text-[var(--color-text-secondary)]">Loading...</span>
        </div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return null;
  }

  return (
    <div className="shell-root">
      <Sidebar />
      <div className="md:ml-[260px]">
        <Header />
        <main className="shell-main">
          {children}
        </main>
      </div>
    </div>
  );
}
