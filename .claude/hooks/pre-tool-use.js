/**
 * pre-tool-use — Injects masday-first priority reminders on source file edits.
 *
 * Reminds to:
 * 1. Check masday MCP tools first (mcp__masday__*) before manual edits
 * 2. Load workflow context (workflow_getActive + workflow_getCurrentTask)
 */

const SRC_EXTENSIONS = new Set([
  '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.py', '.rs', '.go', '.java', '.rb', '.php',
]);

export default function preToolUse(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;
  if (filePath.includes('.claude/hooks/')) return;
  if (filePath.includes('node_modules')) return;

  const ext = filePath.lastIndexOf('.') >= 0
    ? filePath.slice(filePath.lastIndexOf('.')).toLowerCase()
    : '';

  if (!SRC_EXTENSIONS.has(ext)) return;

  return {
    systemMessage:
      `[pre-tool-use] Editing source file. Masday-first priority:\n` +
      `  1. Could a masday MCP tool handle this? (mcp__masday__*)\n` +
      `  2. Is workflow context loaded? (workflow_getActive, workflow_getCurrentTask)\n` +
      `  3. If in active task: workflow_saveProgress after changes.`,
  };
}
