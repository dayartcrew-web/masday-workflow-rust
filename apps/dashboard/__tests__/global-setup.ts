export default async function globalSetup() {
  const g = globalThis as Record<string, unknown>;
  const modCache: Record<string, unknown> = {};
  g.__mod_cache = modCache;

  const deps = [
    'zustand',
    'zustand/middleware',
    'react',
    'react-dom',
    'react-dom/client',
    'react/jsx-runtime',
    'react/jsx-dev-runtime',
    '@tanstack/react-table',
    '@tanstack/react-virtual',
    'reactflow',
    '@xyflow/react',
    'next/link',
    'next/navigation',
    'lucide-react',
    'sonner',
    'use-sync-external-store/shim/index.js',
    'use-sync-external-store/shim/with-selector',
  ];

  for (const dep of deps) {
    try {
      modCache[dep] = await import(dep);
    } catch {
      try {
        modCache[dep] = await import(`${dep}/index.js`);
      } catch {
        // Module not found, skip
      }
    }
  }
}
