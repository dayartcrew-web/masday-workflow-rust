#!/usr/bin/env node
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'fs';
import { join, basename } from 'path';

const CLAUDE_TOOL_TO_OPENCODE = {
  'read': 'read',
  'write': 'write',
  'edit': 'edit',
  'bash': 'bash',
  'glob': 'glob',
  'grep': 'grep',
  'todowrite': 'todowrite',
};

const OPENCODE_CORE_TOOLS = ['read', 'write', 'edit', 'bash', 'glob', 'grep'];

function parseFrontmatter(content) {
  const stripped = content.replace(/^\uFEFF/, '');
  const match = stripped.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);
  if (!match) return null;
  const raw = match[1];
  const body = match[2] || '';
  const meta = {};
  const lines = raw.split(/\r?\n/);
  let currentKey = null;
  let inFolded = false;
  let foldedLines = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (inFolded) {
      if (line.match(/^\s+(\S)/)) {
        foldedLines.push(line.trim());
        i++;
        continue;
      } else {
        meta[currentKey] = foldedLines.join(' ');
        inFolded = false;
        foldedLines = [];
      }
    }
    // Array list item
    const listMatch = line.match(/^\s+-\s+(.+)$/);
    if (listMatch && currentKey && Array.isArray(meta[currentKey])) {
      meta[currentKey].push(listMatch[1]);
      i++;
      continue;
    }
    // Key-value line
    const kvMatch = line.match(/^(\w[\w.-]*):\s*(.*)$/);
    if (kvMatch) {
      currentKey = kvMatch[1];
      const val = kvMatch[2].trimEnd();
      if (val === '') {
        // Check if next line is indented (folded or list)
        const nextLine = lines[i + 1];
        if (nextLine && nextLine.match(/^\s+-\s+/)) {
          meta[currentKey] = [];
          i++;
          continue;
        }
        if (nextLine && nextLine.match(/^\s+(\S)/)) {
          // Plain indented continuation (YAML folded without >)
          inFolded = true;
          foldedLines = [];
          i++;
          continue;
        }
        meta[currentKey] = '';
      } else if (val === '>') {
        // Folded block starts on next line
        const nextLine = lines[i + 1];
        if (nextLine && nextLine.match(/^\s+(\S)/)) {
          inFolded = true;
          foldedLines = [];
          i++;
          continue;
        }
        meta[currentKey] = '';
      } else if (val.startsWith('> ')) {
        // Folded block with content on same line
        const firstPart = val.slice(2);
        const nextLine = lines[i + 1];
        if (nextLine && nextLine.match(/^\s+(\S)/)) {
          inFolded = true;
          foldedLines = [firstPart];
          i++;
          continue;
        }
        meta[currentKey] = firstPart;
      } else {
        meta[currentKey] = val;
      }
    }
    i++;
  }
  if (inFolded && currentKey) {
    meta[currentKey] = foldedLines.join(' ');
  }
  return { meta, body };
}

function convertClaudeAgentToOpencode(sourceContent, filename) {
  const parsed = parseFrontmatter(sourceContent);
  if (!parsed) {
    console.error(`  SKIP ${filename}: no valid frontmatter`);
    return null;
  }
  const { meta, body } = parsed;

  const opencodeName = basename(filename, '.md');
  let description = meta.description || '';
  if (typeof description === 'object') description = '';

  const claudeTools = Array.isArray(meta.tools) ? meta.tools : [];
  const opencodeTools = {};
  const allKnownTools = [...OPENCODE_CORE_TOOLS, 'todowrite'];
  let sourceHasWrite = false;
  let sourceHasEdit = false;
  for (const tool of claudeTools) {
    const normalized = tool.toLowerCase();
    if (allKnownTools.includes(normalized)) {
      opencodeTools[normalized] = true;
      if (normalized === 'write') sourceHasWrite = true;
      if (normalized === 'edit') sourceHasEdit = true;
    }
  }
  if (!sourceHasWrite && Object.keys(opencodeTools).length > 0) opencodeTools.write = false;
  if (!sourceHasEdit && Object.keys(opencodeTools).length > 0) opencodeTools.edit = false;

  const orderedTools = {};
  for (const t of OPENCODE_CORE_TOOLS) {
    if (opencodeTools[t] !== undefined) orderedTools[t] = opencodeTools[t];
  }
  if (opencodeTools.todowrite) orderedTools.todowrite = true;

  // Map Claude model to opencode temperature
  const modelMap = { sonnet: '0.2', haiku: '0.3', opus: '0.1' };
  const temperature = modelMap[meta.model] || '0.2';

  // NOTE: todowrite moved to description body, not frontmatter tools
  const hasTodoWrite = orderedTools.todowrite;
  delete orderedTools.todowrite;

  const role = meta.role || description;
  let frontmatter = `---\nname: ${opencodeName}\ndescription: ${description}\nrole: ${role}\n`;
  frontmatter += '---\n';

  let bodyContent = body.trim();
  if (bodyContent && !bodyContent.startsWith('\n')) {
    bodyContent = '\n' + bodyContent;
  }

  return frontmatter + '\n' + bodyContent + '\n';
}

function convertAll(sourceDir, targetDir) {
  if (!existsSync(sourceDir)) {
    console.error(`Source directory not found: ${sourceDir}`);
    process.exit(1);
  }
  mkdirSync(targetDir, { recursive: true });

  const files = readdirSync(sourceDir).filter(f => f.startsWith('masday-') && f.endsWith('.md'));
  let converted = 0;
  let skipped = 0;

  for (const file of files) {
    const src = join(sourceDir, file);
    const dst = join(targetDir, file);
    const content = readFileSync(src, 'utf-8');
    const result = convertClaudeAgentToOpencode(content, file);
    if (result) {
      writeFileSync(dst, result, 'utf-8');
      converted++;
    } else {
      skipped++;
    }
  }

  console.log(`Converted ${converted} agents, skipped ${skipped}`);
  return { converted, skipped };
}

const [,, command, ...args] = process.argv;

if (command === 'convert') {
  const sourceDir = args[0] || join(process.cwd(), '.claude', 'agents');
  const cwd = process.cwd();

  // 1) Global agents
  const globalTarget = join(process.env.HOME || process.env.USERPROFILE, '.config', 'opencode', 'agent');
  console.log(`Converting agents: ${sourceDir} -> ${globalTarget}`);
  convertAll(sourceDir, globalTarget);

  // 2) Project agents (openagents reference shows .opencode/agent/)
  const projectTarget = join(cwd, '.opencode', 'agent');
  if (projectTarget !== globalTarget) {
    console.log(`Syncing to project: ${sourceDir} -> ${projectTarget}`);
    convertAll(sourceDir, projectTarget);
  }
} else if (command === 'convert-to-dir') {
  const sourceDir = args[0] || join(process.cwd(), '.claude', 'agents');
  const targetDir = args[1];
  if (!targetDir) {
    console.error('Usage: convert-agents.mjs convert-to-dir <sourceDir> <targetDir>');
    process.exit(1);
  }
  console.log(`Converting agents: ${sourceDir} -> ${targetDir}`);
  convertAll(sourceDir, targetDir);
} else {
  console.log('Usage: convert-agents.mjs <convert|convert-to-dir> [sourceDir] [targetDir]');
  console.log('  convert           Convert .claude/agents to ~/.config/opencode/agent');
  console.log('  convert-to-dir    Convert to a specific target directory');
  process.exit(1);
}