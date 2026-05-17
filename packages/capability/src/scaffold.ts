/**
 * Capability Scaffolding
 *
 * Generates agent, skill, command, MCP server, and feature files
 * from templates. All file operations are performed against a project root.
 */

import fs from 'fs';
import path from 'path';
import { createLogger } from '@mcp-rebuild/core';
import {
  loadRegistry,
  registerAgent,
  registerSkill,
  registerCommand,
  type AgentEntry,
  type SkillEntry,
  type CommandEntry,
} from './registry.js';

const logger = createLogger('Scaffolder');

// --- Types ---

export interface Template {
  name: string;
  description: string;
  type: 'agent' | 'skill' | 'command' | 'mcp-server' | 'feature';
}

export interface ScaffoldResult {
  files: Array<{ path: string; content: string }>;
  registry: {
    agents: number;
    skills: number;
    commands: number;
  };
}

export interface McpServerScaffoldResult {
  files: Array<{ path: string; content: string }>;
  packageName: string;
}

export interface ScaffoldFeatureInput {
  name: string;
  description: string;
  agentRole: string;
  skillTrigger: string;
  mcpToolName: string;
  mcpToolDescription: string;
}

export interface ScaffoldMcpServerInput {
  name: string;
  description: string;
  tools: Array<{ name: string; description: string }>;
}

export interface ScaffoldAgentInput {
  name: string;
  role: string;
  description: string;
  instructions: string;
}

export interface ScaffoldSkillInput {
  name: string;
  description: string;
  trigger: string;
  steps: string[];
}

// --- Templates ---

const TEMPLATES: Template[] = [
  {
    name: 'agent',
    description: 'Agent definition with role, description, and skill references',
    type: 'agent',
  },
  {
    name: 'skill',
    description: 'Skill definition with trigger and execution steps',
    type: 'skill',
  },
  {
    name: 'command',
    description: 'Command definition for user-facing actions',
    type: 'command',
  },
  {
    name: 'mcp-server',
    description: 'MCP server package with package.json, tsconfig, and stubs',
    type: 'mcp-server',
  },
  {
    name: 'feature',
    description: 'All-in-one: agent + skill + command + MCP tool',
    type: 'feature',
  },
];

/**
 * List all available templates.
 */
export function listTemplates(): Template[] {
  return [...TEMPLATES];
}

// --- Agent Scaffolding ---

/**
 * Generate an agent definition file.
 */
export function scaffoldAgent(
  projectRoot: string,
  input: ScaffoldAgentInput,
): { path: string } {
  const agentsDir = path.join(projectRoot, '.claude', 'agents');
  if (!fs.existsSync(agentsDir)) {
    fs.mkdirSync(agentsDir, { recursive: true });
  }

  const filename = `${input.name.toLowerCase().replace(/\s+/g, '-')}.md`;
  const filePath = path.join(agentsDir, filename);

  const content = [
    '---',
    `name: ${input.name}`,
    `role: ${input.role}`,
    `description: ${input.description}`,
    'skills: []',
    'status: active',
    '---',
    '',
    `# ${input.name}`,
    '',
    `Role: ${input.role}`,
    '',
    input.instructions,
    '',
  ].join('\n');

  fs.writeFileSync(filePath, content, 'utf-8');

  // Register in registry
  const entry: AgentEntry = {
    name: input.name,
    role: input.role,
    description: input.description,
    skills: [],
    file: filename,
    status: 'active',
  };
  registerAgent(projectRoot, entry);

  logger.info({ name: input.name, filePath }, 'Scaffolded agent');
  return { path: filePath };
}

// --- Skill Scaffolding ---

/**
 * Generate a skill definition file.
 */
