# Analisis Komprehensif: Performance, Security & Gaps

**Tanggal:** 2026-05-27
**Branch:** master (local)
**Remote Branch:** masday-workflow-remote (0 commits ahead of master, 18 commits behind)
**Status:** Analisis selesai

---

## Executive Summary

10 masalah performance, 12 kerentanan keamanan, dan 9 gap fungsional ditemukan.
**3 kerentanan CRITICAL** memerlukan perbaikan segera: command injection di shell tools, path traversal di filesystem tools, dan Prisma API yang masih dipanggil padahal sudah migrasi ke Drizzle.

---

## A. PERFORMANCE ISSUES (10)

### P1. JsonBackend — Write-on-Every-Insert [HIGH]
**File:** `packages/store/src/json-backend.ts:72,85,101`
**Masalah:** Setiap INSERT/UPDATE/DELETE memanggil `save()` yang melakukan `JSON.stringify` seluruh data + `fs.writeFileSync`. Synchronous dan blocking.
```typescript
this.data[parsed.table][pk] = parsed.values;
this.save(); // full serialize + write on EVERY operation
```
**Saran:** Tambah write buffering (save setiap N detik atau N operasi) atau migrasi full ke SQLite/PostgreSQL.

### P2. MemoryStore — Tidak Ada Eviction di `add()` [MEDIUM]
**File:** `packages/memory/src/store.ts:127`
**Masalah:** `add()` hanya `this.memories.set()` tanpa cek `maxMemories`. Method `prune()` ada tapi tidak pernah dipanggil otomatis.
**Saran:** Panggil `this.prune()` di akhir `add()` atau implementasi LRU eviction.

### P3. DualWriteStore — Unbounded Promise Chain [HIGH]
**File:** `packages/store/src/dual-write-store.ts:34`
```typescript
this.pendingReplication = this.pendingReplication.then(() =>
  this.replicateWorkflow(workflow)
).catch(() => {});
```
**Masalah:** Setiap write menambah ke promise chain. Jika DB lambat/down, chain tumbuh tak terbatas. `.catch(() => {})` menelan semua error.
**Saran:** Tambah concurrency limit atau queue dengan max depth. Log warning saat queue > threshold.

### P4. GraphStore — O(n²) Auto-Linking [MEDIUM]
**File:** `packages/memory/src/graph.ts:398-426`
**Masalah:** `autoLink()` membandingkan setiap node baru dengan SEMUA node yang ada. Dengan 1000 nodes, 1 insert = 1000 perbandingan + potential edge additions.
**Saran:** Limit ke batch terbaru (e.g., last 100 nodes) atau pindah ke PostgreSQL recursive CTE.

### P5. Context Pack — Tanpa Size Limit [MEDIUM]
**File:** `packages/intelligence/src/context.ts`
**Masalah:** `buildHybridContextPack()` mengumpulkan memories + docs + chunks tanpa batas ukuran total.
**Saran:** Tambah `maxContextTokens` parameter. Truncate/summarize jika melebihi limit.

### P6. EmbeddingService — FIFO Cache [LOW]
**File:** `packages/memory/src/embedding.ts:50-56`
**Masalah:** Cache 1000 entries dengan FIFO eviction. Cache hit rate rendah untuk workload besar.
**Saran:** Ganti ke LRU cache (`lru-cache` npm package).

### P7. FastEmbed Model Per-Process [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:174-185`
**Masalah:** Setiap MCP process load model FastEmbed (~219 MB). 3 process = ~650 MB hanya untuk embedding.
**Saran:** Konsolidasi ke 1 MCP process atau gunakan API-based embedding.

### P8. MCP Zombie Processes [HIGH] (Infra)
**Masalah:** PID 4065212 memakai 1.1 GB RAM, uptime 2+ hari.
**Saran:** Kill zombie. Tambah lifecycle management di Claude settings.

