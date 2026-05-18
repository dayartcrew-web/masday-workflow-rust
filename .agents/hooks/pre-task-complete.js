/**
 * pre-task-complete — Enforces review gate before workflow.completeTask.
 * Blocks completion unless review.submit with APPROVED verdict was called first.
 */

const COMPLETE_TOOLS = new Set([
  'mcp__masday__workflow_completeTask',
  'mcp__workflow-orchestrator__workflow_complete_task',
  'workflow.completeTask',
]);

export default function preTaskComplete(context) {
  const toolName = context.tool_name || '';

  if (!COMPLETE_TOOLS.has(toolName)) return;

  return {
    systemMessage:
      '[pre-task-complete] workflow.completeTask called. VERIFY: review.submit with APPROVED verdict was called for this task BEFORE completing. If not, call review.submit first. Skipping review gate is a policy violation.',
  };
}