export function scaffoldSkill(
  projectRoot: string,
  input: ScaffoldSkillInput,
): { path: string } {
  const skillsDir = path.join(projectRoot, '.claude', 'skills');
  if (!fs.existsSync(skillsDir)) {
    fs.mkdirSync(skillsDir, { recursive: true });
  }

  const filename = `${input.name.toLowerCase().replace(/\s+/g, '-')}.md`;
  const filePath = path.join(skillsDir, filename);

  const stepsContent = input.steps.map((step, i) => `${i + 1}. ${step}`).join('\n');

  const content = [
    '---',
    `name: ${input.name}`,
    `trigger: ${input.trigger}`,
    `description: ${input.description}`,
    '---',
    '',
    `# ${input.name}`,
    '',
    `Trigger: ${input.trigger}`,
    '',
    '## Steps',
    '',
    stepsContent || '1. (No steps defined)',
    '',
  ].join('\n');

  fs.writeFileSync(filePath, content, 'utf-8');

  // Register in registry
  const entry: SkillEntry = {
    name: input.name,
    trigger: input.trigger,
    description: input.description,
    file: filename,
  };
  registerSkill(projectRoot, entry);

  logger.info({ name: input.name, filePath }, 'Scaffolded skill');
  return { path: filePath };
}

// --- Command Scaffolding ---

function scaffoldCommand(
  projectRoot: string,
  name: string,
  description: string,
): { path: string } {
  const commandsDir = path.join(projectRoot, '.claude', 'commands');
  if (!fs.existsSync(commandsDir)) {
    fs.mkdirSync(commandsDir, { recursive: true });
  }

  const filename = `${name.toLowerCase().replace(/\s+/g, '-')}.md`;
  const filePath = path.join(commandsDir, filename);

  const content = [
    '---',
    `name: ${name}`,
    `description: ${description}`,
    '---',
    '',
    `# ${name}`,
    '',
    description,
    '',
  ].join('\n');

  fs.writeFileSync(filePath, content, 'utf-8');

  const entry: CommandEntry = {
    name,
    description,
    file: filename,
  };
  registerCommand(projectRoot, entry);

  logger.info({ name, filePath }, 'Scaffolded command');
  return { path: filePath };
}

// --- Feature Scaffolding (all-in-one) ---

/**
 * Scaffold a complete feature: agent + skill + command + MCP tool stub.
 */
export function scaffoldFeature(
  projectRoot: string,
  input: ScaffoldFeatureInput,
): ScaffoldResult {
  const files: ScaffoldResult['files'] = [];

  // Agent
  const agentResult = scaffoldAgent(projectRoot, {
    name: `${input.name} Agent`,
    role: input.agentRole,
    description: input.description,
    instructions: `Agent for ${input.name}. Uses ${input.skillTrigger} skill.`,
  });

  const agentContent = fs.readFileSync(agentResult.path, 'utf-8');
  files.push({ path: agentResult.path, content: agentContent });

  // Skill
  const skillResult = scaffoldSkill(projectRoot, {
    name: `${input.name} Skill`,
    description: input.description,
    trigger: input.skillTrigger,
    steps: [
      `Execute ${input.name} workflow`,
      'Process results',
      'Report completion',
    ],
  });

  const skillContent = fs.readFileSync(skillResult.path, 'utf-8');
  files.push({ path: skillResult.path, content: skillContent });

  // Command
  const commandResult = scaffoldCommand(
    projectRoot,
    `${input.name} Command`,
    input.description,
  );

  const commandContent = fs.readFileSync(commandResult.path, 'utf-8');
  files.push({ path: commandResult.path, content: commandContent });

  // MCP Tool stub
  const toolsDir = path.join(projectRoot, '.claude', 'tools');
  if (!fs.existsSync(toolsDir)) {
    fs.mkdirSync(toolsDir, { recursive: true });
  }

  const toolFilename = `${input.mcpToolName.toLowerCase().replace(/\s+/g, '-')}.ts`;
  const toolPath = path.join(toolsDir, toolFilename);
  const toolContent = [
    '/**',
    ` * ${input.mcpToolName}`,
    ` * ${input.mcpToolDescription}`,
    ' */',
    '',
    `export const ${input.mcpToolName.replace(/\s+/g, '')}Tool = {`,
    `  name: '${input.mcpToolName}',`,
    `  description: '${input.mcpToolDescription}',`,
    '  inputSchema: {',
    '    type: "object" as const,',
    '    properties: {}',
    '  },',
    '  execute: async (input: unknown) => {',
    '    // TODO: Implement tool logic',
    '    return { success: true };',
    '  },',
    '};',
    '',
  ].join('\n');

  fs.writeFileSync(toolPath, toolContent, 'utf-8');
  files.push({ path: toolPath, content: toolContent });

  // Get updated registry counts
  const registry = loadRegistry(projectRoot);

  logger.info({ name: input.name, fileCount: files.length }, 'Scaffolded feature');

  return {
    files,
    registry: {
      agents: registry.agents.length,
      skills: registry.skills.length,
      commands: registry.commands.length,
    },
  };
}

