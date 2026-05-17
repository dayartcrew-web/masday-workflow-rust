'use client';

import type { ReactNode } from 'react';
import clsx from 'clsx';

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: ReactNode;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
  className?: string;
}

export function MetricCard({ title, value, subtitle, icon, trend, trendValue, className }: MetricCardProps) {
  return (
    <div className={clsx(
      'relative rounded-[var(--radius-lg,16px)] p-6 flex flex-col gap-3',
      'bg-[var(--color-surface)] border border-[var(--color-border-subtle)]',
      'shadow-[var(--shadow-card-depth,0_8px_40px_rgba(0,0,0,0.45))]',
      'backdrop-blur-[12px]',
      'transition-[box-shadow,border-color,background-color] duration-250 ease-in-out',
      'hover:bg-[var(--color-surface-elevated)] hover:border-[rgba(99,102,241,0.3)]',
      'hover:shadow-[var(--shadow-neon-glow)]',
      'group',
      className,
    )}>
      <div className="flex items-center justify-between">
        <span className="text-[14px] text-[var(--color-text-secondary)] tracking-wide uppercase font-medium">{title}</span>
        {icon && (
          <span className="flex items-center justify-center w-9 h-9 rounded-[var(--radius-md,12px)] bg-[var(--color-surface-elevated)] border border-[var(--color-border-subtle)] transition-all duration-250 ease-in-out group-hover:shadow-[0_0_12px_rgba(99,102,241,0.3)]">
            {icon}
          </span>
        )}
      </div>
      <div className="flex items-end gap-2">
        <span className="text-[32px] font-bold leading-none text-[var(--color-text)] tracking-tight">{value}</span>
        {trend && trendValue && (
          <span className={clsx(
            'text-xs font-semibold mb-1',
            trend === 'up' && 'text-[var(--color-neon-green)]',
            trend === 'down' && 'text-[var(--color-error)]',
            trend === 'neutral' && 'text-[var(--color-text-secondary)]',
          )}>
            {trend === 'up' ? '+' : trend === 'down' ? '-' : ''}{trendValue}
          </span>
        )}
      </div>
      {subtitle && (
        <span className="text-xs text-[var(--color-text-secondary)]">{subtitle}</span>
      )}
    </div>
  );
}
