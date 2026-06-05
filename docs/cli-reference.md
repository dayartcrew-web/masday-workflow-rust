# Masday CLI — Command Reference

```
masday [OPTIONS] <COMMAND>

COMMANDS:
  install      Install masday into current project
  update       Update masday to latest version
  status       Show system health & diagnostics
  config       View and manage configuration
  embed        Manage embedding models & cache

OPTIONS:
  -v, --verbose    Enable verbose output
  -q, --quiet      Suppress non-error output
  -h, --help       Print help
  -V, --version    Print version
```

---

## `masday install`

Install masday into the current project directory. Syncs agents, skills, hooks, and registers MCP servers.

```bash
masday install [OPTIONS]

OPTIONS:
  --mode <MODE>              Install mode: local | remote | standalone
                             Default: auto-detect (local if Cargo.toml, else standalone)
  --remote <URL>             Remote API server URL (implies --mode remote)
  --api-key <KEY>            API key for remote mode
  --platform <PLATFORM>      Target platform: claude-code | gemini | vscode | opencode
                             Default: auto-detect
  --skip-build               Skip cargo build (use existing binaries)
  --local-only               Skip global directories (~/.claude/)
  --force                    Overwrite existing files without prompting
  --no-hooks                 Skip hook installation
  --no-mcp                   Skip MCP server registration

EXAMPLES:
  masday install                        # Auto-detect mode & platform
  masday install --mode standalone      # Agents + skills only, no build
  masday install --remote https://api.masday.io --api-key <KEY>
  masday install --platform vscode --force
```

---

## `masday update`

Update masday binary, agents, skills, hooks, and MCP configs. Preserves user config.

```bash
masday update [OPTIONS]

OPTIONS:
  --check                    Check for available update without applying
  --version <VERSION>        Update to specific version (default: latest)
  --skip-binary              Only update agents/skills/hooks, not the binary
  --skip-config              Don't overwrite config.toml
  --dry-run                  Show what would be updated without changing anything
  --force                    Force re-install even if already up-to-date

EXAMPLES:
  masday update                         # Update everything to latest
  masday update --check                 # Just check for new version
  masday update --version v0.4.0        # Update to specific release
  masday update --skip-binary           # Refresh assets only
```

**What gets preserved:**
- `~/.masday/config.toml` — merged, your values kept
- `.env` — untouched
- MCP registrations — updated in-place
- Hooks — overwritten (source of truth is binary)

---

## `masday status`

Show system health, connectivity, and configuration diagnostics.

```bash
masday status [OPTIONS]

OPTIONS:
  --json                     Output as JSON (for scripting)
  --verbose                  Show detailed component info

OUTPUT:
  ╭──────────────────────────────────────────╮
  │  Masday v0.3.11                          │
  │                                          │
  │  Mode:       local                       │
  │  Platform:   claude-code                 │
  │  Config:     ~/.masday/config.toml ✓     │
  │                                          │
  │  API:        localhost:3010    ✓ healthy  │
  │  Database:   localhost:5434    ✓ connected│
  │  Redis:      localhost:6379    ✓ connected│
  │  MCP:        registered      ✓ 91 tools  │
  │                                          │
  │  Embedding:  ollama (local)   ✓ ready    │
  │  Model:      nomic-embed-text            │
  │  Cache:      ~/.masday/embed-cache/      │
  │                                          │
  │  Agents:     28 synced                   │
  │  Skills:     32 synced                   │
  │  Hooks:      7 installed                 │
  ╰──────────────────────────────────────────╯

EXIT CODES:
  0    All healthy
  1    Partial degradation (some components unavailable)
  2    Critical failure (core service down)
```

---

## `masday config`

View and manage `~/.masday/config.toml`.

```bash
masday config <SUBCOMMAND>

SUBCOMMANDS:
  show                       Print current config
  get <KEY>                  Get a single config value
  set <KEY> <VALUE>          Set a config value
  edit                       Open config in $EDITOR
  reset                      Reset to defaults (with confirmation)
  path                       Print config file path

EXAMPLES:
  masday config show
  masday config get api_url
  masday config set api_url https://api.masday.io
  masday config set embedding.provider ollama
  masday config set embedding.model nomic-embed-text
  masday config set ports.api_port 4000
  masday config edit
```

**Config keys (dot notation):**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mode` | string | — | `local` / `remote` / `standalone` |
| `api_url` | string | `http://localhost:3010` | API server URL |
| `api_key` | string | — | API key (remote mode) |
| `database_url` | string | — | PostgreSQL connection string |
| `platforms` | list | `[claude-code]` | Registered platforms |
| `ports.api_port` | int | `3010` | API server port |
| `ports.db_port` | int | `5434` | PostgreSQL port |
| `ports.redis_port` | int | `6379` | Redis port |
| `ports.dashboard_port` | int | `3001` | Dashboard port |
| `embedding.provider` | string | — | `ollama` / `openai` / disabled |
| `embedding.model` | string | auto | Model name |
| `embedding.base_url` | string | auto | Override base URL |
| `embedding.api_key` | string | — | API key for OpenAI |
| `embedding.dimensions` | int | `768` | Vector dimensions |

---

## `masday embed`

Manage embedding models, caches, and diagnostics.

```bash
masday embed <SUBCOMMAND>

SUBCOMMANDS:
  status                     Show embedding provider & model info
  download                   Download model cache for offline use
  list                       List available & cached models
  test                       Run embedding test with sample text
  clear                      Clear embedding cache
  settings                   Interactive embedding configuration wizard

OPTIONS (for download):
  --provider <P>             Provider: ollama | openai
  --model <M>                Model name to download
  --force                    Re-download even if cached

EXAMPLES:
  masday embed status
  masday embed download --provider ollama --model nomic-embed-text
  masday embed test "Hello world"
  masday embed list
  masday embed clear
  masday embed settings          # Interactive wizard
```

**`embed status` output:**

```
╭─────────────────────────────────────────╮
│  Embedding Service                      │
│                                         │
│  Provider:    ollama                    │
│  Model:       nomic-embed-text          │
│  Dimensions:  768                       │
│  Base URL:    http://localhost:11434    │
│                                         │
│  Status:      ✓ ready                   │
│  Cache:       ~/.masday/embed-cache/    │
│  Cache size:  234 MB                    │
│  Last test:   0.32s latency             │
╰─────────────────────────────────────────╯
```

**`embed settings` wizard:**

```
? Select embedding provider:
  ❯ Ollama (local, free, offline-capable)
    OpenAI (cloud, requires API key)
    Disabled (no semantic search)

? Enter Ollama base URL: (http://localhost:11434)
? Select model:
  ❯ nomic-embed-text (768d, recommended)
    all-minilm (384d, lightweight)
    mxbai-embed-large (1024d, high quality)

? Test embedding now? (Y/n)
  ✓ Embedding test passed — 0.32s, 768 dimensions

Config saved to ~/.masday/config.toml
```

---

## Other Commands

```bash
masday quickstart              # Interactive first-time setup wizard
masday mcp                     # Run as MCP server (for platform registration)
masday uninstall [--global]    # Remove masday from project/global dirs
```
