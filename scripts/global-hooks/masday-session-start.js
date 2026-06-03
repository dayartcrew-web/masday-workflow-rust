#!/usr/bin/env node
//
// Masday Session Start — context bootstrap
//
// Runs on SessionStart. Injects masday project state into the session
// so the agent knows about workflows, tasks, and config.
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
    const cwd = data.cwd || process.cwd();
    const lines = [];

    // Check if in a masday project
    const hasAgents = fs.existsSync(path.join(cwd, ".claude", "agents"));
    const hasConfig = fs.existsSync(path.join(os.homedir(), ".masday", "config.toml"));

    if (hasAgents || hasConfig) {
      lines.push("⚡ Masday session started.");

      // Read config summary
      if (hasConfig) {
        try {
          const config = fs.readFileSync(path.join(os.homedir(), ".masday", "config.toml"), "utf8");
          const modeMatch = config.match(/mode\s*=\s*"([^"]+)"/);
          if (modeMatch) lines.push(`  Mode: ${modeMatch[1]}`);
        } catch {}
      }

      // Check active workflows
      const stateFile = path.join(os.homedir(), ".masday", "compact-state.json");
      if (fs.existsSync(stateFile)) {
        try {
          const state = JSON.parse(fs.readFileSync(stateFile, "utf8"));
          if (state.lastCompact) {
            lines.push(`  Last compact: ${state.lastCompact} (${state.count} total)`);
          }
        } catch {}
      }
    }

    if (lines.length > 0) {
      const output = {
        hookSpecificOutput: {
          hookEventName: "SessionStart",
          additionalContext: lines.join("\n")
        }
      };
      process.stdout.write(JSON.stringify(output));
    } else {
      process.exit(0);
    }
  } catch {
    process.exit(0);
  }
});
