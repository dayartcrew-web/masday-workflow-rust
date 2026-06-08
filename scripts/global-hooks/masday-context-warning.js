#!/usr/bin/env node
//
// Masday UserPromptSubmit — Context Warning Hook
//
// Runs on every user prompt. Reads context metrics from the statusline
// bridge file (written by masday-statusline.js) and injects a warning
// when context is high.
//
// This is a secondary check — the primary context monitoring is done by
// masday-context-monitor.js (PostToolUse). This hook catches cases where
// the user submits a prompt without any tool use in between.
//

const fs = require("fs");
const path = require("path");
const os = require("os");

const CRITICAL_USED_PCT = 75;

let input = '';
const stdinTimeout = setTimeout(() => process.exit(0), 10000);
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', () => {
  clearTimeout(stdinTimeout);
  try {
    const data = JSON.parse(input);
    const sessionId = data.session_id;

    if (!sessionId) { process.exit(0); }
    if (/[/\\]|\.\./.test(sessionId)) { process.exit(0); }

    const metricsPath = path.join(os.tmpdir(), `claude-ctx-${sessionId}.json`);
    if (!fs.existsSync(metricsPath)) { process.exit(0); }

    const metrics = JSON.parse(fs.readFileSync(metricsPath, 'utf8'));
    const now = Math.floor(Date.now() / 1000);

    // Ignore stale metrics
    if (metrics.timestamp && (now - metrics.timestamp) > 60) { process.exit(0); }

    const remaining = metrics.remaining_percentage;
    const usedPct = metrics.used_pct;

    // Only warn at critical level on UserPromptSubmit (PostToolUse handles warning level)
    if (usedPct >= CRITICAL_USED_PCT) {
      const output = {
        continue: true,
        systemMessage:
          `🔴 CONTEXT CRITICAL: Usage at ${usedPct}%. Only ${remaining}% remaining. ` +
          'Auto-compact will trigger very soon. ' +
          'Consider running /compact now to control what gets preserved.'
      };
      process.stdout.write(JSON.stringify(output));
      return;
    }

    process.stdout.write(JSON.stringify({ continue: true }));
  } catch {
    process.stdout.write(JSON.stringify({ continue: true }));
  }
});
