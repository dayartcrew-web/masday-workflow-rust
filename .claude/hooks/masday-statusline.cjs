#!/usr/bin/env node
//
// Masday Statusline — status + context progress bar
//
// Output: "⚡ Masday | DB:✓ | API:✓ | MCP:✓ | ▓▓▓░░░░░░░ 32% | ×4 | folder-project"
//
// Context estimation: parses session JSONL, finds last compact_boundary,
// counts user+assistant content bytes after boundary, adds system overhead.
// Calibrated against real Claude CLI context percentage.
//

const { execSync } = require("child_process");
const net = require("net");
const http = require("http");
const path = require("path");
const fs = require("fs");

const PROJECT = "/home/vibe-dev/masday-workflow-rust";
const DB_PORT = 54341;
const API_PORT = 30101;

// Context estimation config — calibrated against real Claude CLI readings
const CONTEXT_WINDOW_TOKENS = 200000;    // 200K tokens (Claude Opus/Sonnet)
const BYTES_PER_TOKEN = 2;               // ~2 bytes/token for tool_use/tool_result JSON (token-dense)
const SYSTEM_OVERHEAD_TOKENS = 5000;     // Base system prompt overhead

function isPortOpen(port) {
  return new Promise((resolve) => {
    const sock = new net.Socket();
    sock.setTimeout(500);
    sock.on("connect", () => { sock.destroy(); resolve(true); });
    sock.on("error", () => resolve(false));
    sock.on("timeout", () => { sock.destroy(); resolve(false); });
    sock.connect(port, "localhost");
  });
}

/**
 * Estimate active context usage from the current session's JSONL transcript.
 *
 * Strategy:
 *   1. Find the latest session JSONL file
 *   2. Find the last compact_boundary — only count bytes AFTER it
 *   3. Count message.content bytes from user + assistant messages
 *   4. tokens = contentBytes / 2 + 10K overhead
 *
 * Why BYTES_PER_TOKEN = 2: tool_use and tool_result JSON blocks are very token-dense
 * (each JSON string represents many actual tokens). Calibrated against real Claude CLI:
 * 357KB content → 180K tokens = 90% real (±4% accuracy).
 *
 * The 10K overhead covers system prompt + tool schemas injected by Claude Code.
 */
function estimateContext() {
  try {
    const sessionDir = path.join(
      process.env.HOME, ".claude", "projects", "-home-vibe-dev-masday-workflow-rust"
    );
    if (!fs.existsSync(sessionDir)) return null;

    const files = fs.readdirSync(sessionDir).filter(f => f.endsWith(".jsonl"));
    if (files.length === 0) return null;

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
    if (!latest) return null;

    const content = fs.readFileSync(latest, "utf8");
    const lines = content.split("\n");

    // Find last compact_boundary — only count active context
    let startIdx = 0;
    for (let i = lines.length - 1; i >= 0; i--) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.subtype === "compact_boundary") {
          startIdx = i + 1;
          break;
        }
      } catch {}
    }

    // Count message content bytes (user + assistant only, after boundary)
    let contentBytes = 0;
    for (let i = startIdx; i < lines.length; i++) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.type === "user" || obj.type === "assistant") {
          if (obj.message && obj.message.content) {
            const c = obj.message.content;
            contentBytes += typeof c === "string" ? c.length : JSON.stringify(c).length;
          } else if (obj.content) {
            contentBytes += typeof obj.content === "string" ? obj.content.length : JSON.stringify(obj.content).length;
          }
        }
      } catch {}
    }

    const tokens = Math.floor(contentBytes / BYTES_PER_TOKEN) + SYSTEM_OVERHEAD_TOKENS;
    const pct = Math.min(100, Math.max(0, Math.round((tokens / CONTEXT_WINDOW_TOKENS) * 100)));

    const barLen = 10;
    const filled = Math.min(barLen, Math.round(pct / 100 * barLen));
    const bar = "▓".repeat(filled) + "░".repeat(barLen - filled);

    return { pct, bar, tokens };
  } catch {
    return null;
  }
}

async function main() {
  const parts = [];

  // Database check (PostgreSQL on 54341)
  const dbUp = await isPortOpen(DB_PORT);
  parts.push(`DB:${dbUp ? "✓" : "✗"}`);

  // API health check — verify /api/health responds
  let apiHealthy = false;
  try {
    apiHealthy = await new Promise((resolve) => {
      const req = http.get(`http://localhost:${API_PORT}/api/health`, { timeout: 1000 }, (res) => {
        resolve(res.statusCode === 200);
      });
      req.on("error", () => resolve(false));
      req.on("timeout", () => { req.destroy(); resolve(false); });
    });
  } catch {}
  if (apiHealthy) {
    parts.push("API:✓");
  } else {
    const apiPort = await isPortOpen(API_PORT);
    parts.push(apiPort ? "API:⚠" : "API:✗");
  }

  // MCP process check — is the binary running?
  let mcpRunning = false;
  try {
    const result = execSync("pgrep -f masday-mcp 2>/dev/null || true", { encoding: "utf-8" }).trim();
    mcpRunning = result.length > 0;
  } catch {}
  if (mcpRunning) {
    parts.push("MCP:✓");
  } else {
    const mcpBin = `${PROJECT}/target/release/masday-mcp`;
    const mcpExists = fs.existsSync(mcpBin);
    parts.push(mcpExists ? "MCP:⚠" : "MCP:✗");
  }

  // Context progress bar
  const ctx = estimateContext();
  if (ctx) {
    if (ctx.pct >= 75) {
      parts.push(`🔴 ${ctx.bar} ${ctx.pct}%`);
    } else if (ctx.pct >= 50) {
      parts.push(`🟡 ${ctx.bar} ${ctx.pct}%`);
    } else {
      parts.push(`🟢 ${ctx.bar} ${ctx.pct}%`);
    }
  }

  // Workflow status — show active/stuck if any
  try {
    const wfRes = await new Promise((resolve) => {
      const http = require("http");
      const req = http.get(`http://localhost:${API_PORT}/api/workflows`, { timeout: 1500 }, (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => {
          try { resolve(JSON.parse(body)); } catch { resolve(null); }
        });
      });
      req.on("error", () => resolve(null));
      req.on("timeout", () => { req.destroy(); resolve(null); });
    });
    if (Array.isArray(wfRes)) {
      const activeStatuses = ["EXECUTE", "ANALYZE", "PLAN", "INIT"];
      const stuckStatuses = ["FAILED", "PAUSED", "FIX"];
      const projectWfs = wfRes.filter(w => (w.project_path || "").includes("masday-workflow"));
      const active = projectWfs.filter(w => activeStatuses.includes(w.status)).length;
      const stuck = projectWfs.filter(w => stuckStatuses.includes(w.status)).length;
      const wfParts = [];
      if (active > 0) wfParts.push(`▶ ${active}`);
      if (stuck > 0) wfParts.push(`⛔ ${stuck}`);
      if (wfParts.length > 0) parts.push(wfParts.join("|"));
    }
  } catch {}

  // Project folder name + dirty count
  try {
    const dirName = path.basename(PROJECT);
    const dirty = execSync(
      `cd ${PROJECT} && git status --porcelain`,
      { encoding: "utf-8" }
    ).trim();
    parts.push(`${dirName}${dirty ? `(${dirty.split("\n").length})` : ""}`);
  } catch {}

  console.log(`⚡ Masday | ${parts.join(" | ")}`);
}

main().catch(() => console.log("⚡ Masday | ⚠️ error"));