// --- MCP Server Scaffolding ---

/**
 * Scaffold a new MCP server package with package.json, tsconfig, and tool stubs.
 */
export function scaffoldMcpServer(
  projectRoot: string,
  input: ScaffoldMcpServerInput,
): McpServerScaffoldResult {
  const packageName = input.name.toLowerCase().replace(/\s+/g, '-');
  const serverDir = path.join(projectRoot, 'packages', packageName);
  const srcDir = path.join(serverDir, 'src');

  // Create directories
  fs.mkdirSync(srcDir, { recursive: true });

  const files: McpServerScaffoldResult['files'] = [];

  // package.json
  const packageJson = {
    name: `@mcp-rebuild/${packageName}`,
    version: '1.0.0',
    main: './dist/index.js',
    types: './dist/index.d.ts',
    scripts: {
      build: 'tsc',
      test: 'vitest run --reporter=verbose',
    },
    dependencies: {
      '@mcp-rebuild/core': 'workspace:*',
    },
  };

  const pkgPath = path.join(serverDir, 'package.json');
  const pkgContent = JSON.stringify(packageJson, null, 2);
  fs.writeFileSync(pkgPath, pkgContent, 'utf-8');
  files.push({ path: pkgPath, content: pkgContent });

  // tsconfig.json
  const tsconfig = {
    extends: '../../tsconfig.base.json',
    compilerOptions: {
      outDir: './dist',
      rootDir: './src',
    },
    include: ['src/**/*'],
    exclude: ['node_modules', 'dist', 'src/**/*.test.ts'],
  };

  const tsconfigPath = path.join(serverDir, 'tsconfig.json');
  const tsconfigContent = JSON.stringify(tsconfig, null, 2);
  fs.writeFileSync(tsconfigPath, tsconfigContent, 'utf-8');
  files.push({ path: tsconfigPath, content: tsconfigContent });

  // Tool stubs
  for (const tool of input.tools) {
    const toolName = tool.name.replace(/[-\s]+/g, '_');
    const toolFileName = `${toolName}.ts`;
    const toolPath = path.join(srcDir, toolFileName);
    const toolContent = [
      '/**',
      ` * ${tool.name}`,
      ` * ${tool.description}`,
      ' */',
      '',
      `export async function ${toolName}(input: unknown): Promise<unknown> {`,
      '  // TODO: Implement',
      '  return { success: true };',
      '}',
      '',
    ].join('\n');

    fs.writeFileSync(toolPath, toolContent, 'utf-8');
    files.push({ path: toolPath, content: toolContent });
  }

  // index.ts
  const indexPath = path.join(srcDir, 'index.ts');
  const exports = input.tools.map((tool) => {
    const toolName = tool.name.replace(/[-\s]+/g, '_');
    return `export { ${toolName} } from './${toolName}.js';`;
  }).join('\n');

  const indexContent = [
    `/**`,
    ` * ${input.name} MCP Server`,
    ` * ${input.description}`,
    ` */`,
    '',
    exports,
    '',
  ].join('\n');

  fs.writeFileSync(indexPath, indexContent, 'utf-8');
  files.push({ path: indexPath, content: indexContent });

  logger.info({ packageName, toolCount: input.tools.length }, 'Scaffolded MCP server');

  return { files, packageName };
}
