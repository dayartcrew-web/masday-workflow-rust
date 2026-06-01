# Masday CLI Distribution Plan

> Goal: `npx masday install`, `npx masday uninstall`, `npx masday update`
> or: `npm i -g masday` → `masday install / uninstall / update`

---

## Implementation Status: DONE ✅

Implemented in Rust via `masday-cli` crate (not Node.js as originally planned).

| Milestone | Status |
|-----------|--------|
| CLI commands (install/uninstall/update) | ✅ Done — clap-based, 7 commands |
| Template embedding | ✅ Done — `include_dir` crate, 7.6MB binary |
| Multi-platform support | ✅ Done — Claude Code, Gemini CLI, VS Code, OpenCode |
| Local mode (build + install) | ✅ Done — cargo build + template sync |
| Remote mode (--remote URL) | ✅ Done — PATH resolution + connectivity check |
| Security (SSRF, auth) | ✅ Done — URL validation, require --api-key |
| npm/cargo publish | ⬜ TODO — cargo install or GitHub release |
| curl fallback installer | ⬜ TODO — shell script for non-Rust users |
| Binary download from GitHub releases | ⬜ TODO — remote.rs stub exists |

### Distribution Model (Clarified)

**User receives ONLY the `masday` binary.** Everything else is embedded:

```
masday (7.6MB binary)
  ├── compile-time embedded:
  │   ├── 28 agent .md files
  │   ├── 30+ skill directories (SKILL.md + assets)
  │   ├── 6 global hooks (.js)
  │   └── 9 project hooks (.cjs/.js/.sh)
  └── runtime generated:
      ├── MCP configs per platform
      └── settings.json updates
```

**User does NOT receive:**
- Root project source code
- Cargo workspace / Rust source
- pnpm monorepo / TypeScript source
- Dashboard frontend
- Database migrations (remote mode connects to server)

### Remaining TODO

1. **Publish binary**: `cargo install masday` or GitHub Releases with CI
2. **Binary download**: Implement remote.rs download from GitHub releases
3. **curl installer**: Shell script for users without Rust
4. **Version checking**: `masday update --check` against GitHub API

---

## Arsitektur Saat Ini

```
masday-workflow-rebuild/          (root, private:true, pnpm monorepo)
├── apps/agent-runner/            ← @mcp-rebuild/agent-runner (bin: "masday")
│   └── dist/runtime/mcp.js      ← CLI entry point (MCP stdio server)
├── packages/                     ← 12 internal packages (workspace:*)
└── scripts/setup.sh              ← setup + install hooks
```

**Masalah:** 
- Root `package.json` punya `"private": true` → tidak bisa publish
- `agent-runner` bergantung pada 6+ workspace packages → perlu bundled ke single package
- `setup.sh` butuh akses ke `.claude/agents/`, `.claude/skills/`, dll — ini di repo root, bukan di `apps/agent-runner/`

---

## Opsi 1: Publish ke npm Registry (Recommended)

### A. Buat `apps/cli/` package baru (standalone CLI wrapper)

```
apps/cli/
├── package.json          ← name: "masday", public, bin, files
├── src/
│   ├── index.ts          ← CLI entry (commander/inquirer)
│   ├── commands/
│   │   ├── install.ts    ← runs setup.sh equivalent logic
│   │   ├── uninstall.ts  ← cleanup skills/agents/hooks
│   │   └── update.ts     ← git pull + rebuild + re-install
│   └── utils/
├── templates/            ← bundled agents, skills, rules, hooks
│   ├── agents/           ← copied from .claude/agents/
│   ├── skills/           ← copied from .claude/skills/
│   ├── rules/            ← copied from .claude/rules/
│   └── hooks/            ← copied from .claude/hooks/
└── dist/
```

### B. `package.json` untuk `apps/cli/`

```json
{
  "name": "masday",
  "version": "1.0.0",
  "description": "Masday workflow orchestration CLI",
  "type": "module",
  "bin": {
    "masday": "./dist/index.js"
  },
  "files": [
    "dist/",
    "templates/"
  ],
  "scripts": {
    "build": "tsc",
    "prepublishOnly": "npm run build"
  },
  "dependencies": {
    "commander": "^13.0.0",
    "inquirer": "^12.0.0",
    "@mcp-rebuild/agent-runner": "workspace:*"
  }
}
```

### C. Build pipeline (bundle semua internal deps)

Gunakan **esbuild** atau **tsup** untuk bundle `masday` CLI + semua workspace deps ke single file:

```json
{
  "scripts": {
    "build": "tsup src/index.ts --format esm --banner.js '#!/usr/bin/env node'"
  }
}
```

### D. Commands

```bash
# Install (setup everything)
masday install              # full setup: deps, build, agents, skills, hooks
masday install --platform claude    # hanya Claude Code
masday install --platform opencode  # hanya OpenCode
masday install --global             # install ke global dirs

# Uninstall (cleanup)
masday uninstall            # hapus agents, skills, hooks dari semua platform
masday uninstall --global   # hapus dari global dirs

# Update
masday update               # git pull + build + re-install
masday update --check       # cek ada update baru atau tidak

# Run MCP server (backward compat)
masday serve                # alias ke node dist/runtime/mcp.js
```

