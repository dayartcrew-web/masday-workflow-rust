/**
 * post-tool-use — After source file modification, enforces masday wrap-back.
 *
 * Reminds to:
 * 1. Run build/tests to verify changes
 * 2. Save progress to masday pipeline (saveProgress, memory_store)
 * 3. Submit review if in active task
 */

const SRC_EXTENSIONS = new Set([
  '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.py', '.rs', '.go', '.java', '.rb', '.php',
]);

export default function postToolUse(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;
  if (filePath.includes('.claude/hooks/')) return;
  if (filePath.includes('node_modules')) return;

  const ext = filePath.lastIndexOf('.') >= 0
    ? filePath.slice(filePath.lastIndexOf('.')).toLowerCase()
    : '';

  if (!SRC_EXTENSIONS.has(ext)) return;

  return {
    additionalContext:
      `[post-tool-use] Source file modified: ${filePath}\n` +
      `Masday wrap-back:\n` +
      `  1. Run build/tests to verify changes\n` +
      `  2. workflow_saveProgress — persist what changed\n` +
      `  3. memory_store — save context for future sessions\n` +
      `  4. review_submit — quality gate before completeTask`,
  };
}
