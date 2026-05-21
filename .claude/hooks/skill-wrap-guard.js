/**
 * skill-wrap-guard — Enforces masday pipeline wrap-back after non-masday skills.
 *
 * Fires on Skill tool use. Behavior:
 * - masday-* skills: no reminder (they already follow the pipeline)
 * - Non-masday skills: inject wrap-back reminder to saveProgress → review → completeTask → memory.store
 */

const MASDAY_PREFIX = 'masday-';
const MASDAY_SKILLS = new Set([
  'masday-workflow-new', 'masday-workflow-plan', 'masday-workflow-run',
  'masday-workflow-next', 'masday-workflow-continue', 'masday-workflow-verify',
  'masday-workflow-status', 'masday-workflow-fix', 'masday-workflow-init',
  'masday-workflow-audit', 'masday-workflow-discipline', 'masday-workflow-add-task',
  'masday-research', 'masday-web-research', 'masday-parallel-research',
  'masday-parallel-execution', 'masday-create-agent', 'masday-create-skill',
  'masday-create-mcp-skill', 'masday-create-command', 'masday-memory-search',
  'masday-context-retrieval', 'masday-sequential-thinking', 'masday-code-analyze',
  'masday-skill-builder', 'masday-e2e', 'masday-frontend-library',
  'masday-git-workflow', 'masday-github-flow', 'masday-github-pr',
  'masday-cicd-check', 'masday-cicd-ops', 'masday-deploy-check',
  'masday-docker-ops', 'masday-autopilot',
]);

export default function skillWrapGuard(context) {
  const toolName = context.tool_name || '';
  if (toolName !== 'Skill') return;

  const skillName = context.tool_input?.skill || context.tool_input?.args || '';

  // masday skill — no reminder needed
  if (MASDAY_SKILLS.has(skillName) || skillName.startsWith(MASDAY_PREFIX)) return;

  // Non-masday skill — inject wrap-back reminder
  return {
    systemMessage:
      `[skill-wrap-guard] Non-masday skill "${skillName}" invoked. ` +
      `After this skill completes, you MUST wrap back to masday pipeline:\n` +
      `  1. workflow.saveProgress — log what the skill did\n` +
      `  2. review.submit — quality gate (APPROVED needed before completeTask)\n` +
      `  3. policy.validate_completion — check readiness\n` +
      `  4. workflow.completeTask — close the task\n` +
      `  5. memory.store — persist findings\n` +
      `Skipping this wrap-back is a policy violation.`,
  };
}
