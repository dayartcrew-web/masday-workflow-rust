#!/usr/bin/env node
//
// Masday UserPromptSubmit — Context Warning Hook
//
// Runs on every user prompt. Estimates context usage via compact_boundary
// in session JSONL. Caches result for 30s to avoid parsing on every prompt.
//

const fs = require("fs");
const path = require("path");
const os = require("os");

const CONTEXT_WINDOW_TOKENS = 200000;
const BYTES_PER_TOKEN = 4;
const SYSTEM_OVERHEAD_TOKENS = 15000;
const WARN_THRESHOLD = 0.50;
const CRITICAL_THRESHOLD = 0.75;
const CACHE_TTL_MS = 30000; // cache for 30 seconds

const CACHE_FILE = path.join(os.tmpdir(), "masday-context-cache.json");

function getCached() {
  try {
    const raw = fs.readFileSync(CACHE_FILE, "utf8");
    const cache = JSON.parse(raw);
    if (Date.now() - cache.ts < CACHE_TTL_MS) return cache.pct;
  } catch {}
  return null;
}

function setCached(pct) {
  try {
    fs.writeFileSync(CACHE_FILE, JSON.stringify({ pct, ts: Date.now() }));
  } catch {}
}

function estimateContextPct() {
  try {
    const sessionDir = path.join(
      process.env.HOME, ".claude", "projects", "-home-vibe-dev-masday-workflow-rust"
    );
    if (!fs.existsSync(sessionDir)) return 0;

    const files = fs.readdirSync(sessionDir).filter(f => f.endsWith(".jsonl"));
    if (files.length === 0) return 0;

    let latest = null;
    let latestMtime = 0;
    for (const f of files) {
      try {
        const fp = path.join(sessionDir, f);
        const stat = fs.statSync(fp);
        if (stat.mtimeMs > latestMtime) {
          latestMtime = stat.mtimeMs;
          latest = fp;
        }
      } catch {}
    }
    if (!latest) return 0;

    const content = fs.readFileSync(latest, "utf8");
    const lines = content.split("\n");
    let lastBoundaryIdx = -1;

    for (let i = lines.length - 1; i >= 0; i--) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.subtype === "compact_boundary") {
          lastBoundaryIdx = i;
          break;
        }
      } catch {}
    }

    const startIdx = lastBoundaryIdx >= 0 ? lastBoundaryIdx + 1 : 0;
    let activeBytes = 0;
    for (let i = startIdx; i < lines.length; i++) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.type === "user" || obj.type === "assistant") {
          activeBytes += lines[i].length;
        }
      } catch {}
    }

    const tokens = Math.floor(activeBytes / BYTES_PER_TOKEN) + SYSTEM_OVERHEAD_TOKENS;
    return Math.min(1.0, tokens / CONTEXT_WINDOW_TOKENS);
  } catch {
    return 0;
  }
}

async function main() {
  // Use cached result if fresh
  let pct = getCached();
  if (pct === null) {
    pct = estimateContextPct();
    setCached(pct);
  }
  const pctDisplay = Math.round(pct * 100);

  if (pct >= CRITICAL_THRESHOLD) {
    process.stdout.write(JSON.stringify({
      continue: true,
      systemMessage:
        `🔴 Context at ~${pctDisplay}%. Auto-compact will trigger soon.\n` +
        `Consider running /compact now to preserve important context.`,
    }));
    return;
  }

  if (pct >= WARN_THRESHOLD) {
    process.stdout.write(JSON.stringify({
      continue: true,
      systemMessage:
        `🟡 Context at ~${pctDisplay}%. Approaching auto-compact threshold.\n` +
        `If working on complex multi-step tasks, consider /compact soon.`,
    }));
    return;
  }

  process.stdout.write(JSON.stringify({ continue: true }));
}

main().catch(() => process.stdout.write(JSON.stringify({ continue: true })));
