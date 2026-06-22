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
const BYTES_PER_TOKEN = 2.2;           // tool_use/tool_result JSON is token-dense (~2 bytes/token)
const SYSTEM_OVERHEAD_TOKENS = 18000;  // system prompt + CLAUDE.md + rules + tool defs + memory

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
  if (remaining != null && remaining >= 0 && remaining <= 100) {
    const used = Math.max(0, Math.min(100, Math.round(100 - remaining)));
    return { pct: used, remainingPct: remaining, source: "claude-api" };
  }

  // Method 2: Fallback — parse current session's JSONL only
  // Skip if JSONL is stale (>60s old) to avoid showing previous session's usage
  const currentSessionId = data?.session_id;
  if (!currentSessionId) return null;
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

    // Pin to the CURRENT session's JSONL — files are named {session_id}.jsonl.
    // Picking "latest modified" leaks another session's usage into a new session
    // (e.g. a stale high % from a previous session until the new one is written).
    // On a brand-new session the file may not exist yet on the first render: return
    // null (no bar) rather than fall back to a different session — this is the reset.
    const sessionFile = path.join(sessionDir, `${currentSessionId}.jsonl`);
    let latest = null;
    let latestMtime = 0;
    if (fs.existsSync(sessionFile)) {
      latest = sessionFile;
      latestMtime = fs.statSync(sessionFile).mtimeMs;
    } else {
      // cwd-derived dir doesn't have it (cwd slug mismatch / first render): search all
      // project dirs for the current session_id before giving up.
      const dirs = fs.readdirSync(projectsDir).filter(d => {
        try { return fs.statSync(path.join(projectsDir, d)).isDirectory(); } catch { return false; }
      });
      for (const d of dirs) {
        const candidate = path.join(projectsDir, d, `${currentSessionId}.jsonl`);
        if (fs.existsSync(candidate)) {
          latest = candidate;
          latestMtime = fs.statSync(candidate).mtimeMs;
          break;
        }
      }
    }
    if (!latest) return null;  // current session has no JSONL yet → reset (no stale bar)

    // Staleness check: skip JSONL older than 60s (avoids previous session data)
    const fileAge = (Date.now() - latestMtime) / 1000;
    if (fileAge > 60) return null;

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
          const bridge = {
            session_id: session,
            remaining_percentage: ctx.remainingPct,
            used_pct: ctx.pct,
            timestamp: Math.floor(Date.now() / 1000),
            source: ctx.source,
            total_tokens: ctx.totalTokens || null,
            used_tokens: ctx.usedTokens || null,
            messages_tokens: null,
            tools_tokens: null,
            system_tokens: null,
            memory_tokens: null,
            agents_tokens: null,
            skills_tokens: null,
          };
          // Token breakdown from context_window if available
          if (data?.context_window) {
            const cw = data.context_window;
            if (cw.messages_bytes) bridge.messages_tokens = cw.messages_bytes;
            if (cw.tools_bytes) bridge.tools_tokens = cw.tools_bytes;
            if (cw.system_bytes) bridge.system_tokens = cw.system_bytes;
            if (cw.memory_bytes) bridge.memory_tokens = cw.memory_bytes;
            if (cw.agents_bytes) bridge.agents_tokens = cw.agents_bytes;
            if (cw.skills_bytes) bridge.skills_tokens = cw.skills_bytes;
          }
          fs.writeFileSync(bridgePath, JSON.stringify(bridge));
        } catch {}
      }

      // Progress bar (10 segments)
      const barLen = 10;
      const filled = Math.min(barLen, Math.round(ctx.pct / 100 * barLen));
      const bar = "▓".repeat(filled) + "░".repeat(barLen - filled);

      if (ctx.pct >= 75) {
        parts.push(`💀 ${bar} ${ctx.pct}%`);
      } else if (ctx.pct >= 50) {
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

    // Project name + dirty count (exclude build artifacts and generated files)
    try {
      const dirty = execSync(
        `git status --porcelain 2>/dev/null | grep -vE '(^|/)out/|^.. dist/|^.. build/|^.. .next/|^.. target/|^.. node_modules/'`,
        { encoding: "utf-8", cwd }
      ).trim();
      parts.push(`${dirName}${dirty ? `(${dirty.split("\\n").length})` : ""}`);
    } catch {
      parts.push(dirName);
    }

    process.stdout.write(`⚡ Masday | ${parts.join(" | ")}`);
  });
}

main();
