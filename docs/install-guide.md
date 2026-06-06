# Masday CLI — Installation Guide

## Quick Install (Linux)

```bash
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash
```

This downloads the binary, verifies the SHA-256 checksum, and installs to `~/.masday/bin/`.

---

## Manual Download

### Linux x86_64

```bash
curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-linux-x86_64
chmod +x masday
sudo mv masday /usr/local/bin/
```

### Windows x86_64

Download from PowerShell or browser:

```powershell
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe" -OutFile "masday.exe"
```

Or download directly: [masday-windows-x86_64.exe](https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-windows-x86_64.exe)

### Specific Version

Replace `latest` with the version tag:

```bash
curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/v0.3.0/masday-linux-x86_64
```

---

## Verify Checksum

```bash
# Download checksums
curl -fsSL -O https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/checksums-sha256.txt

# Verify Linux binary
sha256sum -c --ignore-missing checksums-sha256.txt
```

---

## First Run

```bash
# Check version
masday --version

# Install masday into your project
cd /path/to/your/project
masday install
```

---

## MCP Server Binary

The `masday` binary includes the MCP server — run `masday mcp` to start. A standalone `masday-mcp` binary is also available for lightweight deployments.

### Download MCP Server

#### Linux x86_64

```bash
mkdir -p ~/.masday/bin
curl -fsSL -o ~/.masday/bin/masday-mcp \
  https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-linux-x86_64
chmod +x ~/.masday/bin/masday-mcp
```

#### Windows x86_64

```powershell
mkdir "$env:USERPROFILE\.masday\bin" -Force
Invoke-WebRequest -Uri "https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-windows-x86_64.exe" -OutFile "$env:USERPROFILE\.masday\bin\masday-mcp.exe"
```

#### Specific Version

```bash
# Replace 'latest' with the version tag
curl -fsSL -o masday-mcp https://github.com/dayartcrew-web/masday-workflow-rust/releases/download/v0.3.0/masday-mcp-linux-x86_64
```

### Configure MCP (stdio mode — local)

Point your MCP client to the `masday-mcp` binary. **No environment variables needed** — the binary uses SQLite at `~/.masday/data.db` automatically.

**Claude Code** (`.mcp.json` in project root):
```json
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "/home/user/.masday/bin/masday-mcp"
    }
  }
}
```

**Windows** (`.mcp.json`):
```json
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "C:\\Users\\YourUser\\.masday\\bin\\masday-mcp.exe"
    }
  }
}
```

### Configure MCP (HTTP/SSE mode — no binary needed)

If `masday-api` is running, you can connect directly without the MCP binary. Requires the API server to be running with `DATABASE_URL` and `MASDAY_API_KEY` set.

```json
{
  "mcpServers": {
    "masday": {
      "url": "http://localhost:30101/mcp"
    }
  }
}
```

### Prerequisites for MCP Server

**Local mode (default):**
- **None** — the MCP binary is self-contained and uses SQLite for persistence
- SQLite database is auto-created at `~/.masday/data.db` on first run

**Remote mode:**
- API server (`masday-api`) must be running with PostgreSQL and `MASDAY_API_KEY` set
- MCP clients connect directly via HTTP/SSE — no MCP binary needed on client machines

## What `masday install` Does

The `masday` binary is self-contained (~39MB). All templates are embedded at compile time — no source code, Node.js, or Rust toolchain needed.

`masday install` extracts and configures:

| Component | Destination | Description |
|-----------|-------------|-------------|
| Agent definitions | `~/.claude/agents/*.md` | 28 AI agent profiles |
| Skill definitions | `~/.claude/skills/*/SKILL.md` | 30+ workflow & builder skills |
| Global hooks | `~/.claude/hooks/` | Statusline, session start, context warning, compact, bash guard |
| Project hooks | `.claude/hooks/` | Pre-commit (fmt+lint), pre-push (build+test) |
| MCP config | `.mcp.json` | Claude Code MCP server registration |
| MCP config | `.claude.json` | Alternative Claude Code config |
| MCP config | `.gemini/settings.json` | Gemini CLI config |
| MCP config | `.vscode/mcp.json` | VS Code Copilot config |
| settings.json updates | `.claude/settings.json` | Statusline, autoCompact, hook registrations |

## Install Modes

### Local Mode (default)

```bash
masday install
```

Requires Rust toolchain on the machine. Builds from source, then installs templates.

### Remote Mode

```bash
masday install --remote https://your-server.com:30101 --api-key your-api-key
```

