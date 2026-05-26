'use client';

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

function NavLink({
  href,
  label,
  icon: Icon,
  isActive,
}: {
  href: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  isActive: boolean;
}) {
  return (
    <li>
      <Link
        href={href}
        className={`flex items-center gap-3 px-3 py-2.5 text-sm font-medium focus-ring sidebar-nav-link ${
          isActive ? 'sidebar-nav-link--active' : ''
        }`}
      >
        <Icon className="w-[18px] h-[18px] flex-shrink-0" />
        <span className="flex-1">{label}</span>
      </Link>
    </li>
  );
}

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="hidden md:flex w-[260px] h-screen flex-col fixed left-0 top-0 z-40 sidebar-root">
      {/* Logo */}
      <div className="px-4 py-5 sidebar-logo-border">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 flex items-center justify-center sidebar-logo-icon">
            <span className="text-white font-bold text-sm">M</span>
          </div>
          <div>
            <h1 className="font-bold text-sm text-[var(--color-text)]">
              Masday Workflow
            </h1>
            <p className="text-[10px] text-[var(--color-text-secondary)]">
              v1.0.0
            </p>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto py-3 px-3 scrollbar-thin">
        {/* Main Navigation */}
        <ul className="space-y-0.5">
          {mainNavItems.map((item) => (
            <NavLink
              key={item.href}
              href={item.href}
              label={item.label}
              icon={item.icon}
              isActive={
                pathname === item.href ||
                (item.href !== '/' && pathname.startsWith(item.href))
              }
            />
          ))}
        </ul>

        {/* Divider */}
        <div className="my-3 mx-2 sidebar-divider" />

        {/* System Navigation */}
        <ul className="space-y-0.5">
          {systemNavItems.map((item) => (
            <NavLink
              key={item.href}
              href={item.href}
              label={item.label}
              icon={item.icon}
              isActive={
                pathname === item.href ||
                (item.href !== '/' && pathname.startsWith(item.href))
              }
            />
          ))}
        </ul>
      </nav>

      {/* Footer */}
      <div className="px-4 py-3 space-y-2 sidebar-footer">
        <ConnectionStatus />
      </div>
    </aside>
  );
}
