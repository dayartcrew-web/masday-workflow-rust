/**
 * workflow-lock — Ensures Claude dispatches workflow orchestrator MCP tools
 * before making changes. Fires on Edit/Write/Bash/Agent to inject a reminder.
 */

const EDIT_TOOLS = new Set(['Edit', 'Write', 'MultiEdit']);
const SRC_EXTS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs']);

export default function workflowLock(context) {
  const toolName = context.tool_name || '';
  const filePath = context.tool_input?.file_path || '';
  const command = context.tool_input?.command || '';
  const subagentType = context.tool_input?.subagent_type || '';

  if (filePath.includes('.claude/hooks/')) return;
  if (filePath.includes('node_modules')) return;

  // Edit/Write source files — remind to load workflow context
  if (EDIT_TOOLS.has(toolName)) {
    const ext = filePath.lastIndexOf('.') >= 0 ? filePath.slice(filePath.lastIndexOf('.')) : '';
    if (!SRC_EXTS.has(ext)) return;

    return {
      systemMessage: '[workflow-lock] Editing source file. Ensure workflow context is loaded: call workflow.getActive and workflow.getCurrentTask before making changes.',
    };
  }

  // Agent dispatch — remind to validate execution
  if (toolName === 'Agent' && subagentType?.startsWith('masday-')) {
    return {
      systemMessage: `[workflow-lock] Dispatching agent "${subagentType}". Ensure policy.validate_execution was called for the current task before dispatch.`,
    };
  }

  // Bash build/test — remind to save progress
  if (toolName === 'Bash' && /\b(pnpm\s+(build|test|tsc|lint|check))\b/.test(command)) {
    return {
      systemMessage: '[workflow-lock] Running build/test. Save progress with workflow.saveProgress after results are available.',
    };
  }
}
