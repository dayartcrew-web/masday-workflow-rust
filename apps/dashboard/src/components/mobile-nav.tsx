'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  GitBranch,
  ListTodo,
  Brain,
  MessageSquare,
} from 'lucide-react';

const tabItems = [
  { href: '/', label: 'Home', icon: LayoutDashboard },
  { href: '/workflows', label: 'Flows', icon: GitBranch },
  { href: '/tasks', label: 'Tasks', icon: ListTodo },
  { href: '/memory', label: 'Memory', icon: Brain },
  { href: '/chat', label: 'Chat', icon: MessageSquare },
];

export function MobileNav() {
  const pathname = usePathname();

  return (
    <nav className="md:hidden fixed bottom-0 left-0 right-0 z-50 mobile-nav-root">
      <div className="flex items-center justify-around h-14 max-w-lg mx-auto px-1">
        {tabItems.map((item) => {
          const isActive =
            pathname === item.href ||
            (item.href !== '/' && pathname.startsWith(item.href));
          return (
            <Link
              key={item.href}
              href={item.href}
              className={`mobile-tab ${isActive ? 'mobile-tab--active' : ''}`}
            >
              <div className="mobile-tab-icon-wrap">
                <item.icon className="w-[18px] h-[18px]" strokeWidth={isActive ? 2.2 : 1.8} />
                {isActive && <span className="mobile-tab-dot" />}
              </div>
              <span className="mobile-tab-label">{item.label}</span>
            </Link>
          );
        })}
      </div>
      {/* Safe area spacer for notched devices */}
      <div className="h-[env(safe-area-inset-bottom)]" />
    </nav>
  );
}
