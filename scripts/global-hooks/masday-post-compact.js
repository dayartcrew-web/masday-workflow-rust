#!/usr/bin/env node
//
// Masday PostCompact Hook
//
// Fires AFTER Claude Code completes context compaction.
// Injects a recovery reminder so Claude knows to re-read files
// and restore working context.
//
// Hook output schema: { continue: true, systemMessage: "..." }
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
    const now = new Date().toISOString();
    const cwd = data.cwd || process.cwd();

    // Read compact state
    const stateFile = path.join(os.homedir(), ".masday", "compact-state.json");
    let totalCompacts = 0;
    try {
      const state = JSON.parse(fs.readFileSync(stateFile, "utf8"));
      totalCompacts = state.count || 0;
    } catch {}

    // Inject recovery prompt
    const output = {
      continue: true,
      systemMessage: [
        "🔄 CONTEXT COMPACTION COMPLETE",
        "",
        `Total compacts this session: ${totalCompacts}`,
        `Time: ${now}`,
        "",
        "IMPORTANT: Your context was just compacted. You may have lost:",
        "- Recent file reads and edits",
        "- Tool outputs from earlier in the session",
        "- Conversation history details",
        "",
        "Recovery actions:",
        "1. Re-read any files you were actively working on",
        "2. Check git status for current state of changes",
        "3. Review workflow state if in a masday project (masday status)",
        "4. Ask the user if you seem to have lost context",
        "",
        "Do NOT assume the user knows you were compacted. Just recover naturally.",
      ].join("\n"),
    };

    process.stdout.write(JSON.stringify(output));
  } catch {
    process.stdout.write(JSON.stringify({ continue: true }));
  }
});
