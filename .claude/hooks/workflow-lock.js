/**
 * workflow-lock — Only fires when an active workflow exists.
 * Reminds to load workflow context before editing source files
 * or dispatching masday agents.
 */

import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

const EDIT_TOOLS = new Set(['Edit', 'Write', 'MultiEdit']);
const SRC_EXTS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.rs', '.py', '.go']);

async function hasActiveWorkflow() {
  try {
    const statePath = join(process.cwd(), '.masday', 'state.json');
    const raw = await readFile(statePath, 'utf8');
    const state = JSON.parse(raw);
    return !!state?.activeWorkflow;
  } catch {
    return false;
  }
}

export default async function workflowLock(context) {
  // Only fire when a workflow is active
  if (!await hasActiveWorkflow()) return;

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
      systemMessage: '[workflow-lock] Editing source file with active workflow. Ensure workflow_getActive + getCurrentTask called first.',
    };
  }

  // Agent dispatch — remind to validate execution
  if (toolName === 'Agent' && subagentType?.startsWith('masday-')) {
    return {
      systemMessage: `[workflow-lock] Dispatching "${subagentType}". Call policy_validate_execution for current task first.`,
    };
  }
}
