#!/usr/bin/env node
// Auto-generate .claude/registry.json by scanning .claude/agents/ and .claude/skills/
// Usage: node scripts/registry-sync.mjs [--dry-run]
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'fs';
import { join } from 'path';

const ROOT = process.cwd();
const DRY = process.argv.includes('--dry-run');
const REGISTRY_PATH = join(ROOT, '.claude/registry.json');

function parseFrontmatter(content) {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};
  const fm = {};
  match[1].split('\n').forEach(line => {
    const idx = line.indexOf(':');
    if (idx > 0) {
      const key = line.slice(0, idx).trim();
      let val = line.slice(idx + 1).trim();
      // Remove quotes
      if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'")))
        val = val.slice(1, -1);
      // Handle multiline >
      if (val.startsWith('>')) val = val.slice(1).trim();
      fm[key] = val;
    }
  });
  return fm;
}

function scanAgents() {
  const dir = join(ROOT, '.claude/agents');
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter(f => f.endsWith('.md')).map(f => {
    const content = readFileSync(join(dir, f), 'utf8');
    const fm = parseFrontmatter(content);
    return {
      name: fm.name || f.replace('.md', ''),
      file: `.claude/agents/${f}`,
      model: fm.model || 'sonnet',
      category: inferCategory(fm.description || ''),
      description: (fm.description || '').split('\n')[0].trim(),
    };
  }).sort((a, b) => a.name.localeCompare(b.name));
}

function scanSkills() {
  const dir = join(ROOT, '.claude/skills');
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => {
      const skillFile = join(dir, d.name, 'SKILL.md');
      if (!existsSync(skillFile)) return null;
      const content = readFileSync(skillFile, 'utf8');
      const fm = parseFrontmatter(content);
      return {
        name: fm.name || d.name,
        directory: `.claude/skills/${d.name}`,
        category: inferCategory(fm.description || ''),
        description: (fm.description || '').split('\n')[0].trim(),
      };
    })
    .filter(Boolean)
    .sort((a, b) => a.name.localeCompare(b.name));
}

function inferCategory(desc) {
  const d = desc.toLowerCase();
  if (d.includes('test') || d.includes('tdd') || d.includes('e2e')) return 'quality';
  if (d.includes('review') || d.includes('audit') || d.includes('security')) return 'quality';
  if (d.includes('workflow') || d.includes('orchestr')) return 'orchestration';
  if (d.includes('research') || d.includes('analyz') || d.includes('detect')) return 'analysis';
  if (d.includes('deploy') || d.includes('docker') || d.includes('git') || d.includes('ci')) return 'operations';
  if (d.includes('scaffold') || d.includes('create')) return 'scaffolding';
  if (d.includes('build') || d.includes('implement') || d.includes('frontend') || d.includes('component')) return 'implementation';
  if (d.includes('doc')) return 'documentation';
  return 'general';
}

// Load existing registry for hooks/mcpServers/docs
let existing = { version: 1, components: {} };
if (existsSync(REGISTRY_PATH)) {
  existing = JSON.parse(readFileSync(REGISTRY_PATH, 'utf8'));
}

const registry = {
  version: 1,
  components: {
    agents: scanAgents(),
    skills: scanSkills(),
    hooks: existing.components?.hooks || [],
    mcpServers: existing.components?.mcpServers || [],
    docs: existing.components?.docs || [],
  },
};

const agentCount = registry.components.agents.length;
const skillCount = registry.components.skills.length;

if (DRY) {
  console.log(`[DRY RUN] Would write ${agentCount} agents, ${skillCount} skills`);
  console.log(JSON.stringify(registry, null, 2));
} else {
  writeFileSync(REGISTRY_PATH, JSON.stringify(registry, null, 2) + '\n');
  console.log(`✅ Registry updated: ${agentCount} agents, ${skillCount} skills`);
}
