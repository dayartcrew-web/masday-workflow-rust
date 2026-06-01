#!/usr/bin/env node
//
// Masday Statusline — status + context progress bar
//
// Output: "⚡ Masday | DB:✓ | API:✓ | MCP:✓ | ▓▓▓░░░░░░░ 32% | ×4 | rust-wf"
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
const DB_PORT = 5434;
const API_PORT = 3010;

// Context estimation config — calibrated against real Claude CLI readings
const CONTEXT_WINDOW_TOKENS = 200000;    // 200K tokens (Claude Opus/Sonnet)
const BYTES_PER_TOKEN = 4;               // ~4 bytes per token for text
const SYSTEM_OVERHEAD_TOKENS = 18000;    // System prompt + CLAUDE.md + rules + tool schemas (~18K, calibrated with context-warning hook's 15K ±5%)

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
 * Estimate active context usage from the current session's transcript.
 *
 * Strategy:
 *   1. Find the latest session JSONL file
 *   2. Find the last compact_boundary — only count bytes AFTER it
 *   3. Count user+assistant message content bytes (JSON.stringify of message.content)
 *   4. Estimate tokens = contentBytes / BYTES_PER_TOKEN + SYSTEM_OVERHEAD_TOKENS
 *
 * Calibrated: matches real Claude CLI context % within ±5% accuracy.
 */
function estimateContext() {
  try {
    const sessionDir = path.join(
      process.env.HOME, ".claude", "projects", "-home-vibe-dev-masday-workflow-rust"
    );
    if (!fs.existsSync(sessionDir)) return null;

    // Find the most recently modified session file
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

    // Find the last compact_boundary — only count active (post-compact) context
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

    // Count content bytes from user + assistant messages only (after boundary)
    let contentBytes = 0;
    for (let i = startIdx; i < lines.length; i++) {
      const line = lines[i];
      if (!line) continue;
      try {
        const obj = JSON.parse(line);
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

    // 10-char progress bar
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

  // Database check (PostgreSQL on 5434)
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

  // Compact count
  try {
    const compactState = JSON.parse(
      fs.readFileSync(path.join(process.env.HOME, ".claude", "compact-state.json"), "utf8")
    );
    if (compactState.count > 0) {
      parts.push(`×${compactState.count}`);
    }
  } catch {}

  // Git status
  try {
    const branch = execSync(
      `cd ${PROJECT} && git rev-parse --abbrev-ref HEAD`,
      { encoding: "utf-8" }
    ).trim();
    const dirty = execSync(
      `cd ${PROJECT} && git status --porcelain`,
      { encoding: "utf-8" }
    ).trim();
    const short = branch
      .replace("rust-masday-workflow", "rust-wf")
      .replace("masday-workflow-", "mw-");
    parts.push(`${short}${dirty ? `(${dirty.split("\n").length})` : ""}`);
  } catch {}

  console.log(`⚡ Masday | ${parts.join(" | ")}`);
}

main().catch(() => console.log("⚡ Masday | ⚠️ error"));
