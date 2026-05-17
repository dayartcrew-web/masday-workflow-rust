import type { Metadata } from 'next';
import './globals.css';
import { ThemeProvider } from '@/components/theme-provider';

export const metadata: Metadata = {
  title: 'Masday Workflow Dashboard',
  description: 'Autonomous AI agent platform management dashboard',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)] antialiased">
        <ThemeProvider>
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
