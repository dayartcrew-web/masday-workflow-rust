#!/usr/bin/env node
// ============================================================
// SessionStart hook — Auto-inject relevant memories
// Reads .masday/state/memories.json from the project directory
// Outputs top memories as context for the session
// ============================================================

const fs = require('fs')
const path = require('path')

const PROJECT_DIR = process.cwd()
const MEMORIES_FILE = path.join(PROJECT_DIR, '.masday', 'state', 'memories.json')

try {
  if (!fs.existsSync(MEMORIES_FILE)) {
    console.log('[agentic-mem] No memories stored yet. Memories will be saved as you work.')
    process.exit(0)
  }

  const raw = fs.readFileSync(MEMORIES_FILE, 'utf-8')
  const data = JSON.parse(raw)

  const memories = Array.isArray(data) ? data : (data.memories ?? [])

  if (memories.length === 0) {
    console.log('[agentic-mem] Memory store is empty. Start by asking me to remember something.')
    process.exit(0)
  }

  // Score memories: aligned with 4-layer memory spec weighting
  // Spec: similarity*0.6 + recency*0.15 + importance*0.15 + usage*0.1
  // Local file has no similarity/usage vectors, so redistribute:
  //   importance*0.75 + recency*0.25 (preserves relative weight ratio from spec)
  const now = Date.now()
  const scored = memories.map(m => {
    const ageDays = (now - (m.updatedAt || m.createdAt || now)) / (86400000)
    const recency = Math.exp(-0.693 * ageDays / 7)
    const importance = m.importance ?? 0.5
    return { ...m, score: importance * 0.75 + recency * 0.25 }
  })

  // Sort by score, take top 15
  scored.sort((a, b) => b.score - a.score)
  const top = scored.slice(0, 15)

  // Output as context injection
  const total = memories.length
  console.log(`[agentic-mem] Loaded ${total} memories, showing top ${top.length}:`)
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

  // Get stats
  const typeCounts = {}
  for (const m of memories) {
    const t = m.type || 'fact'
    typeCounts[t] = (typeCounts[t] || 0) + 1
  }

  console.log(`Stats: ${total} total | ${Object.entries(typeCounts).map(([k, v]) => `${k}:${v}`).join(' ')}`)
  console.log('Use memory.search for specific queries, memory.store to store new context.')

} catch (err) {
  // Silent fail — don't block session start
  console.log(`[agentic-mem] Could not load memories: ${err.message}`)
  process.exit(0)
}
