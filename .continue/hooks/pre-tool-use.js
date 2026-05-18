const SRC_EXTENSIONS = new Set([
  '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.py', '.rs', '.go', '.java', '.rb', '.php',
]);

export default function preToolUse(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;

  const ext = filePath.lastIndexOf('.') >= 0
    ? filePath.slice(filePath.lastIndexOf('.')).toLowerCase()
    : '';

  if (!SRC_EXTENSIONS.has(ext)) return;

  return {
    systemMessage: `Editing ${ext} source file. Ensure workflow context is loaded before making changes.`,
  };
}
