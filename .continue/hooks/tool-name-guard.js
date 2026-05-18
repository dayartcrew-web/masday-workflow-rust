import { readFile } from 'node:fs/promises';

const MCP_TOOL_FILE = 'apps/agent-runner/src/runtime/mcp.ts';

const CAMEL_CASE_TOOL_RE = /reg\("(?:workflow|memory|search|policy|capability|filesystem|git|npm|docker|cicd|github|tests)\.([a-z][a-zA-Z0-9]*[A-Z][a-zA-Z0-9]*)"/g;

export default async function toolNameGuard() {
  let content;
  try {
    content = await readFile(MCP_TOOL_FILE, 'utf-8');
  } catch {
    return;
  }

  const violations = [];
  let match;
  while ((match = CAMEL_CASE_TOOL_RE.exec(content)) !== null) {
    violations.push(match[1]);
  }

  if (violations.length > 0) {
    return {
      systemMessage: `MCP tool names should use snake_case, not camelCase. Found: ${violations.join(', ')}. Fix: use underscores (e.g. start_task not startTask).`,
    };
  }
}
