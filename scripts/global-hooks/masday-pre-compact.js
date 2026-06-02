#!/usr/bin/env node
//
// Masday PreCompact Hook
//
// Fires BEFORE Claude Code auto-compacts the context window.
// Uses valid hook output schema: systemMessage (not hookSpecificOutput).
//

const fs = require("fs");
const path = require("path");
const http = require("http");

const LOG_FILE = path.join(process.env.HOME, ".claude", "compact-log.jsonl");

function log(entry) {
  try {
    fs.appendFileSync(LOG_FILE, JSON.stringify(entry) + "\n");
  } catch {}
}

function saveToMasday(summary, content) {
  return new Promise((resolve) => {
    const data = JSON.stringify({
      memory_type: "context-preserve",
      summary,
      content,
      created_by_agent: "compact-hook",
      importance_score: 8.0,
      tags: ["auto-compact", "context-preserve"],
    });

    const req = http.request(
      {
        hostname: "localhost",
        port: 30101,
        path: "/api/memories",
        method: "POST",
        headers: { "Content-Type": "application/json" },
        timeout: 3000,
      },
      (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => resolve(res.statusCode === 200));
      }
    );
    req.on("error", () => resolve(false));
    req.on("timeout", () => { req.destroy(); resolve(false); });
    req.write(data);
    req.end();
  });
}

async function main() {
  const now = new Date().toISOString();

  log({
    event: "PreCompact",
    timestamp: now,
    cwd: process.cwd(),
    pid: process.pid,
  });

  // Try to save to masday memory
  const saved = await saveToMasday(
    `[COMPACT] Context auto-compaction triggered at ${now}`,
    `Auto-compact triggered. Check session history for full context. CWD: ${process.cwd()}`
  );

  if (saved) {
    log({ event: "PreCompactSave", status: "saved", timestamp: now });
  }

  // Valid output schema — systemMessage injects context into Claude
  const output = {
    continue: true,
    systemMessage: [
      "⚠️ CONTEXT COMPACTION IMMINENT ⚠️",
      "",
      "Before compacting, preserve:",
      "1. Current workflow state (workflow_id, task_id, plan_id)",
      "2. Active file edits in progress",
      "3. Uncommitted changes status",
      "4. Key decisions made this session",
      "",
      saved
        ? "✅ Session context auto-saved to masday memory."
        : "⚠️ Could not auto-save to masday memory.",
    ].join("\n"),
  };

  process.stdout.write(JSON.stringify(output));
}

main().catch(() => process.stdout.write(JSON.stringify({ continue: true })));
