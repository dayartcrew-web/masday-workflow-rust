#!/usr/bin/env node
//
// Masday PostCompact Hook
//
// Fires AFTER Claude Code completes context compaction.
// Uses valid hook output schema: systemMessage (not hookSpecificOutput).
//

const fs = require("fs");
const path = require("path");
const os = require("os");

const LOG_FILE = path.join(process.env.HOME, ".claude", "compact-log.jsonl");
const STATE_FILE = path.join(process.env.HOME, ".claude", "compact-state.json");
const CONTEXT_CACHE_FILE = path.join(os.tmpdir(), "masday-context-cache.json");

function log(entry) {
  try {
    fs.appendFileSync(LOG_FILE, JSON.stringify(entry) + "\n");
  } catch {}
}

function getState() {
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, "utf8"));
  } catch {
    return { count: 0, lastCompact: null };
  }
}

function saveState(state) {
  try {
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
  } catch {}
}

async function main() {
  const now = new Date().toISOString();
  const state = getState();
  state.count++;
  state.lastCompact = now;
  state.lastCwd = process.cwd();
  saveState(state);

  // Reset context cache to 0 — context was just compacted
  try {
    fs.writeFileSync(CONTEXT_CACHE_FILE, JSON.stringify({ pct: 0, ts: Date.now() }));
  } catch {}

  log({
    event: "PostCompact",
    timestamp: now,
    cwd: process.cwd(),
    totalCompacts: state.count,
  });

  // Valid output schema — systemMessage injects recovery prompt into Claude
  const output = {
    continue: true,
    systemMessage: [
      "🔄 CONTEXT COMPACTION COMPLETE",
      "",
      `Total compacts this session: ${state.count}`,
      `Last compact: ${now}`,
      "",
      "IMPORTANT: Your context was just compacted. You may have lost:",
      "- Recent file reads and edits",
      "- Tool outputs from earlier in the session",
      "- Conversation history details",
      "",
      "Recovery actions:",
      "1. Check masday memory for auto-saved context",
      "2. Re-read any files you were working on",
      "3. Check git status for current state",
      "4. Review workflow state if in a masday project",
    ].join("\n"),
  };

  process.stdout.write(JSON.stringify(output));
}

main().catch(() => process.stdout.write(JSON.stringify({ continue: true })));
