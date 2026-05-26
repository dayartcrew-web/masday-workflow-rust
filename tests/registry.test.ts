import { describe, it, expect } from 'vitest'
import { readFileSync, existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = join(__dirname, '..')
const registryPath = join(ROOT, '.claude', 'registry.json')
const registryExists = existsSync(registryPath)
const registry = registryExists
  ? JSON.parse(readFileSync(registryPath, 'utf-8'))
  : { components: { agents: [], skills: [], hooks: [], mcpServers: [] } }

describe.skipIf(!registryExists)('Registry consistency', () => {

  it('all registered agents exist as files', () => {
    for (const agent of registry.components.agents) {
      const path = join(ROOT, agent.file)
      expect(existsSync(path), `Agent file missing: ${agent.file}`).toBe(true)
    }
  })

  it('all registered skills exist as directories', () => {
    for (const skill of registry.components.skills) {
      const path = join(ROOT, skill.directory)
      expect(existsSync(path), `Skill directory missing: ${skill.directory}`).toBe(true)
      expect(
        existsSync(join(path, 'SKILL.md')),
        `SKILL.md missing in ${skill.directory}`
      ).toBe(true)
    }
  })

  it('all registered hooks exist as files', () => {
    for (const hook of registry.components.hooks) {
      const path = join(ROOT, hook.file)
      expect(existsSync(path), `Hook file missing: ${hook.file}`).toBe(true)
    }
  })

  it('all registered MCP server paths exist', () => {
    for (const server of registry.components.mcpServers) {
      const path = join(ROOT, server.path)
      expect(existsSync(path), `MCP server path missing: ${server.path}`).toBe(true)
    }
  })

  it('no orphan agent files', () => {
    const agentFiles = readdirSync(join(ROOT, '.claude', 'agents'))
      .filter(f => f.endsWith('.md'))
    const registeredNames = new Set(registry.components.agents.map(a => a.file.split('/').pop()))
    for (const f of agentFiles) {
      expect(registeredNames.has(f), `Orphan agent file: ${f}`).toBe(true)
    }
  })

  it('no orphan skill directories', () => {
    const skillDirs = readdirSync(join(ROOT, '.claude', 'skills'), { withFileTypes: true })
      .filter(d => d.isDirectory())
      .map(d => d.name)
    const registeredNames = new Set(registry.components.skills.map(s => s.directory.split('/').pop()))
    for (const d of skillDirs) {
      expect(registeredNames.has(d), `Orphan skill directory: ${d}`).toBe(true)
    }
  })

  it('registry has correct component counts', () => {
    expect(registry.components.agents.length).toBe(27)
    expect(registry.components.skills.length).toBe(36)
    expect(registry.components.hooks.length).toBe(14)
    expect(registry.components.mcpServers.length).toBe(1)
  })

  it('every agent has required fields', () => {
    for (const agent of registry.components.agents) {
      expect(agent).toHaveProperty('name')
      expect(agent).toHaveProperty('file')
      expect(agent).toHaveProperty('model')
      expect(agent).toHaveProperty('category')
      expect(agent.file).toMatch(/^\.claude\/agents\/.+\.md$/)
      expect(['haiku', 'sonnet', 'opus']).toContain(agent.model)
    }
  })

  it('every skill has required fields', () => {
    for (const skill of registry.components.skills) {
      expect(skill).toHaveProperty('name')
      expect(skill).toHaveProperty('directory')
      expect(skill).toHaveProperty('category')
      expect(skill.directory).toMatch(/^\.claude\/skills\//)
    }
  })

  it('every hook has required fields', () => {
    for (const hook of registry.components.hooks) {
      expect(hook).toHaveProperty('name')
      expect(hook).toHaveProperty('file')
      expect(hook).toHaveProperty('type')
      expect(['executable', 'advisory']).toContain(hook.type)
    }
  })

  it('executable hooks have .js or .mjs extension', () => {
    for (const hook of registry.components.hooks) {
      if (hook.type === 'executable') {
        expect(hook.file).toMatch(/\.(js|mjs)$/)
      }
    }
  })

  it('advisory hooks have .md extension', () => {
    for (const hook of registry.components.hooks) {
      if (hook.type === 'advisory') {
        expect(hook.file).toMatch(/\.md$/)
      }
    }
  })
})