### P9. PostgreSQL Dead Tuples [MEDIUM] (Infra)
**Masalah:** 7 tabel punya dead tuples, belum pernah VACUUM. Memory table 70% waste.
**Saran:** `VACUUM ANALYZE;` + aktifkan autovacuum.

### P10. EpisodicMemory — Tidak Persist Tanpa DB [LOW]
**File:** `packages/memory/src/episodic.ts:98-111`
**Masalah:** `persistToPrisma()` hanya jalan jika `prismaClient` diset. Messages hilang saat restart jika DB unavailable.
**Saran:** Pastikan `setEpisodicDb()` selalu dipanggil, atau tambah file-based fallback.

---

## B. SECURITY VULNERABILITIES (12)

### S1. Command Injection di `git.commit` [CRITICAL]
**File:** `apps/agent-runner/src/runtime/mcp.ts:850`
```typescript
execSync(`git commit -m "${message.replace(/"/g, '\\"')}"`, ...)
```
**Masalah:** Hanya escape double quotes. Backticks `$()`, `$(...)`, dan newline `\n` tetap bisa inject shell commands.
**Contoh attack:** `message = "test$(rm -rf /)"` → dieksekusi.
**Fix:** Gunakan `execFileSync` atau `spawn` dengan args array.

### S2. Command Injection di `cicd.pipeline_trigger` [CRITICAL]
**File:** `apps/agent-runner/src/runtime/mcp.ts:886`
```typescript
execSync(`gh workflow run ${pipeline}`, ...)
```
**Masalah:** `pipeline` adalah user input yang langsung disisipkan ke shell command tanpa sanitasi.
**Contoh attack:** `pipeline = "test; rm -rf /"`
**Fix:** Validasi dengan regex `^[a-zA-Z0-9_-]+$` sebelum execute.

### S3. Command Injection di `npm.run` [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:861`
```typescript
execSync(`pnpm run ${script}`, ...)
```
**Masalah:** `script` adalah user input. Bisa inject arbitrary commands.
**Fix:** Whitelist allowed script names atau gunakan `execFileSync`.

### S4. Command Injection di `docker.build` dan `docker.run` [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:867-872`
```typescript
execSync(`docker build -t ${tag} .`, ...)
execSync(`docker run --rm ${image}`, ...)
```
**Masalah:** `tag` dan `image` user-controlled, tidak disanitasi.
**Fix:** Validasi format (alphanumeric + dots + colons + slashes only).

### S5. Command Injection di `github.pr_create` [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:896`
```typescript
execSync(`gh pr create --title "${title.replace(/"/g, '\\"')}" ...`)
```
**Masalah:** Sama seperti S1 — hanya escape double quotes, backtick injection tetap bisa.
**Fix:** Gunakan `execFileSync` atau spawn dengan args array.

### S6. Path Traversal di `filesystem.*` [CRITICAL]
**File:** `apps/agent-runner/src/runtime/mcp.ts:721-725`
```typescript
// filesystem.read — no path restriction
fs.readFileSync(fp, "utf-8")
// filesystem.write — write to ANY path
fs.writeFileSync(fp, content)
// filesystem.delete — delete ANY file
fs.unlinkSync(fp)
```
**Masalah:** Tools filesystem menerima path absolut tanpa batasan. File sensitif (`/etc/passwd`, `~/.ssh/id_rsa`, `.env`) bisa dibaca/ditulis/dihapus.
**Fix:** Batasi ke project root. Reject paths yang mengandung `..` atau di luar allowed directories.

### S7. `safePath()` Lemah [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:132-138`
```typescript
if (!resolved.includes(".masday") && !resolved.includes(".claude")
    && !fs.existsSync(path.join(resolved, "package.json"))) return fallback;
```
**Masalah:** Cek `includes(".masday")` bisa di-bypass: `/etc/.masday/evil` lolos validasi.
**Fix:** Gunakan `resolved.startsWith(allowedRoot)` untuk membatasi ke project directory.

