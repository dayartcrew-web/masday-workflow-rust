````markdown
# Masday CLI Production Readiness Plan

## Vision

Masday harus terasa seperti:

- Claude Code
- Bun
- Deno
- Vercel CLI

Prinsip utama:

> User tidak perlu mengetahui Rust, Cargo, build process, atau source code repository.

---

# Phase 1 — Stabilize Distribution

## Goal

Menjadikan GitHub Release sebagai satu-satunya sumber distribusi resmi.

### Tasks

- [ ] Audit seluruh command yang masih menjalankan `cargo build`
- [ ] Audit seluruh command yang masih menjalankan `cargo run`
- [ ] Audit seluruh dependency terhadap source repository
- [ ] Pastikan `install`, `quickstart`, dan `update` hanya menggunakan release binary
- [ ] Hapus auto-detection `Cargo.toml`
- [ ] Hapus implicit local build mode

### Success Criteria

```bash
masday install
```

Tidak pernah menjalankan:

```bash
cargo build
```

---

# Phase 2 — Release Artifact Standardization

## Goal

Semua platform memiliki artefak release yang konsisten.

### Supported Targets

- [ ] Linux x86_64
- [ ] Linux ARM64
- [ ] macOS Intel
- [ ] macOS Apple Silicon
- [ ] Windows x86_64

### Artifact Naming

```text
masday-linux-x86_64
masday-linux-aarch64
masday-macos-x86_64
masday-macos-aarch64
masday-windows-x86_64.exe
```

### Release Validation

- [ ] Binary executable
- [ ] Checksum generated
- [ ] Release notes generated
- [ ] Signature verification (optional)

### Success Criteria

Setiap release memiliki artefak lengkap untuk semua platform.

---

# Phase 3 — Installer Experience

## Goal

Instalasi satu command.

### Install Script

Linux/macOS:

```bash
curl -fsSL https://install.masday.ai | sh
```

Windows:

```powershell
irm https://install.masday.ai/windows.ps1 | iex
```

### Installer Responsibilities

- [ ] Detect OS
- [ ] Detect architecture
- [ ] Download latest release
- [ ] Install binary
- [ ] Add PATH automatically
- [ ] Verify checksum
- [ ] Run health check

### Success Criteria

User baru dapat menjalankan:

```bash
masday quickstart
```

kurang dari 2 menit setelah instalasi.

---

# Phase 4 — Quickstart Optimization

## Goal

Zero-config onboarding.

### Quickstart Flow

```text
Start
 ?
Platform Detection
 ?
Mode Selection
 ?
API Setup
 ?
MCP Registration
 ?
Agent Sync
 ?
Skill Sync
 ?
Done
```

### Tasks

- [ ] Auto detect Claude Code
- [ ] Auto detect Gemini CLI
- [ ] Auto detect VSCode
- [ ] Auto detect OpenCode
- [ ] Auto generate MCP config
- [ ] Auto install hooks
- [ ] Auto sync agents
- [ ] Auto sync skills

### Success Criteria

```bash
masday quickstart
```

Berhasil tanpa dokumentasi tambahan.

---

# Phase 5 — Health & Diagnostics

## Goal

Debugging mudah untuk user publik.

### New Commands

```bash
masday doctor
```

```bash
masday doctor --json
```

### Checks

- [ ] CLI version
- [ ] API connectivity
- [ ] MCP registration
- [ ] Agents
- [ ] Skills
- [ ] Hooks
- [ ] Embedding runtime
- [ ] Database connectivity
- [ ] Config validation

### Example Output

```text
? CLI Installed
? API Connected
? MCP Registered
? Agents Synced
? Skills Synced
? Hooks Installed
? Embedding Running
```

### Success Criteria

90% support ticket dapat diselesaikan dengan output doctor.

---

# Phase 6 — Update System

## Goal

Reliable self-update.

### Tasks

- [ ] Semver support
- [ ] Stable channel
- [ ] Beta channel
- [ ] Rollback support
- [ ] Backup existing config
- [ ] Restore after update

### Commands

```bash
masday update
```

```bash
masday update --beta
```

```bash
masday rollback
```

### Success Criteria

Update tanpa kehilangan konfigurasi pengguna.

---

# Phase 7 — Configuration Management

## Goal

Predictable config structure.

### Directory Layout

```text
~/.masday
+-- config/
+-- cache/
+-- logs/
+-- agents/
+-- skills/
+-- hooks/
+-- mcp/
+-- bin/
```

### Tasks

- [ ] Versioned config schema
- [ ] Automatic migration
- [ ] Config backup
- [ ] Config validation

### Success Criteria

Upgrade versi tidak merusak konfigurasi lama.

---

# Phase 8 — Observability

## Goal

Monitoring production behavior.

### Logging

- [ ] Structured logs
- [ ] Debug mode
- [ ] Error tracking
- [ ] Installation logs

### Commands

```bash
masday logs
```

```bash
masday logs --tail
```

### Success Criteria

Semua error penting dapat ditelusuri dari log.

---

# Phase 9 — Security Hardening

## Goal

Secure public distribution.

### Tasks

- [ ] SHA256 checksum
- [ ] Binary verification
- [ ] Release signing
- [ ] Secure token storage
- [ ] Secrets masking
- [ ] HTTPS-only downloads

### Success Criteria

Semua release dapat diverifikasi integritasnya.

---

# Phase 10 — Documentation

## Goal

Production-grade documentation.

### Pages

- [ ] Installation
- [ ] Quickstart
- [ ] MCP Setup
- [ ] Agents
- [ ] Skills
- [ ] Hooks
- [ ] Troubleshooting
- [ ] Doctor Command
- [ ] Upgrade Guide

### Success Criteria

User baru dapat setup tanpa bantuan maintainer.

---

# Release Readiness Checklist

## Distribution

- [ ] GitHub Release is source of truth
- [ ] Multi-platform binaries available
- [ ] Installer available

## CLI

- [ ] No cargo dependency for users
- [ ] Quickstart works
- [ ] Update works
- [ ] Doctor works

## Reliability

- [ ] Rollback works
- [ ] Config migration works
- [ ] Logs available

## Security

- [ ] Checksums generated
- [ ] Binary verified
- [ ] Secrets protected

## Documentation

- [ ] Installation guide complete
- [ ] Troubleshooting guide complete
- [ ] Quickstart documented

---

# Definition of Production Ready

User dapat:

```bash
curl -fsSL https://install.masday.ai | sh
```

kemudian:

```bash
masday quickstart
```

dan memperoleh:

- MCP terdaftar
- Agent tersinkronisasi
- Skill tersinkronisasi
- Hook terpasang
- API terkoneksi

tanpa perlu menginstal Rust, Cargo, atau membangun source code.
````
