#!/usr/bin/env node
//
// Masday PostCompact Hook
//
// Fires AFTER context compaction completes.
// 1. Saves post-compact state to masday memory (always)
// 2. Only injects resume prompt when there are RUNNING tasks
// 3. Passes cleanly when tasks are DONE (no unnecessary blocking)
//

const fs = require("fs");
const path = require("path");
const os = require("os");
const http = require("http");

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

// HTTP GET from masday API
function apiGet(apiPath, timeout = 3000) {
  return new Promise((resolve) => {
    const req = http.request(
      { hostname: "localhost", port: 30101, path: apiPath, method: "GET", timeout },
      (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => {
          try { resolve(JSON.parse(body)); }
          catch { resolve(null); }
        });
      }
    );
    req.on("error", () => resolve(null));
    req.on("timeout", () => { req.destroy(); resolve(null); });
    req.end();
  });
}

// HTTP POST to masday API
function apiPost(apiPath, body, timeout = 3000) {
  return new Promise((resolve) => {
    const data = JSON.stringify(body);
    const req = http.request(
      { hostname: "localhost", port: 30101, path: apiPath, method: "POST",
        headers: { "Content-Type": "application/json" }, timeout },
      (res) => {
        let buf = "";
        res.on("data", (c) => (buf += c));
        res.on("end", () => resolve(res.statusCode >= 200 && res.statusCode < 300));
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
  const cwd = process.cwd();

  // Update compact state
  const state = getState();
  state.count++;
  state.lastCompact = now;
  state.lastCwd = cwd;
  saveState(state);

  // Reset context cache to 0
  try {
    fs.writeFileSync(CONTEXT_CACHE_FILE, JSON.stringify({ pct: 0, ts: Date.now() }));
  } catch {}

  log({ event: "PostCompact", timestamp: now, cwd, totalCompacts: state.count });

  // Step 1: ALWAYS save post-compact state to masday memory
  let savedState = false;
  let activeTasks = [];
  let activeWorkflow = null;

  try {
    const workflows = await apiGet("/api/workflows");
    if (Array.isArray(workflows) && workflows.length > 0) {
      // Find active (non-terminal) workflow
      activeWorkflow = workflows.find(w =>
        w.status && !["DONE", "FAILED"].includes(w.status.toUpperCase())
      ) || null;

      if (activeWorkflow) {
        const tasks = await apiGet(`/api/workflows/${activeWorkflow.id}/tasks`);
        if (Array.isArray(tasks)) {
          activeTasks = tasks.filter(t =>
            t.status && ["RUNNING", "PENDING"].includes(t.status.toUpperCase())
          );
        }
      }
    }

    // Save state to memory (always, regardless of active tasks)
    savedState = await apiPost("/api/memories", {
      memory_type: "context-preserve",
      summary: `[POST-COMPACT #${state.count}] ${activeWorkflow ? `Workflow ${activeWorkflow.id.substring(0, 8)} | ${activeTasks.length} active tasks` : "No active workflow"}`,
      content: JSON.stringify({
        event: "post-compact",
        compact_number: state.count,
        timestamp: now,
        cwd,
        workflow: activeWorkflow ? {
          id: activeWorkflow.id,
          status: activeWorkflow.status,
          name: activeWorkflow.name,
        } : null,
        active_tasks: activeTasks,
      }),
      created_by_agent: "compact-hook",
      importance_score: activeTasks.length > 0 ? 0.9 : 0.5,
      tags: ["auto-compact", "context-preserve", "post-compact"],
    });

    log({ event: "PostCompactSave", saved: savedState, active_tasks: activeTasks.length, timestamp: now });
  } catch (e) {
    log({ event: "PostCompactSave", error: e.message, timestamp: now });
  }

  // Step 2: Load pre-compact git state as backup context
  let gitInfo = "";
  try {
    const gitFile = path.join(process.env.HOME, ".claude", "compact-git-state.json");
    if (fs.existsSync(gitFile)) {
      const gs = JSON.parse(fs.readFileSync(gitFile, "utf8"));
      gitInfo = `Git: ${gs.branch} | ${gs.status.split("\n").length} changes`;
    }
  } catch {}

  // Step 3: Build output based on task state
  const hasRunningTasks = activeTasks.some(t => t.status.toUpperCase() === "RUNNING");
  const hasActiveWorkflow = activeWorkflow !== null;

  const lines = [
    "🔄 CONTEXT COMPACTION COMPLETE",
    "",
    `Compact #${state.count} at ${now}`,
  ];

  if (savedState) {
    lines.push("✅ State saved to masday memory.");
  }

  // Only inject resume instructions when there are RUNNING tasks
  if (hasRunningTasks) {
    lines.push("");
    lines.push("⚠️ ACTIVE RUNNING TASKS DETECTED — resume work:");
    lines.push(`   Workflow: ${activeWorkflow.id.substring(0, 8)} (${activeWorkflow.status})`);
    activeTasks.filter(t => t.status.toUpperCase() === "RUNNING").forEach(t => {
      lines.push(`   ▶ Task: ${t.name || t.id.substring(0, 8)} [RUNNING]`);
    });
    lines.push("");
    lines.push("Resume actions:");
    lines.push("1. Use mcp__masday__workflow_getStatus to check workflow state");
    lines.push("2. Use mcp__masday__workflow_saveProgress to continue where you left off");
    lines.push("3. Re-read files you were editing before compaction");
  } else if (hasActiveWorkflow) {
    // Workflow exists but no RUNNING tasks — tasks may be DONE or PENDING
    lines.push(`   Workflow ${activeWorkflow.id.substring(0, 8)}: ${activeWorkflow.status} (no running tasks)`);
    if (gitInfo) lines.push(`   ${gitInfo}`);
  } else {
    // No active workflow — clean state, no resume needed
    lines.push("   No active workflow — clean state.");
    if (gitInfo) lines.push(`   ${gitInfo}`);
  }

  const output = { continue: true, systemMessage: lines.join("\n") };
  process.stdout.write(JSON.stringify(output));
}

main().catch(() => process.stdout.write(JSON.stringify({ continue: true })));
