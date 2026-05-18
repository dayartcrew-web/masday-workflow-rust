export default function postToolUse(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;

  const srcExtensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.py', '.rs', '.go'];
  const ext = filePath.lastIndexOf('.') >= 0
    ? filePath.slice(filePath.lastIndexOf('.')).toLowerCase()
    : '';

  if (!srcExtensions.includes(ext)) return;

  return {
    additionalContext: `Source file modified: ${filePath}. Consider running build and tests to verify changes.`,
  };
}
