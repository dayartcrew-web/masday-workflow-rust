'use client';

import { useState } from 'react';
import { usePathname } from 'next/navigation';
import { Moon, Sun, LogOut, User, Menu } from 'lucide-react';
import { useAuthStore } from '@/stores/auth-store';
import { useSettingsStore } from '@/stores/settings-store';
import { useWebSocketStore } from '@/stores/websocket-store';
import { MobileDrawer } from '@/components/mobile-drawer';
import Link from 'next/link';

const pageTitles: Record<string, string> = {
  '/': 'Dashboard',
  '/workflows': 'Workflows',
  '/tasks': 'Tasks',
  '/memory': 'Memory Explorer',
  '/graph': 'Knowledge Graph',
  '/chat': 'Chat',
  '/agent': 'Agent Trace',
  '/analytics': 'Analytics',
  '/providers': 'Providers',
  '/policy': 'Policy',
  '/audit': 'Audit',
  '/settings': 'Settings',
};

export function Header() {
  const pathname = usePathname();
  const theme = useSettingsStore((s) => s.theme);
  const toggleTheme = useSettingsStore((s) => s.toggleTheme);
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const connected = useWebSocketStore((s) => s.connected);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const title = pageTitles[pathname] || 'Masday Workflow';

  return (
    <>
      <MobileDrawer open={drawerOpen} onClose={() => setDrawerOpen(false)} />
      <header className="header-root flex items-center justify-between px-4 md:px-6 sticky top-0 z-30">
        <div className="flex items-center gap-3">
          {/* Mobile menu button area */}
          <Link href="/" className="md:hidden flex items-center justify-center min-w-[44px] min-h-[44px]">
            <div className="w-7 h-7 flex items-center justify-center sidebar-logo-icon rounded-lg">
              <span className="text-white font-bold text-xs">M</span>
            </div>
          </Link>
          <h2 className="text-lg font-semibold text-[var(--color-text)]">{title}</h2>
          {connected && (
            <span className="hidden sm:inline-flex items-center gap-1.5 text-xs text-[var(--color-neon-green)]">
              <span className="header-live-dot" />
              Live
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setDrawerOpen(true)}
            className="md:hidden min-h-[44px] min-w-[44px] flex items-center justify-center rounded-lg header-menu-btn focus-ring"
            aria-label="Open navigation menu"
          >
            <Menu className="w-5 h-5" />
          </button>
          <button
            onClick={toggleTheme}
            className="min-h-[44px] min-w-[44px] flex items-center justify-center rounded-lg header-theme-btn focus-ring"
            aria-label="Toggle theme"
          >
            {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </button>

          {user && (
            <div className="flex items-center gap-2">
              <div className="hidden sm:flex items-center gap-2 text-sm text-[var(--color-text-secondary)]">
                <User className="w-4 h-4" />
                <span>{user.name}</span>
                <span className="text-xs px-2 py-0.5 rounded-md header-user-badge">
                  {user.role}
                </span>
              </div>
              <button
                onClick={() => {
                  logout();
                  window.location.href = '/login';
                }}
                className="min-h-[44px] min-w-[44px] flex items-center justify-center rounded-lg header-logout-btn focus-ring"
                aria-label="Logout"
              >
                <LogOut className="w-4 h-4" />
              </button>
            </div>
          )}
        </div>
      </header>
    </>
  );
}
