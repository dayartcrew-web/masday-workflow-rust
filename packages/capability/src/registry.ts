/**
 * Capability Registry
 *
 * Scans .claude/ directories for agent, skill, and command definitions.
 * Parses YAML frontmatter from markdown files and maintains a registry.
 */

import fs from 'fs';
import path from 'path';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('CapabilityRegistry');

// --- Types ---

export interface AgentEntry {
  name: string;
  role: string;
  description: string;
  skills: string[];
  file: string;
  status: 'active' | 'inactive';
}

export interface SkillEntry {
  name: string;
  trigger: string;
  description: string;
  file: string;
}

export interface CommandEntry {
  name: string;
  description: string;
  file: string;
}

export interface Registry {
  version: number;
  agents: AgentEntry[];
  skills: SkillEntry[];
  commands: CommandEntry[];
}

// --- Frontmatter Parsing ---

/**
 * Parse YAML-like frontmatter from a markdown file.
 *
 * Expects content to start with `---`, followed by key: value pairs,
 * and a closing `---`. Returns a map of key-value pairs.
 */
export function parseFrontmatter(content: string): Record<string, string> {
  const result: Record<string, string> = {};

  const trimmed = content.trimStart();
  if (!trimmed.startsWith('---')) {
    return result;
  }

  // Find closing ---
  const endIndex = trimmed.indexOf('---', 3);
  if (endIndex === -1) {
    return result;
  }

  const frontmatter = trimmed.slice(3, endIndex).trim();
  const lines = frontmatter.split('\n');

  for (const line of lines) {
    const colonIndex = line.indexOf(':');
    if (colonIndex === -1) continue;

    const key = line.slice(0, colonIndex).trim();
    const value = line.slice(colonIndex + 1).trim();
    result[key] = value;
  }

  return result;
}

// --- Registry File Operations ---

const REGISTRY_FILE = 'registry.json';

function getRegistryPath(projectRoot: string): string {
  const claudeDir = path.join(projectRoot, '.claude');
  if (!fs.existsSync(claudeDir)) {
    fs.mkdirSync(claudeDir, { recursive: true });
  }
  return path.join(claudeDir, REGISTRY_FILE);
}

/**
 * Load the registry from the project's .claude/ directory.
 * Returns a fresh registry if none exists.
 */
export function loadRegistry(projectRoot: string): Registry {
  const registryPath = getRegistryPath(projectRoot);

  if (!fs.existsSync(registryPath)) {
    return initializeRegistry(projectRoot);
  }

  try {
    const content = fs.readFileSync(registryPath, 'utf-8');
    const parsed = JSON.parse(content) as Registry;
    logger.info({ projectRoot }, 'Loaded capability registry');
    return parsed;
  } catch {
    logger.warn({ registryPath }, 'Failed to parse registry, reinitializing');
    return initializeRegistry(projectRoot);
  }
}

/**
 * Save the registry to the project's .claude/ directory.
 */
export function saveRegistry(projectRoot: string, registry: Registry): void {
  const registryPath = getRegistryPath(projectRoot);
  fs.writeFileSync(registryPath, JSON.stringify(registry, null, 2), 'utf-8');
  logger.info({ projectRoot }, 'Saved capability registry');
}

/**
 * Initialize a fresh registry.
 */
export function initializeRegistry(projectRoot: string): Registry {
  const registry: Registry = {
    version: 1,
    agents: [],
    skills: [],
    commands: [],
  };

  // Scan existing files and populate
  registry.agents = scanExistingAgents(projectRoot);
  registry.skills = scanExistingSkills(projectRoot);
  registry.commands = scanExistingCommands(projectRoot);

  saveRegistry(projectRoot, registry);
  logger.info(
    {
      projectRoot,
      agents: registry.agents.length,
      skills: registry.skills.length,
      commands: registry.commands.length,
    },
    'Initialized capability registry',
  );

  return registry;
}

// --- Scanning ---

/**
 * Scan .claude/agents/ for agent definition files.
 *
 * Parses frontmatter from each .md file to extract name, role,
 * description, and skills list.
 */
