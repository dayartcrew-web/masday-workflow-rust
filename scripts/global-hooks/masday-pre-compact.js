#!/usr/bin/env node
//
// Masday PreCompact Hook
//
// Fires BEFORE Claude Code auto-compacts the context window.
// Saves context to masday memory (if API available) and injects
// a system reminder about what to preserve.
//
// Hook output schema: { continue: true, systemMessage: "..." }
// systemMessage is injected into Claude's context before compaction.
//

const fs = require("fs");
const path = require("path");
const os = require("os");

let input = '';
const stdinTimeout = setTimeout(() => process.exit(0), 10000);
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', () => {
  clearTimeout(stdinTimeout);
  try {
    const data = JSON.parse(input);
    const sessionId = data.session_id;
    const now = new Date().toISOString();
    const cwd = data.cwd || process.cwd();

    // Write compact event to bridge file for statusline
    if (sessionId && !/[/\\]|\.\./.test(sessionId)) {
      try {
        const bridgePath = path.join(os.tmpdir(), `claude-ctx-${sessionId}.json`);
        fs.writeFileSync(bridgePath, JSON.stringify({
          session_id: sessionId,
          remaining_percentage: 0,
          used_pct: 100,
          timestamp: Math.floor(Date.now() / 1000),
          compacting: true
        }));
      } catch {}
    }

    // Save session state snapshot
    const stateFile = path.join(os.homedir(), ".masday", "compact-state.json");
    let state = { count: 0, lastCompact: null };
    try { state = JSON.parse(fs.readFileSync(stateFile, "utf8")); } catch {}
    state.count++;
    state.lastCompact = now;
    state.lastCwd = cwd;
    try { fs.mkdirSync(path.dirname(stateFile), { recursive: true }); } catch {}
    try { fs.writeFileSync(stateFile, JSON.stringify(state, null, 2)); } catch {}

    // Inject preservation reminder
    const output = {
      continue: true,
      systemMessage: [
        "⚠️ CONTEXT COMPACTION IMMINENT ⚠️",
        "",
        "Before compacting, preserve:",
        "1. Current workflow state (workflow_id, task_id, plan_id)",
        "2. Active file edits in progress",
        "3. Uncommitted changes status (run git status)",
        "4. Key decisions made this session",
        "5. Any important context the user shared",
        "",
        "After compaction, you will receive a recovery reminder.",
        "Re-read important files to restore your working context."
      ].join("\n"),
    };

    process.stdout.write(JSON.stringify(output));
  } catch {
    process.stdout.write(JSON.stringify({ continue: true }));
  }
});
