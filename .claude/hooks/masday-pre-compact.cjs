#!/usr/bin/env node
//
// Masday PreCompact Hook
//
// Fires BEFORE context compaction.
// Saves real progress to masday API first, then allows compaction.
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

// HTTP helper — POST JSON to masday API
function apiPost(apiPath, body, timeout = 3000) {
  return new Promise((resolve) => {
    const data = JSON.stringify(body);
    const req = http.request(
      { hostname: "localhost", port: 3010, path: apiPath, method: "POST",
        headers: { "Content-Type": "application/json" }, timeout },
      (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => {
          try { resolve({ ok: res.statusCode >= 200 && res.statusCode < 300, data: JSON.parse(body) }); }
          catch { resolve({ ok: false, data: null }); }
        });
      }
    );
    req.on("error", () => resolve({ ok: false, data: null }));
    req.on("timeout", () => { req.destroy(); resolve({ ok: false, data: null }); });
    req.write(data);
    req.end();
  });
}

// HTTP helper — GET from masday API
function apiGet(apiPath, timeout = 3000) {
  return new Promise((resolve) => {
    const req = http.request(
      { hostname: "localhost", port: 3010, path: apiPath, method: "GET", timeout },
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

async function main() {
  const now = new Date().toISOString();
  const cwd = process.cwd();

  log({ event: "PreCompact", timestamp: now, cwd });

  let savedProgress = false;
  let activeTaskInfo = null;

  // Step 1: Fetch active workflows + tasks
  try {
    const workflows = await apiGet("/api/workflows");
    if (Array.isArray(workflows) && workflows.length > 0) {
      // Find the most recent active workflow (not DONE/FAILED)
      const active = workflows.find(w =>
        w.status && !["DONE", "FAILED"].includes(w.status.toUpperCase())
      );

      if (active) {
        // Get tasks for this workflow
        const tasks = await apiGet(`/api/workflows/${active.id}/tasks`);
        const runningTasks = Array.isArray(tasks)
          ? tasks.filter(t => t.status && t.status.toUpperCase() === "RUNNING")
          : [];

        activeTaskInfo = {
          workflow_id: active.id,
          workflow_status: active.status,
          workflow_name: active.name || active.metadata?.description || "",
          running_tasks: runningTasks.map(t => ({
            id: t.id,
            name: t.name,
            status: t.status,
          })),
        };

        // Step 2: Save real progress to masday memory
        const saveResult = await apiPost("/api/memories", {
          memory_type: "context-preserve",
          summary: `[PRE-COMPACT] Workflow ${active.id.substring(0, 8)} | Status: ${active.status} | Running tasks: ${runningTasks.length}`,
          content: JSON.stringify({
            event: "pre-compact",
            timestamp: now,
            cwd,
            workflow: { id: active.id, status: active.status, name: active.name },
            running_tasks: runningTasks,
            plan_id: active.current_plan_id || null,
            task_id: active.current_task_id || null,
          }),
          created_by_agent: "compact-hook",
          importance_score: 0.8,
          tags: ["auto-compact", "context-preserve", "pre-compact"],
        });

        savedProgress = saveResult.ok;
        log({ event: "PreCompactSave", status: savedProgress ? "saved" : "failed", workflow_id: active.id, timestamp: now });
      }
    }
  } catch (e) {
    log({ event: "PreCompactSave", status: "error", error: e.message, timestamp: now });
  }

  // Step 3: Save git status to file-based memory as backup
  try {
    const { execFileSync } = require("child_process");
    const gitStatus = execFileSync("git", ["status", "--short"], { cwd, timeout: 3000 }).toString().trim();
    const gitBranch = execFileSync("git", ["branch", "--show-current"], { cwd, timeout: 3000 }).toString().trim();

    if (gitStatus) {
      fs.writeFileSync(
        path.join(process.env.HOME, ".claude", "compact-git-state.json"),
        JSON.stringify({ branch: gitBranch, status: gitStatus, timestamp: now, cwd })
      );
    }
  } catch {}

  // Build system message
  const lines = [
    "⚠️ CONTEXT COMPACTION IMMINENT ⚠️",
    "",
  ];

  if (savedProgress) {
    lines.push("✅ Session progress auto-saved to masday memory.");
    if (activeTaskInfo) {
      lines.push(`   Workflow: ${activeTaskInfo.workflow_id.substring(0, 8)} (${activeTaskInfo.workflow_status})`);
      lines.push(`   Running tasks: ${activeTaskInfo.running_tasks.length}`);
    }
  } else if (activeTaskInfo) {
    lines.push("⚠️ Could NOT auto-save to masday API — context may be lost!");
    lines.push(`   Active workflow: ${activeTaskInfo.workflow_id}`);
  } else {
    lines.push("ℹ️ No active workflow found — safe to compact.");
  }

  lines.push("", "After compaction, check masday memory to resume work.");

  const output = { continue: true, systemMessage: lines.join("\n") };
  process.stdout.write(JSON.stringify(output));
}

main().catch(() => process.stdout.write(JSON.stringify({ continue: true })));
