'use client';

import { useEffect } from 'react';
import { useSettingsStore } from '@/stores/settings-store';

const THEME_KEY = 'masday-theme';
const DEFAULT_THEME = 'dark' as const;

type Theme = 'light' | 'dark';

function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  const isDark = theme === 'dark';

  root.classList.toggle('dark', isDark);
  root.style.colorScheme = isDark ? 'dark' : 'light';
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const theme = useSettingsStore((s) => s.theme);

  useEffect(() => {
    const saved = localStorage.getItem(THEME_KEY) as Theme | null;
    const initial: Theme = saved === 'light' || saved === 'dark' ? saved : DEFAULT_THEME;
    applyTheme(initial);
    useSettingsStore.getState().setTheme(initial);
  }, []);

  useEffect(() => {
    applyTheme(theme);
    localStorage.setItem(THEME_KEY, theme);
  }, [theme]);

  return <>{children}</>;
}