### E. Publish target

**GitHub Packages** (private repo):
```json
{
  "publishConfig": {
    "registry": "https://npm.pkg.github.com",
    "access": "restricted"
  }
}
```

User setup:
```bash
# ~/.npmrc
@dayartcrew-web:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

Install:
```bash
npm install -g @dayartcrew-web/masday
# atau
npx @dayartcrew-web/masday install
```

---

## Opsi 2: `npx` dari GitHub (tanpa npm registry)

```bash
# Langsung dari repo (tanpa publish)
npx github:dayartcrew-web/masday-workflow-rebuild install
```

**Cara kerja:** npx clone repo, jalankan bin entry.

**Keuntungan:** tidak perlu publish ke registry
**Kekurangan:** lambat (clone tiap kali), tidak ada versioning, tidak reliable

---

## Opsi 3: Install Script via `curl` ( seperti rustup / nvm )

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-rebuild/main/scripts/install.sh | bash

# Uninstall
curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-rebuild/main/scripts/uninstall.sh | bash

# Update
curl -fsSL https://raw.githubusercontent.com/dayartcrew-web/masday-workflow-rebuild/main/scripts/update.sh | bash
```

**Keuntungan:** paling simpel, tidak perlu Node.js pre-installed
**Kekurangan:** tidak idiomatic untuk Node project

---

## Rekomendasi: Hybrid (Opsi 1 + Opsi 3)

| Use Case | Command |
|----------|---------|
| Developer (Node.js) | `npm i -g masday && masday install` |
| CI/CD | `npx masday install --platform claude` |
| Quick setup | `curl -fsSL .../install.sh \| bash` |

### Roadmap Implementasi

1. **Buat `apps/cli/`** — standalone CLI package dengan `commander`
2. **Bundle** — gunakan `tsup` untuk bundle semua deps ke single file
3. **Templates** — copy `.claude/agents/`, `.claude/skills/`, `.claude/rules/` ke `templates/` sebagai bagian dari CLI package
4. **Install command** — rewrite `setup.sh` logic ke TypeScript
5. **Uninstall command** — cleanup `masday-*` dari global dirs + project dirs
6. **Update command** — `git pull` + rebuild + re-install agents/skills
7. **Publish** — GitHub Packages (private) atau npm (public)
8. **curl fallback** — shell script untuk non-Node users

### File Structure Result

```
apps/cli/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              ← CLI entry
│   ├── commands/
│   │   ├── install.ts
│   │   ├── uninstall.ts
│   │   ├── update.ts
│   │   └── serve.ts
│   ├── lib/
│   │   ├── platform.ts       ← detect Claude/Gemini/OpenCode/VSCode
│   │   ├── agent-sync.ts     ← copy agents to platform dirs
│   │   ├── skill-sync.ts     ← copy skills to global dirs
│   │   ├── hook-setup.ts     ← install hooks
│   │   └── mcp-config.ts     ← write MCP configs
│   └── utils/
│       ├── fs.ts
│       └── exec.ts
├── templates/
│   ├── agents/               ← .claude/agents/*.md
│   ├── skills/               ← .claude/skills/masday-*/
│   ├── rules/                ← .claude/rules/*
│   └── hooks/                ← .claude/hooks/*
└── dist/                     ← bundled output
    └── index.js              ← single file CLI
```

### Publish ke GitHub Packages

```bash
# Build
cd apps/cli && pnpm build

# Publish (dari monorepo root)
pnpm --filter masday publish

# Atau manual
cd apps/cli && npm publish --registry=https://npm.pkg.github.com
```

### User Experience

```bash
# Install global
npm install -g @dayartcrew-web/masday

# Setup project
cd my-project
masday install
# → Installing dependencies...
# → Building packages...
# → Installing 36 skills...
# → Converting 27 agents...
# → Setting up hooks...
# → Done!

# Update
masday update

# Uninstall
masday uninstall
```

---

## Perhatian Khusus

### Permission Issues (root-owned dirs)
CLI `install` harus:
- Cek writability sebelum write ke global dirs
- Fallback ke project-local jika global tidak writable
- Tampilkan pesan fix yang jelas: `sudo chown -R $(whoami) ~/.config/opencode/agent/`

### Monorepo Workspace Deps
- `apps/cli/` harus bundle semua internal deps (tidak bisa pakai `workspace:*` di published package)
- Gunakan `tsup` atau `esbuild` untuk inlining
- Alternatif: publish semua internal packages juga (tidak recommended — terlalu banyak)

### .env File
- CLI tidak boleh overwrite `.env` yang sudah ada
- Prompt user untuk missing values pada `masday install`

### Versioning
- Single version di `apps/cli/package.json`
- Sync dengan git tags: `v1.0.0`, `v1.1.0`, etc.
- `masday --version` → baca dari package.json
