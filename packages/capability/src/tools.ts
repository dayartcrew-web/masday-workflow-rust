/**
 * Capability MCP Tool Business Logic
 *
 * Contains the core logic for 10 capability MCP tools.
 * These functions return plain objects; Phase 7 wraps them in MCP protocol.
 */

import type { StorageBackend } from '@mcp-rebuild/store';
import { WorkflowAuditor } from '@mcp-rebuild/policy';
import type { AuditResult } from '@mcp-rebuild/policy';
import {
  loadRegistry,
  scanExistingAgents,
  type AgentEntry,
  type SkillEntry,
} from './registry.js';
import {
  listTemplates,
  scaffoldAgent,
  scaffoldSkill,
  scaffoldFeature,
  scaffoldMcpServer,
  type ScaffoldFeatureInput,
  type ScaffoldMcpServerInput,
  type Template,
  type ScaffoldResult,
  type McpServerScaffoldResult,
} from './scaffold.js';
import { checkSystemReadiness, getSystemStats, type SystemReadinessResult } from './health.js';

// --- Tool 1: capability.create_agent ---

export interface CreateAgentInput {
  projectRoot: string;
  name: string;
  role: string;
  description: string;
  instructions: string;
}

export interface CreateAgentResult {
  path: string;
  registered: boolean;
}

export function createAgentTool(
  _storage: StorageBackend,
  input: CreateAgentInput,
): CreateAgentResult {
  const result = scaffoldAgent(input.projectRoot, {
    name: input.name,
    role: input.role,
    description: input.description,
    instructions: input.instructions,
  });

  return {
    path: result.path,
    registered: true,
  };
}

// --- Tool 2: capability.create_skill ---

export interface CreateSkillInput {
  projectRoot: string;
  name: string;
  description: string;
  trigger: string;
  steps: string[];
}

export interface CreateSkillResult {
  path: string;
  registered: boolean;
}

export function createSkillTool(
  _storage: StorageBackend,
  input: CreateSkillInput,
): CreateSkillResult {
  const result = scaffoldSkill(input.projectRoot, {
    name: input.name,
    description: input.description,
    trigger: input.trigger,
    steps: input.steps,
  });

  return {
    path: result.path,
    registered: true,
  };
}

// --- Tool 3: capability.list_agents ---

export interface ListAgentsInput {
  projectRoot: string;
}

export interface ListAgentsResult {
  agents: AgentEntry[];
  count: number;
}

export function listAgentsTool(
  _storage: StorageBackend,
  input: ListAgentsInput,
): ListAgentsResult {
  const registry = loadRegistry(input.projectRoot);
  return {
    agents: registry.agents,
    count: registry.agents.length,
  };
}

// --- Tool 4: capability.list_skills ---

export interface ListSkillsInput {
  projectRoot: string;
}

export interface ListSkillsResult {
  skills: SkillEntry[];
  count: number;
}

export function listSkillsTool(
  _storage: StorageBackend,
  input: ListSkillsInput,
): ListSkillsResult {
  const registry = loadRegistry(input.projectRoot);
  return {
    skills: registry.skills,
    count: registry.skills.length,
  };
}

// --- Tool 5: capability.list_templates ---

export interface ListTemplatesResult {
  templates: Template[];
  count: number;
}

export function listTemplatesTool(
  _storage: StorageBackend,
): ListTemplatesResult {
  const templates = listTemplates();
  return {
    templates,
    count: templates.length,
  };
}

// --- Tool 6: capability.match_agent ---

export interface MatchAgentInput {
  projectRoot: string;
  taskDescription: string;
  requiredRole?: string;
}

export interface MatchAgentResult {
  matched: boolean;
  agent: AgentEntry | null;
  score: number;
  reason: string;
}

/**
 * Match a task description to the best agent.
 *
 * Uses simple keyword overlap scoring against agent role, description,
 * and skills.
 */
export function matchAgentTool(
  _storage: StorageBackend,
  input: MatchAgentInput,
): MatchAgentResult {
  const agents = scanExistingAgents(input.projectRoot);

  if (agents.length === 0) {
    return {
      matched: false,
      agent: null,
      score: 0,
      reason: 'No agents registered in the registry',
    };
  }

  // Filter by role if specified
  const candidates = input.requiredRole
    ? agents.filter((a) => a.role === input.requiredRole)
    : agents;

  if (candidates.length === 0) {
    return {
      matched: false,
      agent: null,
      score: 0,
      reason: `No agents found with role: ${input.requiredRole}`,
    };
  }

  // Score each agent by keyword overlap
  const taskTokens = tokenize(input.taskDescription);

  let bestAgent: AgentEntry | null = null;
  let bestScore = 0;

  for (const agent of candidates) {
    const agentText = [agent.name, agent.role, agent.description, ...agent.skills].join(' ');
    const agentTokens = tokenize(agentText);

    const score = overlapScore(taskTokens, agentTokens);

    if (score > bestScore) {
      bestScore = score;
      bestAgent = agent;
    }
  }

  // Require minimum score of 0.05 for a match
  if (bestScore < 0.05 || !bestAgent) {
    return {
      matched: false,
      agent: bestAgent,
      score: bestScore,
      reason: `Best agent score (${bestScore.toFixed(2)}) is below minimum threshold (0.05)`,
    };
  }

  return {
    matched: true,
    agent: bestAgent,
    score: bestScore,
    reason: `Matched agent "${bestAgent.name}" with score ${bestScore.toFixed(2)}`,
  };
}

function tokenize(text: string): Set<string> {
  return new Set(
    text
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, ' ')
      .split(/\s+/)
      .filter((t) => t.length > 2),
  );
}

function overlapScore(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 && b.size === 0) return 1;
  if (a.size === 0 || b.size === 0) return 0;

  let matches = 0;
  for (const token of a) {
    if (b.has(token)) matches++;
  }
  return matches / a.size;
}

// --- Tool 7: capability.scaffold_feature ---

export function scaffoldFeatureTool(
  _storage: StorageBackend,
  input: ScaffoldFeatureInput & { projectRoot: string },
): ScaffoldResult {
  return scaffoldFeature(input.projectRoot, input);
}

// --- Tool 8: capability.scaffold_mcp_server ---

export function scaffoldMcpServerTool(
  _storage: StorageBackend,
  input: ScaffoldMcpServerInput & { projectRoot: string },
): McpServerScaffoldResult {
  return scaffoldMcpServer(input.projectRoot, input);
}

// --- Tool 9: capability.system_readiness ---

export function systemReadinessTool(
  storage: StorageBackend,
  projectRoot?: string,
): SystemReadinessResult & { stats: ReturnType<typeof getSystemStats> } {
  const readiness = checkSystemReadiness(storage, projectRoot);
  const stats = getSystemStats(storage);

  return {
    ...readiness,
    stats,
  };
}

// --- Tool 10: capability.workflow_audit ---

export interface WorkflowAuditInput {
  workflowId?: string;
}

export function workflowAuditTool(
  storage: StorageBackend,
  input: WorkflowAuditInput,
): Promise<AuditResult | AuditResult[]> {
  const auditor = new WorkflowAuditor(storage);

  if (input.workflowId) {
    return auditor.audit(input.workflowId);
  }

  return auditor.auditAll();
}
