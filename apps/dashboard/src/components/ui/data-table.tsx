'use client';

import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { ChevronUp, ChevronDown } from 'lucide-react';
import clsx from 'clsx';

interface Column<T extends object> {
  key: Extract<keyof T, string> | string;
  label: string;
  sortable?: boolean;
  render?: (item: T) => ReactNode;
}

interface DataTableProps<T extends object> {
  columns: Column<T>[];
  data: T[];
  keyField?: Extract<keyof T, string> | string;
  pageSize?: number;
  emptyMessage?: string;
  onRowClick?: (item: T) => void;
}

export function DataTable<T extends object>({
  columns,
  data,
  keyField = 'id',
  pageSize = 20,
  emptyMessage = 'No data available',
  onRowClick,
}: DataTableProps<T>) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  const [page, setPage] = useState(0);

  const handleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir(sortDir === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  const sorted = [...data].sort((a, b) => {
    if (!sortKey) return 0;
    const av = a[sortKey as keyof T];
    const bv = b[sortKey as keyof T];
    if (av == null || bv == null) return 0;
    const cmp = String(av).localeCompare(String(bv));
    return sortDir === 'asc' ? cmp : -cmp;
  });

  const totalPages = Math.ceil(sorted.length / pageSize);
  const paged = sorted.slice(page * pageSize, (page + 1) * pageSize);

  useEffect(() => {
    const lastPage = Math.max(totalPages - 1, 0);
    if (page > lastPage) {
      // When the dataset shrinks, keep pagination in a valid range.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setPage(lastPage);
    }
  }, [page, totalPages]);

  return (
    <div className="overflow-x-auto glass-surface scrollbar-thin">
      <table className="w-full text-sm min-w-[480px]">
        <thead>
          <tr style={{ background: 'var(--color-surface-elevated)', borderBottom: '1px solid var(--color-border-subtle)' }}>
            {columns.map((col) => (
              <th
                key={col.key}
                className="text-left py-2.5 px-3 md:px-4 text-[var(--color-text-secondary)] font-medium whitespace-nowrap uppercase tracking-wider"
                style={{ fontSize: 'var(--font-size-small)' }}
                onClick={col.sortable ? () => handleSort(col.key) : undefined}
              >
                <span className={clsx(col.sortable && 'cursor-pointer select-none inline-flex items-center gap-1 transition-base hover:text-[var(--color-primary)]')}>
                  {col.label}
                  {col.sortable && sortKey === col.key && (
                    sortDir === 'asc' ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />
                  )}
                </span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {paged.length === 0 ? (
            <tr>
              <td colSpan={columns.length} className="py-8 text-center text-[var(--color-text-secondary)] text-body">
                {emptyMessage}
              </td>
            </tr>
          ) : (
            paged.map((item, idx) => (
              <tr
                key={String(item[keyField as keyof T] ?? idx)}
                className={clsx(
                  'transition-base',
                  onRowClick && 'cursor-pointer',
                )}
                style={{
                  background: 'var(--color-surface)',
                  borderBottom: '1px solid var(--color-border-subtle)',
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'var(--color-surface-elevated)';
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'var(--color-surface)';
                }}
                onClick={onRowClick ? () => onRowClick(item) : undefined}
              >
                {columns.map((col) => (
                  <td key={String(col.key)} className="py-2 px-3 md:px-4 text-[var(--color-text)]">
                    {col.render ? col.render(item) : String(item[col.key as keyof T] ?? '')}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
      {totalPages > 1 && (
        <div className="flex items-center justify-between px-3 md:px-4 py-3 text-xs text-[var(--color-text-secondary)]" style={{ borderTop: '1px solid var(--color-border-subtle)' }}>
          <span>Page {page + 1} of {totalPages}</span>
          <div className="flex gap-2">
            <button
              onClick={() => setPage(Math.max(0, page - 1))}
              disabled={page === 0}
              className="px-3 py-1.5 rounded-md transition-base disabled:opacity-40"
              style={{ border: '1px solid var(--color-border-subtle)', background: 'var(--color-surface)' }}
            >
              Prev
            </button>
            <button
              onClick={() => setPage(Math.min(totalPages - 1, page + 1))}
              disabled={page >= totalPages - 1}
              className="px-3 py-1.5 rounded-md transition-base disabled:opacity-40"
              style={{ border: '1px solid var(--color-border-subtle)', background: 'var(--color-surface)' }}
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
