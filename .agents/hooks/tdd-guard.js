import { access } from 'node:fs/promises';
import { join, dirname, basename, extname } from 'node:path';

const SRC_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx']);

function guessTestPath(srcPath) {
  const dir = dirname(srcPath);
  const base = basename(srcPath, extname(srcPath));
  const ext = extname(srcPath);

  return [
    join(dir, `${base}.test${ext}`),
    join(dir, `${base}.spec${ext}`),
    join(dir, '__tests__', `${base}.test${ext}`),
    join(dir, 'tests', `${base}.test${ext}`),
    join(dir, 'test', `${base}.test${ext}`),
  ];
}

export default async function tddGuard(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;

  const ext = extname(filePath).toLowerCase();
  if (!SRC_EXTENSIONS.has(ext)) return;
  if (filePath.includes('.test.') || filePath.includes('.spec.')) return;
  if (filePath.includes('node_modules')) return;

  const testPaths = guessTestPath(filePath);
  for (const testPath of testPaths) {
    try {
      await access(testPath);
      return;
    } catch {
      // continue checking
    }
  }

  return {
    systemMessage: `No test file found for ${basename(filePath)}. Consider creating a test file first (TDD workflow).`,
  };
}
