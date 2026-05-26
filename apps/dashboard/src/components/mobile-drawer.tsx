'use client';

import { useEffect } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  GitBranch,
  ListTodo,
  Brain,
  Network,
  MessageSquare,
  Bot,
  BarChart3,
  Server,
  Shield,
  Settings,
  ClipboardCheck,
  X,
} from 'lucide-react';
import { ConnectionStatus } from '@/components/ui/connection-status';

const mainNavItems = [
  { href: '/', label: 'Dashboard', icon: LayoutDashboard },
  { href: '/workflows', label: 'Workflows', icon: GitBranch },
  { href: '/tasks', label: 'Tasks', icon: ListTodo },
  { href: '/memory', label: 'Memory', icon: Brain },
  { href: '/graph', label: 'Knowledge Graph', icon: Network },
  { href: '/chat', label: 'Chat', icon: MessageSquare },
  { href: '/agent', label: 'Agent Trace', icon: Bot },
];

const systemNavItems = [
  { href: '/analytics', label: 'Analytics', icon: BarChart3 },
  { href: '/providers', label: 'Providers', icon: Server },
  { href: '/policy', label: 'Policy', icon: Shield },
  { href: '/audit', label: 'Audit', icon: ClipboardCheck },
  { href: '/settings', label: 'Settings', icon: Settings },
];

interface MobileDrawerProps {
  open: boolean;
  onClose: () => void;
}

export function MobileDrawer({ open, onClose }: MobileDrawerProps) {
  const pathname = usePathname();

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  useEffect(() => {
    onClose();
  }, [pathname]);

  return (
    <>
      <div
        className={`drawer-overlay ${open ? 'drawer-overlay--visible' : ''}`}
        onClick={onClose}
        aria-hidden="true"
      />
      <aside
        className={`drawer-panel ${open ? 'drawer-panel--open' : ''}`}
        aria-label="Mobile navigation"
      >
        <div className="drawer-header">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 flex items-center justify-center sidebar-logo-icon">
              <span className="text-white font-bold text-sm">M</span>
            </div>
            <div>
              <h1 className="font-bold text-sm text-[var(--color-text)]">Masday Workflow</h1>
              <p className="text-[10px] text-[var(--color-text-secondary)]">v1.0.0</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="drawer-close-btn focus-ring min-h-[44px] min-w-[44px] flex items-center justify-center"
            aria-label="Close navigation menu"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <nav className="flex-1 overflow-y-auto py-3 px-3 scrollbar-thin">
          <ul className="space-y-0.5">
            {mainNavItems.map((item) => (
              <li key={item.href}>
                <Link
                  href={item.href}
                  className={`flex items-center gap-3 px-3 py-3 text-sm font-medium focus-ring sidebar-nav-link ${
                    pathname === item.href ||
                    (item.href !== '/' && pathname.startsWith(item.href))
                      ? 'sidebar-nav-link--active'
                      : ''
                  }`}
                >
                  <item.icon className="w-[18px] h-[18px] flex-shrink-0" />
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>

          <div className="my-3 mx-2 sidebar-divider" />

          <ul className="space-y-0.5">
            {systemNavItems.map((item) => (
              <li key={item.href}>
                <Link
                  href={item.href}
                  className={`flex items-center gap-3 px-3 py-3 text-sm font-medium focus-ring sidebar-nav-link ${
                    pathname === item.href ||
                    (item.href !== '/' && pathname.startsWith(item.href))
                      ? 'sidebar-nav-link--active'
                      : ''
                  }`}
                >
                  <item.icon className="w-[18px] h-[18px] flex-shrink-0" />
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </nav>

        <div className="px-4 py-3 space-y-2 sidebar-footer">
          <ConnectionStatus />
        </div>
      </aside>
    </>
  );
}
