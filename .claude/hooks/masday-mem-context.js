#!/usr/bin/env node
// ============================================================
// SessionStart hook — Auto-inject relevant memories
// Reads .masday/state/memories.json from the project directory
// Filters by project context, outputs top memories
// ============================================================

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const PROJECT_DIR = process.cwd()
const MEMORIES_FILE = path.join(PROJECT_DIR, '.masday', 'state', 'memories.json')
const HOOK_DIR = path.dirname(fileURLToPath(import.meta.url))
const CONFIG_FILE = path.join(HOOK_DIR, 'masday-mem-config.json')

const DEFAULT_CONFIG = { maxMemories: 15, foreignDomains: {}, matchThreshold: 2 }

function loadConfig() {
  try {
    const raw = fs.readFileSync(CONFIG_FILE, 'utf-8')
    return { ...DEFAULT_CONFIG, ...JSON.parse(raw) }
  } catch {
    return DEFAULT_CONFIG
  }
}

function isForeignMemory(memory, config) {
  const tags = (memory.tags || []).map(t => t.toLowerCase())
  const content = (memory.content || '').toLowerCase()
  const summary = (memory.summary || '').toLowerCase()
  const text = `${tags.join(' ')} ${content} ${summary}`

  for (const domain of Object.values(config.foreignDomains)) {
    const matchCount = domain.filter(kw => text.includes(kw)).length
    if (matchCount >= config.matchThreshold) return true
  }
  return false
}

try {
  const config = loadConfig()

  if (!fs.existsSync(MEMORIES_FILE)) {
    console.log('[masday-mem] No memories stored yet. Memories will be saved as you work.')
    process.exit(0)
  }

  const raw = fs.readFileSync(MEMORIES_FILE, 'utf-8')
  const data = JSON.parse(raw)
  const allMemories = Array.isArray(data) ? data : (data.memories ?? [])

  if (allMemories.length === 0) {
    console.log('[masday-mem] Memory store is empty. Start by asking me to remember something.')
    process.exit(0)
  }

  // Filter: separate project-relevant from foreign
  const relevant = []
  const foreign = []
  for (const m of allMemories) {
    if (isForeignMemory(m, config)) {
      foreign.push(m)
    } else {
      relevant.push(m)
    }
  }

  // Score memories: importance * 0.5 + recency * 0.5
  const now = Date.now()
  const scoreMemory = m => {
    const ageDays = (now - (m.updatedAt || m.createdAt || now)) / 86400000
    const recency = Math.exp(-0.693 * ageDays / 7)
    const importance = m.importance ?? 0.5
    return { ...m, score: importance * 0.5 + recency * 0.5 }
  }

  const scored = relevant.map(scoreMemory)
  scored.sort((a, b) => b.score - a.score)
  const top = scored.slice(0, config.maxMemories)

  // Output as context injection
  console.log(`[masday-mem] Loaded ${allMemories.length} memories (${foreign.length} foreign filtered), showing top ${top.length}:`)
  console.log('')

  // Group by type
  const byType = {}
  for (const m of top) {
    const type = m.type || 'fact'
    if (!byType[type]) byType[type] = []
    byType[type].push(m)
  }

  for (const [type, items] of Object.entries(byType)) {
    console.log(`[${type.toUpperCase()}]`)
    for (const m of items) {
      const age = Math.round((now - (m.updatedAt || m.createdAt || now)) / 86400000)
      const content = m.content.length > 150 ? m.content.slice(0, 150) + '...' : m.content
      console.log(`  - ${content} (importance: ${m.importance ?? 0.5}, ${age}d ago)`)
    }
    console.log('')
  }

  // Stats
  const typeCounts = {}
  for (const m of relevant) {
    const t = m.type || 'fact'
    typeCounts[t] = (typeCounts[t] || 0) + 1
  }

  console.log(`Stats: ${relevant.length} relevant | ${Object.entries(typeCounts).map(([k, v]) => `${k}:${v}`).join(' ')}`)
  if (foreign.length > 0) {
    console.log(`Filtered: ${foreign.length} foreign-domain memories excluded`)
  }
  console.log('Use memory_search for specific queries, memory_store to store new context.')

} catch (err) {
  console.log(`[masday-mem] Could not load memories: ${err.message}`)
}

export default function () {}
