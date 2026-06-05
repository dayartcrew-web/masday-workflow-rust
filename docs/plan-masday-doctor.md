# Plan: masday doctor Command

## Status: DRAFT

## Problem

1. `masday status` menunjukkan DB/Redis "not running" padahal jalan — cuma cek Docker container, bukan actual TCP connection
2. Tidak ada command untuk diagnose + auto-fix masalah
3. `masday install` local mode tidak bisa connect ke DB/Redis yang sudah berjalan

## Solution

### 1. Fix `masday status` — Actual Connection Tests

**Current gaps in `status.rs`:**
- `check_database_health()` cuma cek Docker container `masday-postgres`, bukan TCP port
- `check_redis_health()` cuma cek Docker container `masday-redis`, bukan TCP port
- Jika user punya PostgreSQL/Redis native (bukan Docker), status salah "not running"

**Fix:**
```rust
/// Check database via TCP connection (not just Docker)
async fn check_database_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    if config.mode != "local" {
        return not_configured("not in local mode");
    }

    // 1. If database_url configured, try actual connection
    if let Some(ref db_url) = config.database_url {
        return check_postgres_external(db_url, verbose).await;
    }

    // 2. Try TCP connect to configured port
    let port = config.db_port.unwrap_or(5434);
    if tcp_connectable("localhost", port).await {
        return healthy(format!("connected (port {})", port));
    }

    // 3. Check Docker container
    if is_container_running("masday-postgres") {
        return healthy("running (Docker)");
    }

    // 4. Not available
    unhealthy("not running").with_hint("run 'masday db start' or configure database_url")
}

/// Check Redis via TCP ping (not just Docker)
fn check_redis_health(config: &MasdayConfig, verbose: bool) -> ComponentHealth {
    if config.mode != "local" {
        return not_configured("not in local mode");
    }

    let port = config.redis_port.unwrap_or(6379);

    // 1. Try TCP connect + PING command
    if let Ok(stream) = std::net::TcpStream::connect(format!("localhost:{}", port)) {
        // Send Redis PING
        if redis_ping_ok(&stream) {
            return healthy(format!("connected (port {})", port));
        }
    }

    // 2. Check Docker container
    if is_container_running("masday-redis") {
        return healthy("running (Docker)");
    }

    // 3. Not available
    degraded("not running").with_hint("run 'masday db start'")
}
```

### 2. New Command: `masday doctor`

Deep diagnostics + auto-fix. Unlike `masday status` (read-only), `doctor` actively tries to fix issues.

```
masday doctor [--fix] [--verbose]

OPTIONS:
  --fix       Automatically apply fixes (without this flag, just reports)
  --verbose   Show detailed diagnostics
```

**What doctor checks:**

| # | Check | Auto-fix (--fix) |
|---|-------|-------------------|
| 1 | Config file exists & valid | Create default config |
| 2 | Binary in PATH | Add to PATH / copy to ~/.masday/bin |
| 3 | PostgreSQL reachable | Start Docker or show connection string |
| 4 | Redis reachable | Start Docker or show connection string |
| 5 | API server responding | Start API server |
| 6 | Database migrations current | Run `masday db migrate` |
| 7 | MCP registered in platforms | Re-register MCP |
| 8 | Agents/skills synced | Re-sync from binary |
| 9 | Hooks installed | Re-install hooks |
| 10 | Embedding provider test | Test embedding call |
| 11 | Disk space check | Warn if <500MB |
| 12 | Config values consistency | Fix port conflicts, mode mismatches |

**Output example:**
```
╭──────────────────────────────────────────────────╮
│  Masday Doctor v0.3.13                            │
│                                                    │
│  [CHECK] Config file            ✓ found           │
│  [CHECK] Binary in PATH         ✓ /usr/local/bin  │
│  [CHECK] PostgreSQL             ✗ unreachable     │
│  [FIX]   Starting PostgreSQL... ✓ Docker started  │
│  [CHECK] Redis                  ✗ not running     │
│  [FIX]   Starting Redis...      ✓ Docker started  │
│  [CHECK] API server             ✗ not responding  │
│  [FIX]   Starting API server... ✓ port 3010       │
│  [CHECK] DB migrations          ✗ 2 pending       │
│  [FIX]   Running migrations...  ✓ up to date      │
│  [CHECK] MCP registration       ✓ 91 tools        │
│  [CHECK] Agents synced          ✓ 28              │
│  [CHECK] Hooks installed        ✓ 7               │
│  [CHECK] Embedding              ⚠ not configured  │
│  [CHECK] Disk space             ✓ 12GB free       │
│                                                    │
│  Summary: 9 passed, 2 fixed, 1 warning             │
╰──────────────────────────────────────────────────╯

⚠ Embedding not configured — semantic search disabled
  Fix: masday config set embedding.provider ollama
```

### 3. Implementation Plan

**File: `masday-cli/src/commands/doctor.rs`**

```rust
pub struct DoctorCheck {
    name: String,
    status: DoctorStatus,  // Pass, Fail, Warning, Fixed, Skipped
    message: String,
    fix_applied: Option<String>,
}

pub async fn run(fix: bool, verbose: bool) -> Result<()> {
    let mut checks = Vec::new();

    // Run all checks
    checks.push(check_config().await);
    checks.push(check_binary_path().await);
    checks.push(check_database(&config, fix).await);
    checks.push(check_redis(&config, fix));
    checks.push(check_api_server(&config, fix).await);
    checks.push(check_migrations(&config, fix).await);
    checks.push(check_mcp_registration(fix));
    checks.push(check_agents_synced(fix));
    checks.push(check_hooks_installed(fix));
    checks.push(check_embedding(&config));
    checks.push(check_disk_space());
    checks.push(check_config_consistency(&config, fix));

    // Output results
    output_doctor_report(&checks, verbose);
}
```

### 4. Also Fix: `masday status` DB/Redis Detection

Add TCP connection check as fallback when Docker check fails:

```rust
/// Quick TCP connectivity test
async fn tcp_connectable(host: &str, port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("{}:{}", host, port))
        .await
        .is_ok()
}
```

### Execution Order

| # | Task | Priority |
|---|------|----------|
| 1 | Add `tcp_connectable()` to status.rs | P0 |
| 2 | Fix `check_database_health` — TCP fallback | P0 |
| 3 | Fix `check_redis_health` — TCP fallback | P0 |
| 4 | Create `doctor.rs` with 12 checks | P0 |
| 5 | Add `--fix` auto-fix logic | P1 |
| 6 | Register `doctor` command in main.rs | P0 |
| 7 | Add to CLI reference docs | P2 |
