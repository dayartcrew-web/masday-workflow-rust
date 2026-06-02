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

## What `masday install` Does

The `masday` binary is self-contained (~7.6MB). All templates are embedded at compile time — no source code, Node.js, or Rust toolchain needed (remote mode).

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
masday install --remote https://your-server.com:3010 --api-key your-api-key
```

No Rust needed. Downloads MCP server binary and connects to a remote API server for persistence.

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

Check `.mcp.json` in your project root — it should point to the masday binary:

```json
{
  "mcpServers": {
    "masday": {
      "command": "masday-mcp",
      "args": [],
      "cwd": "/path/to/project",
      "env": {
        "DATABASE_URL": "postgresql://..."
      }
    }
  }
}
```

---

## Supported Platforms

| Platform | Architecture | Binary | Status |
|----------|-------------|--------|--------|
| Linux | x86_64 | `masday-linux-x86_64` | ✅ Supported |
| Windows | x86_64 | `masday-windows-x86_64.exe` | ✅ Supported |
| macOS | x86_64 | — | 🔜 Planned |
| macOS | Apple Silicon | — | 🔜 Planned |

## Requirements

- **Claude Code**, **Gemini CLI**, **VS Code Copilot**, or **OpenCode** — any MCP-compatible AI client
- **PostgreSQL 16** with pgvector (for local mode or self-hosted remote)
- **Redis 7** (optional, for caching)

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
