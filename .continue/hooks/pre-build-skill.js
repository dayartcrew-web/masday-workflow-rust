/**
 * pre-build-skill — Validates .claude/skills/ and .claude/agents/ files on edit.
 * Checks: tools list against MCP registry, anti-patterns, param gaps.
 */

const MCP_TOOLS = new Set([
  // workflow
  'workflow.create', 'workflow.execute', 'workflow.getStatus', 'workflow.get',
  'workflow.list', 'workflow.addTask', 'workflow.start_task', 'workflow.complete_task',
  'workflow.save_progress', 'workflow.list_tasks', 'workflow.get_current_task',
  'workflow.get_plan', 'workflow.get_active', 'workflow.delete', 'workflow.ping',
  'workflow.create_parallel_branches', 'workflow.complete_parallel_branch',
  'workflow.list_parallel_branches', 'workflow.set_execution_mode',
  'workflow.mark_synthesis_ready', 'workflow.mark_verification_ready',
  'workflow.resume_suggestion',
  // review
  'review.submit', 'review.get_latest',
  // session
  'session.get_state', 'session.patch_state', 'session.init_context',
  // local
  'local.init', 'local.sync', 'local.push', 'local.save_artifact',
  // memory
  'memory.store', 'memory.store_research', 'memory.recall_recent',
  'memory.recall_documents', 'memory.recall_document_by_type', 'memory.recall_by_task',
  'memory.update', 'memory.delete', 'memory.delete_by_workflow', 'memory.search',
  'memory.stats',
  // search
  'search.hybrid_context_pack', 'search.context_fingerprint', 'search.code_search',
  // policy
  'policy.check_session_readiness', 'policy.validate_execution',
  'policy.validate_completion', 'policy.validate_parallel_completion',
  'policy.detect_scope_drift', 'policy.require_context_refresh',
  // capability
  'capability.ping', 'capability.list_agents', 'capability.list_skills',
  'capability.list_templates', 'capability.match_agent', 'capability.system_readiness',
  'capability.workflow_audit', 'capability.create_agent', 'capability.create_skill',
  'capability.scaffold_feature', 'capability.scaffold_mcp_server',
  // filesystem
  'filesystem.read', 'filesystem.write', 'filesystem.list', 'filesystem.delete',
  'filesystem.stat',
  // git
  'git.status', 'git.diff', 'git.commit',
  // npm
  'npm.install', 'npm.run',
  // docker
  'docker.build', 'docker.run', 'docker.ps',
  // cicd
  'cicd.pipeline_status', 'cicd.pipeline_trigger', 'cicd.runs_view',
  // github
  'github.pr_create', 'github.pr_list', 'github.issue_list',
  // tests
  'tests.run',
]);

const CC_TOOLS = new Set([
  'Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'PowerShell',
  'Agent', 'AskUserQuestion', 'TodoWrite', 'EnterPlanMode', 'ExitPlanMode',
  'Skill', 'NotebookEdit', 'WebSearch', 'LSP', 'Monitor',
  'EnterWorktree', 'ExitWorktree', 'PushNotification',
]);

const ANTI_PATTERNS = [
  { re: /filesystem\.write\s*\(/g, msg: 'Use Claude Code Write tool instead of filesystem.write MCP tool' },
  { re: /status_before/g, msg: 'status_before is NOT a valid param for workflow.save_progress' },
  { re: /status_after/g, msg: 'status_after is NOT a valid param for workflow.save_progress' },
  { re: /\.msd\//g, msg: 'Use .masday/ instead of .msd/' },
  { re: /\bCommonJS\b/g, msg: 'Project uses ESM — avoid referencing CommonJS' },
  { re: /subagent_type:\s*["']msd-/g, msg: 'Agent names use masday-* prefix, not msd-*' },
];

export default async function preBuildSkill(context) {
  const filePath = context.tool_input?.file_path || '';
  if (!filePath) return;

  const isSkill = filePath.includes('.claude/skills/') || filePath.includes('.claude\\skills\\');
  const isAgent = filePath.includes('.claude/agents/') || filePath.includes('.claude\\agents\\');
  if (!isSkill && !isAgent) return;

  const content = context.tool_input?.content || context.tool_input?.new_string || '';
  if (!content) return;

  const warnings = [];

  // 1. Check tools in frontmatter
  const toolsMatch = content.match(/^---\n[\s\S]*?\ntools:\s*\n((?:\s+- .+\n)*)/m);
  if (toolsMatch) {
    const toolLines = toolsMatch[1].trim().split('\n').map(l => l.replace(/^\s+-\s+/, '').trim());
    for (const tool of toolLines) {
      if (!MCP_TOOLS.has(tool) && !CC_TOOLS.has(tool)) {
        warnings.push(`UNKNOWN tool in frontmatter: "${tool}" — not found in MCP registry or Claude Code built-ins`);
      }
    }
  }

  // 2. Check anti-patterns in body
  for (const { re, msg } of ANTI_PATTERNS) {
    re.lastIndex = 0;
    if (re.test(content)) {
      warnings.push(msg);
    }
  }

  // 3. Check subagent_type references in skills
  if (isSkill) {
    const agentRefs = content.matchAll(/subagent_type:\s*["']([^"']+)["']/g);
    for (const m of agentRefs) {
      if (m[1].startsWith('msd-')) {
        warnings.push(`subagent_type "${m[1]}" should be "masday-${m[1].slice(4)}"`);
      }
    }
  }

  if (warnings.length === 0) return;

  return {
    systemMessage: `[pre-build-skill] ${warnings.length} issue(s) found in ${isAgent ? 'agent' : 'skill'} file:\n${warnings.map(w => `- ${w}`).join('\n')}\nFix before proceeding.`,
  };
}