export function scanExistingAgents(projectRoot: string): AgentEntry[] {
  const agentsDir = path.join(projectRoot, '.claude', 'agents');
  const entries: AgentEntry[] = [];

  if (!fs.existsSync(agentsDir)) {
    return entries;
  }

  const files = fs.readdirSync(agentsDir).filter((f) => f.endsWith('.md'));

  for (const file of files) {
    const filePath = path.join(agentsDir, file);
    try {
      const content = fs.readFileSync(filePath, 'utf-8');
      const fm = parseFrontmatter(content);

      entries.push({
        name: fm.name ?? path.basename(file, '.md'),
        role: fm.role ?? 'general',
        description: fm.description ?? '',
        skills: fm.skills ? fm.skills.split(',').map((s) => s.trim()) : [],
        file,
        status: (fm.status as AgentEntry['status']) ?? 'active',
      });
    } catch {
      logger.warn({ file }, 'Failed to parse agent file');
    }
  }

  return entries;
}

/**
 * Scan .claude/skills/ for skill definition files.
 */
export function scanExistingSkills(projectRoot: string): SkillEntry[] {
  const skillsDir = path.join(projectRoot, '.claude', 'skills');
  const entries: SkillEntry[] = [];

  if (!fs.existsSync(skillsDir)) {
    return entries;
  }

  const files = fs.readdirSync(skillsDir).filter((f) => f.endsWith('.md'));

  for (const file of files) {
    const filePath = path.join(skillsDir, file);
    try {
      const content = fs.readFileSync(filePath, 'utf-8');
      const fm = parseFrontmatter(content);

      entries.push({
        name: fm.name ?? path.basename(file, '.md'),
        trigger: fm.trigger ?? '',
        description: fm.description ?? '',
        file,
      });
    } catch {
      logger.warn({ file }, 'Failed to parse skill file');
    }
  }

  return entries;
}

/**
 * Scan .claude/commands/ for command definition files.
 */
export function scanExistingCommands(projectRoot: string): CommandEntry[] {
  const commandsDir = path.join(projectRoot, '.claude', 'commands');
  const entries: CommandEntry[] = [];

  if (!fs.existsSync(commandsDir)) {
    return entries;
  }

  const files = fs.readdirSync(commandsDir).filter((f) => f.endsWith('.md'));

  for (const file of files) {
    const filePath = path.join(commandsDir, file);
    try {
      const content = fs.readFileSync(filePath, 'utf-8');
      const fm = parseFrontmatter(content);

      entries.push({
        name: fm.name ?? path.basename(file, '.md'),
        description: fm.description ?? '',
        file,
      });
    } catch {
      logger.warn({ file }, 'Failed to parse command file');
    }
  }

  return entries;
}

// --- Registration ---

/**
 * Register an agent entry. Updates if name already exists.
 */
export function registerAgent(projectRoot: string, entry: AgentEntry): Registry {
  const registry = loadRegistry(projectRoot);
  const existingIndex = registry.agents.findIndex((a) => a.name === entry.name);

  if (existingIndex >= 0) {
    registry.agents[existingIndex] = entry;
  } else {
    registry.agents.push(entry);
  }

  saveRegistry(projectRoot, registry);
  return registry;
}

/**
 * Register a skill entry. Updates if name already exists.
 */
export function registerSkill(projectRoot: string, entry: SkillEntry): Registry {
  const registry = loadRegistry(projectRoot);
  const existingIndex = registry.skills.findIndex((s) => s.name === entry.name);

  if (existingIndex >= 0) {
    registry.skills[existingIndex] = entry;
  } else {
    registry.skills.push(entry);
  }

  saveRegistry(projectRoot, registry);
  return registry;
}

/**
 * Register a command entry. Updates if name already exists.
 */
export function registerCommand(projectRoot: string, entry: CommandEntry): Registry {
  const registry = loadRegistry(projectRoot);
  const existingIndex = registry.commands.findIndex((c) => c.name === entry.name);

  if (existingIndex >= 0) {
    registry.commands[existingIndex] = entry;
  } else {
    registry.commands.push(entry);
  }

  saveRegistry(projectRoot, registry);
  return registry;
}
