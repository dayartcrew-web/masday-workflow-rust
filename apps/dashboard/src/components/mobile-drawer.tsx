'use client';

import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  Menu,
  X,
  BarChart3,
  Server,
  Shield,
  Settings,
  ClipboardCheck,
  Network,
  Bot,
  User,
  LogOut,
} from 'lucide-react';
import { useAuthStore } from '@/stores/auth-store';

const systemNavItems = [
  { href: '/analytics', label: 'Analytics', icon: BarChart3 },
  { href: '/providers', label: 'Providers', icon: Server },
  { href: '/policy', label: 'Policy', icon: Shield },
  { href: '/audit', label: 'Audit', icon: ClipboardCheck },
  { href: '/graph', label: 'Graph', icon: Network },
  { href: '/agent', label: 'Agent', icon: Bot },
  { href: '/settings', label: 'Settings', icon: Settings },
];

export function MobileDrawer() {
  const [isOpen, setIsOpen] = useState(false);
  const pathname = usePathname();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);

  useEffect(() => {
    setIsOpen(false);
  }, [pathname]);

  useEffect(() => {
    document.body.style.overflow = isOpen ? 'hidden' : '';
    return () => { document.body.style.overflow = ''; };
  }, [isOpen]);

  const handleLogout = () => {
    logout();
    window.location.href = '/login';
  };

  return (
    <>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="md:hidden p-2 rounded-lg transition-colors duration-150 hover:bg-[var(--color-surface-elevated)] active:bg-[var(--color-surface-elevated)]"
        aria-label={isOpen ? 'Close menu' : 'Open menu'}
      >
        <Menu className="w-5 h-5 text-[var(--color-text)]" />
      </button>

      {/* Backdrop */}
      <div
        className={`md:hidden fixed inset-0 z-40 bg-black/60 backdrop-blur-sm transition-opacity duration-300 ${
          isOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
        }`}
        onClick={() => setIsOpen(false)}
        aria-hidden="true"
      />

      {/* Drawer panel */}
      <div
        className={`md:hidden fixed top-0 right-0 z-50 h-full w-72 transform transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] ${
          isOpen ? 'translate-x-0' : 'translate-x-full'
        }`}
        style={{
          background: 'var(--color-surface)',
          borderLeft: '1px solid var(--color-border-subtle)',
          boxShadow: isOpen ? '-8px 0 32px rgba(0,0,0,0.4)' : 'none',
        }}
      >
        <div className="flex flex-col h-full">
          {/* Header */}
          <div className="flex items-center justify-between px-4 h-14 border-b border-[var(--color-border-subtle)]">
            <span className="text-xs font-semibold text-[var(--color-text-secondary)] uppercase tracking-widest" style={{ fontFamily: 'var(--font-family-mono)' }}>
              Navigate
            </span>
            <button
              onClick={() => setIsOpen(false)}
              className="p-1.5 rounded-lg hover:bg-[var(--color-surface-elevated)] transition-colors"
              aria-label="Close menu"
            >
              <X className="w-4 h-4 text-[var(--color-text-secondary)]" />
            </button>
          </div>

          {/* Links */}
          <nav className="flex-1 overflow-y-auto py-2 px-3 scrollbar-thin">
            <ul className="space-y-0.5">
              {systemNavItems.map((item) => {
                const isActive =
                  pathname === item.href ||
                  (item.href !== '/' && pathname.startsWith(item.href));
                return (
                  <li key={item.href}>
                    <Link
                      href={item.href}
                      className={`flex items-center gap-3 px-3 py-2.5 text-sm rounded-lg transition-colors duration-150 ${
                        isActive
                          ? 'bg-[var(--color-surface-elevated)] text-[var(--color-primary)] font-medium'
                          : 'text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-surface-elevated)]'
                      }`}
                    >
                      <item.icon className="w-[16px] h-[16px] flex-shrink-0 opacity-70" />
                      {item.label}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </nav>

          {/* User footer */}
          {user && (
            <div className="px-4 py-3 border-t border-[var(--color-border-subtle)]">
              <div className="flex items-center gap-3 mb-2">
                <div className="w-8 h-8 rounded-full bg-[var(--color-surface-elevated)] flex items-center justify-center text-xs font-semibold text-[var(--color-primary)]" style={{ fontFamily: 'var(--font-family-mono)' }}>
                  {user.name?.charAt(0)?.toUpperCase() || '?'}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium text-[var(--color-text)] truncate">
                    {user.name}
                  </p>
                  <p className="text-[11px] text-[var(--color-text-secondary)] truncate" style={{ fontFamily: 'var(--font-family-mono)' }}>
                    {user.email}
                  </p>
                </div>
              </div>
              <button
                onClick={handleLogout}
                className="flex items-center gap-2 w-full px-3 py-2 text-sm text-[var(--color-text-secondary)] hover:text-[var(--color-error)] rounded-lg transition-colors duration-150 hover:bg-[var(--color-surface-elevated)]"
              >
                <LogOut className="w-4 h-4" />
                Sign Out
              </button>
            </div>
          )}
        </div>
      </div>
    </>
  );
}
