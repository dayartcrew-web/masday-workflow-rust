/**
 * skill-wrap-guard — Reminds to save masday pipeline state after non-masday skills.
 */

const MASDAY_PREFIX = 'masday-';

export default function skillWrapGuard(context) {
  const toolName = context.tool_name || '';
  if (toolName !== 'Skill') return;

  const skillName = context.tool_input?.skill || context.tool_input?.args || '';

  // masday skill — no reminder needed
  if (skillName.startsWith(MASDAY_PREFIX)) return;

  // Non-masday skill — short wrap-back reminder
  return {
    systemMessage:
      `[skill-wrap-guard] After "${skillName}" completes: saveProgress → review_submit → completeTask → memory_store.`,
  };
}