### S8. Raw SQL dengan User Data [MEDIUM]
**File:** `apps/agent-runner/src/runtime/mcp.ts:282-283,440-448`
```typescript
await drizzleDb.execute(sql`UPDATE "Memory" SET embedding = ${vecStr}::vector WHERE id = ${rec.id}`);
```
**Masalah:** Meskipun drizzle `sql` tag parameterizes, konstruksi `vecStr` dari user content bisa menyebabkan issue jika format tidak sesuai.
**Fix:** Pastikan validasi format vector string sebelum execute.

### S9. No Authentication pada MCP Server [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:101`
**Masalah:** MCP server via stdio tidak punya authentication. Siapapun yang bisa connect ke stdio punya akses penuh: filesystem, shell commands, database.
**Fix:** Untuk production, tambah token-based auth atau restrict stdio access.

### S10. Error Swallowing di DualWriteStore [MEDIUM]
**File:** `packages/store/src/dual-write-store.ts:34`
```typescript
.catch(() => {});  // silently swallows ALL errors
```
**Masalah:** Error DB ditelan tanpa logging. Data bisa hilang tanpa diketahui.
**Fix:** Log error minimal: `.catch(err => logger.warn(...))`.

### S11. `any` Type Bypass (Code Quality) [LOW]
**Files:** `mcp.ts:108-109`, `dual-write-store.ts:8-10`, `graph.ts:10`, `episodic.ts:6`
**Masalah:** Multiple `eslint-disable no-explicit-any` bypass type safety. Bisa menyebabkan runtime errors.
**Fix:** Definisikan proper types untuk Drizzle client interface.

### S12. Prisma Variable Names yang Misleading [LOW]
**Files:** `packages/memory/src/graph.ts:10`, `packages/memory/src/episodic.ts:6`
**Masalah:** Variabel masih bernama `prismaClient` padahal sudah migrasi ke Drizzle. Menyembunyikan potensi bug.
**Fix:** Rename ke `drizzleDb` atau `dbClient`.

---

## C. FUNCTIONAL GAPS (9)

### G1. GraphStore Persist Pakai Prisma API — TIDAK BERFUNGSI [CRITICAL]
**File:** `packages/memory/src/graph.ts:368-395`
```typescript
prismaClient.graphNode.create({ data: { ... } })
prismaClient.graphEdge.create({ data: { ... } })
```
**Masalah:** Kode memanggil `prismaClient.graphNode.create()` tapi client yang di-set via `setGraphDb()` adalah Drizzle. **Graph persistence ke PostgreSQL TIDAK BERFUNGSI.**
**Fix:** Ganti ke Drizzle insert: `drizzleDb.insert(graphNodesTable).values({ ... })`.

### G2. EpisodicMemory Persist Pakai Prisma API — TIDAK BERFUNGSI [CRITICAL]
**File:** `packages/memory/src/episodic.ts:98-111`
```typescript
prismaClient.episodicMemory.create({ data: { ... } })
```
**Masalah:** Sama seperti G1. **Episodic memory persistence ke PostgreSQL TIDAK BERFUNGSI.**
**Fix:** Ganti ke Drizzle insert.

### G3. `semantic-search.code_search` Adalah Stub Kosong [HIGH]
**File:** `apps/agent-runner/src/runtime/mcp.ts:542-544`
```typescript
server.registerTool("semantic-search.code_search", ..., async ({ query }) => {
  return ok({ query, results: [] }); // ALWAYS returns empty
});
```
**Masalah:** Code search selalu return empty array. Tool tidak berguna.
**Fix:** Implementasi real code indexing atau hubungkan ke file-based search.

### G4. `workflow.set_execution_mode` Adalah No-Op [MEDIUM]
**File:** `apps/agent-runner/src/runtime/mcp.ts:346`
```typescript
async ({ session_key, mode }) => ok({ sessionKey: session_key, mode })
```
**Masalah:** Tidak benar-benar mengubah execution mode. Hanya echo input.
**Fix:** Persist ke SessionState table.

