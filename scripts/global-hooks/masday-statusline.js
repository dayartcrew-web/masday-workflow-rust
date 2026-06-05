#!/usr/bin/env node
//
// Masday Statusline — status + context progress bar
//
// Output: "⚡ Masday | DB:✓ | API:✓ | MCP:✓ | ▓▓▓░░░░░░░ 32% | ×4 | project-name"
//
// Also writes context metrics to bridge file for the context-monitor PostToolUse hook.
// Bridge file: /tmp/claude-ctx-{session_id}.json
//

const fs = require("fs");
const path = require("path");
const os = require("os");
const net = require("net");
const http = require("http");
const { execSync } = require("child_process");

// Context estimation config
const CONTEXT_WINDOW_TOKENS = 200000;
const BYTES_PER_TOKEN = 2.5;           // tool_use/tool_result JSON is token-dense
const SYSTEM_OVERHEAD_TOKENS = 5000;
const AUTO_COMPACT_BUFFER_PCT = 16.5;  // Claude Code reserves ~16.5% for autocompact

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
 * Read a port value from ~/.masday/config.toml
 */
function readConfigPort(key) {
  try {
    const configPath = path.join(os.homedir(), ".masday", "config.toml");
    if (!fs.existsSync(configPath)) return null;
    const content = fs.readFileSync(configPath, "utf8");
    const match = content.match(new RegExp(key + `\\s*=\\s*(\\d+)`));
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

/**
 * Estimate context from Claude Code's stdin data OR fallback to JSONL parsing.
 * Claude Code provides context_window in statusline stdin data.
 */
function estimateContext(data) {
  // Method 1: Use Claude Code's own context_window data (most accurate)
  const remaining = data?.context_window?.remaining_percentage;
  if (remaining != null) {
    // Normalize: subtract buffer from remaining, scale to usable range
    const usableRemaining = Math.max(0, ((remaining - AUTO_COMPACT_BUFFER_PCT) / (100 - AUTO_COMPACT_BUFFER_PCT)) * 100);
    const used = Math.max(0, Math.min(100, Math.round(100 - usableRemaining)));
    return { pct: used, remainingPct: remaining, source: "claude-api" };
  }

  // Method 2: Fallback — parse session JSONL
  try {
    const claudeDir = process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), ".claude");
    const projectsDir = path.join(claudeDir, "projects");
    if (!fs.existsSync(projectsDir)) return null;

    // Find the project dir matching current cwd
    const cwd = data?.workspace?.current_dir || process.cwd();
    const cwdSlug = cwd.replace(/\//g, "-").replace(/^-/, "");
    let sessionDir = path.join(projectsDir, cwdSlug);

    // Try alternate slug format (underscores)
    if (!fs.existsSync(sessionDir)) {
      const cwdSlugAlt = cwd.replace(/\//g, "-");
      sessionDir = path.join(projectsDir, cwdSlugAlt);
    }
    if (!fs.existsSync(sessionDir)) {
      // Try to find any project dir
      const dirs = fs.readdirSync(projectsDir).filter(d => {
        const full = path.join(projectsDir, d);
        return fs.statSync(full).isDirectory();
      });
      // Use the most recently modified one
      let best = null;
      let bestMtime = 0;
      for (const d of dirs) {
        const full = path.join(projectsDir, d);
        const jsonlFiles = fs.readdirSync(full).filter(f => f.endsWith(".jsonl"));
        for (const f of jsonlFiles) {
          const stat = fs.statSync(path.join(full, f));
          if (stat.mtimeMs > bestMtime) {
            bestMtime = stat.mtimeMs;
            best = full;
          }
        }
      }
      if (best) sessionDir = best;
      else return null;
    }

    const files = fs.readdirSync(sessionDir).filter(f => f.endsWith(".jsonl"));
    if (files.length === 0) return null;

    let latest = null;
    let latestMtime = 0;
    for (const f of files) {
      try {
        const fp = path.join(sessionDir, f);
        const stat = fs.statSync(fp);
        if (stat.mtimeMs > latestMtime) { latestMtime = stat.mtimeMs; latest = fp; }
      } catch {}
    }
    if (!latest) return null;

    const content = fs.readFileSync(latest, "utf8");
    const lines = content.split("\n");

    // Find last compact_boundary — only count active context after it
    let startIdx = 0;
    for (let i = lines.length - 1; i >= 0; i--) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.subtype === "compact_boundary") { startIdx = i + 1; break; }
      } catch {}
    }

    let contentBytes = 0;
    for (let i = startIdx; i < lines.length; i++) {
      if (!lines[i]) continue;
      try {
        const obj = JSON.parse(lines[i]);
        if (obj.type === "user" || obj.type === "assistant") {
          if (obj.message?.content) {
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
    return { pct, remainingPct: 100 - pct, source: "jsonl-estimate" };
  } catch {
    return null;
  }
}

async function main() {
  // Parse stdin data from Claude Code
  let input = '';
  const stdinTimeout = setTimeout(() => process.exit(0), 3000);
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', chunk => input += chunk);
  process.stdin.on('end', async () => {
    clearTimeout(stdinTimeout);

    let data = {};
    try { data = JSON.parse(input); } catch {}

    const model = data.model?.display_name || 'Masday';
    const session = data.session_id || '';
    const cwd = data.workspace?.current_dir || process.cwd();
    const dirName = path.basename(cwd);

    const parts = [];
    const cfgPort = parseInt(process.env.MASDAY_API_PORT || readConfigPort("api_port") || "30101", 10);
    const cfgDbPort = parseInt(process.env.MASDAY_DB_PORT || readConfigPort("db_port") || "54341", 10);

    // Database check
    const dbUp = await isPortOpen(cfgDbPort);
    parts.push(`DB:${dbUp ? "✓" : "✗"}`);

    // API health check
    let apiHealthy = false;
    try {
      apiHealthy = await new Promise((resolve) => {
        const req = http.get(`http://localhost:${cfgPort}/api/health`, { timeout: 1000 }, (res) => {
          resolve(res.statusCode === 200);
        });
        req.on("error", () => resolve(false));
        req.on("timeout", () => { req.destroy(); resolve(false); });
      });
    } catch {}
    parts.push(`API:${apiHealthy ? "✓" : "✗"}`);

    // Context estimation
    const ctx = estimateContext(data);
    if (ctx) {
      // Write bridge file for context-monitor hook
      const sessionSafe = session && !/[/\\]|\.\./.test(session);
      if (sessionSafe) {
        try {
          const bridgePath = path.join(os.tmpdir(), `claude-ctx-${session}.json`);
          fs.writeFileSync(bridgePath, JSON.stringify({
            session_id: session,
            remaining_percentage: ctx.remainingPct,
            used_pct: ctx.pct,
            timestamp: Math.floor(Date.now() / 1000),
            source: ctx.source
          }));
        } catch {}
      }

      // Progress bar (10 segments)
      const barLen = 10;
      const filled = Math.min(barLen, Math.round(ctx.pct / 100 * barLen));
      const bar = "▓".repeat(filled) + "░".repeat(barLen - filled);

      if (ctx.pct >= 80) {
        parts.push(`💀 ${bar} ${ctx.pct}%`);
      } else if (ctx.pct >= 65) {
        parts.push(`🟡 ${bar} ${ctx.pct}%`);
      } else {
        parts.push(`🟢 ${bar} ${ctx.pct}%`);
      }
    }

    // Active workflows (filtered by current project)
    try {
      const wfRes = await new Promise((resolve) => {
        const projectPath = encodeURIComponent(cwd);
        const req = http.get(`http://localhost:${cfgPort}/api/workflows?project_path=${projectPath}`, { timeout: 1500 }, (res) => {
          let body = "";
          res.on("data", (c) => (body += c));
          res.on("end", () => { try { resolve(JSON.parse(body)); } catch { resolve(null); } });
        });
        req.on("error", () => resolve(null));
        req.on("timeout", () => { req.destroy(); resolve(null); });
      });
      if (Array.isArray(wfRes)) {
        const active = wfRes.filter(w => ["EXECUTE", "ANALYZE", "PLAN", "INIT"].includes(w.status)).length;
        const stuck = wfRes.filter(w => ["FAILED", "PAUSED", "FIX"].includes(w.status)).length;
        const wfParts = [];
        if (active > 0) wfParts.push(`▶${active}`);
        if (stuck > 0) wfParts.push(`⛔${stuck}`);
        if (wfParts.length > 0) parts.push(wfParts.join(" "));
      }
    } catch {}

    // Project name + dirty count
    try {
      const dirty = execSync(`git status --porcelain 2>/dev/null`, { encoding: "utf-8", cwd }).trim();
      parts.push(`${dirName}${dirty ? `(${dirty.split("\n").length})` : ""}`);
    } catch {
      parts.push(dirName);
    }

    process.stdout.write(`⚡ Masday | ${parts.join(" | ")}`);
  });
}

main();