No Rust needed. Connects directly to a remote API server — no MCP binary required on client.

**API server env vars:**

```env
DATABASE_URL="postgresql://USER:PASS@localhost:54341/masday_workflow"
MASDAY_API_KEY="your-api-key"
```

MCP clients connect via HTTP/SSE directly to the API server (see "Configure MCP (HTTP/SSE mode)" above).

## Uninstall

```bash
# Remove from project
masday uninstall

# Remove binary
rm ~/.masday/bin/masday

# Remove global agents/skills/hooks
rm -rf ~/.claude/agents/masday-*.md
rm -rf ~/.claude/skills/masday-*
rm -rf ~/.claude/hooks/masday-*
```

## Update

```bash
# Re-download latest binary
curl -fsSL https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/install.sh | bash

# Or with masday itself
masday update

# Then re-install into project
cd /path/to/your/project
masday install
```

## Troubleshooting

### `command not found: masday`

Add `~/.masday/bin` to your PATH:

```bash
echo 'export PATH="$PATH:$HOME/.masday/bin"' >> ~/.bashrc
source ~/.bashrc
```

### `masday install` fails with permission error

Make sure `~/.claude/` is writable:

```bash
ls -la ~/.claude/
```

### MCP server not connecting

**Check if binary exists:**

```bash
# Linux
ls -la ~/.masday/bin/masday-mcp

# Windows
dir "%USERPROFILE%\.masday\bin\masday-mcp.exe"
```

**If missing, download from releases:**

```bash
# Linux
curl -fsSL -o ~/.masday/bin/masday-mcp \
  https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-mcp-linux-x86_64
chmod +x ~/.masday/bin/masday-mcp
```

**Check `.mcp.json`** in your project root — it should point to the correct binary path:

```json
{
  "mcpServers": {
    "masday": {
      "type": "stdio",
      "command": "/home/user/.masday/bin/masday-mcp"
    }
  }
}
```

**Check SQLite database exists:**

```bash
ls -la ~/.masday/data.db
# If missing, it will be auto-created on first run
```

**Alternatively, use HTTP mode** (requires API server):

```json
{
  "mcpServers": {
    "masday": {
      "url": "http://localhost:30101/mcp"
    }
  }
}
```

---

## Supported Platforms

| Platform | Architecture | CLI Binary | MCP Binary | Status |
|----------|-------------|------------|------------|--------|
| Linux | x86_64 | `masday-linux-x86_64` | `masday-mcp-linux-x86_64` | ✅ Supported |
| Windows | x86_64 | `masday-windows-x86_64.exe` | `masday-mcp-windows-x86_64.exe` | ✅ Supported |
| macOS | x86_64 | — | — | 🔜 Planned |
| macOS | Apple Silicon | — | — | 🔜 Planned |

## Requirements

- **Claude Code**, **Gemini CLI**, **VS Code Copilot**, or **OpenCode** — any MCP-compatible AI client
- **Local mode:** No additional requirements — SQLite is embedded
- **Remote mode:** PostgreSQL 16 with pgvector + running `masday-api` server + `DATABASE_URL`/`MASDAY_API_KEY` env vars on the server
- **Redis 7** (optional, for API server caching in remote mode)

## Embedding Setup

Masday supports semantic search via vector embeddings. Three providers available:

### Local (Recommended — no external service)

```bash
export EMBEDDING_PROVIDER=local
export EMBEDDING_MODEL=all-MiniLM-L6-v2     # 384 dims, fast, ~90MB download
export EMBEDDING_DIMENSIONS=384
```

Model auto-downloads from HuggingFace on first embed. No Ollama or API key needed.

Supported models:

| Model | Dimensions | Size | Best for |
|-------|-----------|------|----------|
| `all-MiniLM-L6-v2` | 384 | ~90MB | Fast, general purpose |
| `bge-small-en-v1.5` | 384 | ~130MB | English text |
| `bge-base-en-v1.5` | 768 | ~430MB | Higher quality |
| `nomic-embed-text-v1.5` | 768 | ~270MB | Code + text |

### Ollama (requires running Ollama)

```bash
export EMBEDDING_PROVIDER=ollama
export EMBEDDING_MODEL=nomic-embed-text
```

### OpenAI (requires API key)

```bash
export EMBEDDING_PROVIDER=openai
export EMBEDDING_API_KEY=sk-...
export EMBEDDING_MODEL=text-embedding-3-small
```

> **Note:** `EMBEDDING_DIMENSIONS` must match both the model output and your pgvector column size.
