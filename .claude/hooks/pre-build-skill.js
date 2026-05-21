/**
 * pre-build-skill — Validates .claude/skills/ and .claude/agents/ files on edit.
 * Checks: tools list against MCP registry, anti-patterns, param gaps.
 * SYNC: Must match apps/agent-runner/src/runtime/mcp.ts — the single source of truth.
 */

const MCP_TOOLS = new Set([
  // workflow (19+4 stubs)
  'workflow_create', 'workflow_execute', 'workflow_getStatus', 'workflow_get',
  'workflow_list', 'workflow_addTask', 'workflow_startTask', 'workflow_completeTask',
  'workflow_saveProgress', 'workflow_listTasks', 'workflow_getCurrentTask',
  'workflow_getPlan', 'workflow_getActive', 'workflow_createPlan',
  'workflow_createParallelBranches', 'workflow_completeParallelBranch',
  'workflow_listParallelBranches', 'workflow_delete', 'workflow_ping',
  'workflow_set_execution_mode', 'workflow_mark_synthesis_ready',
  'workflow_mark_verification_ready', 'workflow_resume_suggestion',
  // review (2 stubs)
  'review_submit', 'review_get_latest',
  // session (3 stubs)
  'session_get_state', 'session_patch_state', 'session_init_context',
  // local (4 stubs)
  'local_init', 'local_sync', 'local_push', 'local_save_artifact',
  // memory (11)
  'memory_store', 'memory_store_research', 'memory_recall_recent',
  'memory_recall_documents', 'memory_recall_document_by_type', 'memory_recall_by_task',
  'memory_update', 'memory_delete', 'memory_delete_by_workflow', 'memory_search',
  'memory_stats',
  // semantic-search (3) — NOT search.*
  'semantic-search_search_hybrid_context_pack', 'semantic-search_search_context_fingerprint', 'semantic-search_code_search',
  // policy (6)
  'policy_check_session_readiness', 'policy_validate_execution',
  'policy_validate_completion', 'policy_validate_parallel_completion',
  'policy_detect_scope_drift', 'policy_require_context_refresh',
  // capability (11)
  'capability_ping', 'capability_list_agents', 'capability_list_skills',
  'capability_list_templates', 'capability_match_agent', 'capability_system_readiness',
  'capability_workflow_audit', 'capability_create_agent', 'capability_create_skill',
  'capability_scaffold_feature', 'capability_scaffold_mcp_server',
  // filesystem (5)
  'filesystem_read', 'filesystem_write', 'filesystem_list', 'filesystem_delete',
  'filesystem_stat',
  // git (3 stubs)
  'git_status', 'git_diff', 'git_commit',
  // npm (2 stubs)
  'npm_install', 'npm_run',
  // docker (3 stubs)
  'docker_build', 'docker_run', 'docker_ps',
  // cicd (3 stubs)
  'cicd_pipeline_status', 'cicd_pipeline_trigger', 'cicd_runs_view',
  // github (3 stubs)
  'github_pr_create', 'github_pr_list', 'github_issue_list',
  // tests (1 stub)
  'tests_run',
]);

const CC_TOOLS = new Set([
  'Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'PowerShell',
  'Agent', 'AskUserQuestion', 'TodoWrite', 'EnterPlanMode', 'ExitPlanMode',
  'Skill', 'NotebookEdit', 'WebSearch', 'LSP', 'Monitor',
  'EnterWorktree', 'ExitWorktree', 'PushNotification',
]);

const ANTI_PATTERNS = [
  { re: /filesystem\.write\s*\(/g, msg: 'Use Claude Code Write tool instead of filesystem_write MCP tool' },
  { re: /status_before/g, msg: 'status_before is NOT a valid param for workflow_save_progress' },
  { re: /status_after/g, msg: 'status_after is NOT a valid param for workflow_save_progress' },
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