### G5. `workflow.mark_synthesis_ready` dan `mark_verification_ready` Adalah No-Op [MEDIUM]
**File:** `apps/agent-runner/src/runtime/mcp.ts:347-349`
**Masalah:** Hanya echo input, tidak update SessionState.
**Fix:** Persist flags ke SessionState table.

### G6. `policy.detect_scope_drift` Trivial [MEDIUM]
**File:** `apps/agent-runner/src/runtime/mcp.ts:613-614`
```typescript
const suspiciousKeywords = ["unrelated", "off-topic", "completely different"];
const driftDetected = suspiciousKeywords.some(k => a.outputText.toLowerCase().includes(k));
```
**Masalah:** Hanya cek 3 keyword literal. Bukan drift detection yang sesungguhnya.
**Fix:** Implementasi real drift detection berdasarkan task acceptance criteria vs output.

### G7. Remote Branch Stale [INFO]
**Masalah:** `masday-workflow-remote` branch 18 commits behind master, 0 ahead. Branch ini sudah tidak relevan dan bisa di-delete.

### G8. mcp.ts File Size — 1031 Lines [MEDIUM]
**Masalah:** Seluruh MCP server (88 tools) ada di 1 file 1031 lines. Sulit di-maintain, review, dan test.
**Saran:** Pecah per namespace: `workflow-tools.ts`, `memory-tools.ts`, `shell-tools.ts`, dll.

### G9. Uncommitted Changes di Master [INFO]
**Masalah:** 18 files modified (uncommitted) + 3 untracked directories di master.
**File penting:** `mcp.ts`, `Dockerfile`, `pnpm-lock.yaml`, `scripts/regenerate-fingerprints.ts`

---

## D. Prioritas Perbaikan

| # | Aksi | Severity | Impact | Effort |
|---|------|----------|--------|--------|
| 1 | Fix command injection di shell tools (S1-S5) | CRITICAL | Security | Kecil |
| 2 | Fix path traversal di filesystem tools (S6) | CRITICAL | Security | Kecil |
| 3 | Fix GraphStore/Episodic Prisma→Drizzle (G1-G2) | CRITICAL | Functionality | Kecil |
| 4 | Kill MCP zombie (P8) | HIGH | -1.1 GB RAM | Sekali klik |
| 5 | VACUUM ANALYZE PostgreSQL (P9) | MEDIUM | Bersihkan bloat | Sekali jalan |
| 6 | Fix safePath validation (S7) | HIGH | Security | Kecil |
| 7 | DualWriteStore queue limit (P3) | HIGH | Stability | Kecil |
| 8 | Add error logging to DualWriteStore catch (S10) | MEDIUM | Observability | Trivial |
| 9 | Konsolidasi MCP jadi 1 process (P7) | HIGH | -650 MB RAM | Config |
| 10 | MemoryStore eviction di add() (P2) | MEDIUM | Memory safety | Kecil |
| 11 | Implement real code_search (G3) | HIGH | Functionality | Medium |
| 12 | Context pack size limit (P5) | MEDIUM | Stability | Medium |
| 13 | Fix no-op tools (G4-G6) | MEDIUM | Functionality | Medium |
| 14 | Split mcp.ts per namespace (G8) | MEDIUM | Maintainability | Medium |

---

## E. Quick Wins (< 30 menit total)

1. **Kill zombie:** `kill 4065212` → hemat 1.1 GB
2. **VACUUM:** `psql -c "VACUUM ANALYZE;"`
3. **Error logging:** Ganti `.catch(() => {})` → `.catch(err => logger.warn(...))`
4. **Input validation:** Tambah regex `^[a-zA-Z0-9_.\\s-]+$` ke shell tool inputs
5. **Path restriction:** Ganti `safePath` cek ke `resolved.startsWith(projectRoot)`
6. **Rename prismaClient:** Batch rename ke `drizzleDb` di graph.ts dan episodic.ts

---

*Dokumen ini dihasilkan dari analisis manual seluruh source code pada branch master lokal, dikonsolidasi dengan temuan dari analisis sebelumnya di branch masday-workflow-remote.*
