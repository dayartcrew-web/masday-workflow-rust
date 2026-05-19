import { readFile } from 'node:fs/promises';

const MCP_TOOL_FILE = 'apps/agent-runner/src/runtime/mcp.ts';

// SYNC: Must match apps/agent-runner/src/runtime/mcp.ts — the single source of truth.
// Naming convention: camelCase for core methods (startTask not start_task).
// Some stubs use snake_case (set_execution_mode, pipeline_status, etc.) — that is valid.
// This guard detects UNKNOWN namespace registrations in mcp.ts.
const KNOWN_NAMESPACES = new Set([
  'workflow', 'memory', 'semantic-search', 'policy', 'capability',
  'filesystem', 'review', 'session', 'local',
  'git', 'npm', 'docker', 'cicd', 'github', 'tests',
  'reminder', 'projectRules',
]);

export default async function toolNameGuard() {
  let content;
  try {
    content = await readFile(MCP_TOOL_FILE, 'utf-8');
  } catch {
    return;
  }

  const TOOL_RE = /registerTool\("([^.]+)\.([^"]+)"/g;
  const unknown = [];
  let match;
  while ((match = TOOL_RE.exec(content)) !== null) {
    const ns = match[1];
    if (!KNOWN_NAMESPACES.has(ns)) {
      unknown.push(`${ns}.${match[2]}`);
    }
  }

  if (unknown.length > 0) {
    return {
      systemMessage: `[tool-name-guard] Unknown MCP namespace(s): ${unknown.join(', ')}. Must be one of: ${[...KNOWN_NAMESPACES].join(', ')}.`,
    };
  }
}
