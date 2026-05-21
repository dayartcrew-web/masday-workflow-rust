/**
 * pre-task-complete — Enforces review gate before workflow_completeTask.
 * Blocks completion unless review_submit with APPROVED verdict was called first.
 */

const COMPLETE_TOOLS = new Set([
  'mcp__masday__workflow_completeTask',
  'mcp__workflow-orchestrator__workflow_complete_task',
  'workflow_completeTask',
]);

export default function preTaskComplete(context) {
  const toolName = context.tool_name || '';

  if (!COMPLETE_TOOLS.has(toolName)) return;

  return {
    systemMessage:
      '[pre-task-complete] workflow_completeTask called. VERIFY: review_submit with APPROVED verdict was called for this task BEFORE completing. If not, call review_submit first. Skipping review gate is a policy violation.',
  };
}
