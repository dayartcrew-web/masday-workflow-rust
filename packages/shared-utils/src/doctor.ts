import { execSync } from "child_process";
import { existsSync, readdirSync, rmSync, readFileSync, renameSync } from "fs";
import { join } from "path";
import { logger } from "./logger.js";

export interface DoctorDiagnosis {
  check: string;
  status: "pass" | "fail" | "fixed";
  message: string;
  autoFixed?: boolean;
}

export interface DoctorReport {
  timestamp: string;
  diagnoses: DoctorDiagnosis[];
  fixedCount: number;
  failCount: number;
  allPassed: boolean;
}

/**
 * Auto-diagnose and fix common MCP server startup issues.
 *
 * Fixes:
 * 1. EventEmitter max listeners exhaustion (exit handler leak)
 * 2. Stale PostgreSQL connections (EMAXCONNSESSION)
 * 3. Stale SQLite lock files (.db-journal, .db-wal)
 * 4. Corrupt JSON state cache
 */
export function runDoctor(projectRoot?: string): DoctorReport {
  const diagnoses: DoctorDiagnosis[] = [];
  const root = projectRoot ?? process.cwd();

  // Fix 1: EventEmitter max listeners
  diagnoses.push(fixEventEmitterLeak());

  // Fix 2: Stale PostgreSQL connections
  diagnoses.push(fixStalePgConnections());

  // Fix 3: Stale SQLite lock files
  if (projectRoot) {
    diagnoses.push(fixStaleLockFiles(root));
  }

  // Fix 4: Corrupt JSON state cache
  if (projectRoot) {
    diagnoses.push(fixCorruptJsonCache(root));
  }

  const fixedCount = diagnoses.filter(d => d.autoFixed).length;
  const failCount = diagnoses.filter(d => d.status === "fail" && !d.autoFixed).length;

  const report: DoctorReport = {
    timestamp: new Date().toISOString(),
    diagnoses,
    fixedCount,
    failCount,
    allPassed: failCount === 0,
  };

  if (fixedCount > 0) {
    logger.info(`Auto-fixed ${fixedCount} issue(s), ${failCount} remaining`);
  } else if (failCount > 0) {
    logger.warn(`Found ${failCount} issue(s) that could not be auto-fixed`);
  }

  return report;
}

function fixEventEmitterLeak(): DoctorDiagnosis {
  const exitListeners = process.listenerCount("exit");
  if (exitListeners > 10) {
    process.setMaxListeners(exitListeners + 5);
    return {
      check: "event_emitter_listeners",
      status: "fixed",
      message: `Exit listeners was ${exitListeners} (limit 10), increased maxListeners to ${exitListeners + 5}`,
      autoFixed: true,
    };
  }
  return {
    check: "event_emitter_listeners",
    status: "pass",
    message: `Exit listeners: ${exitListeners} (within limit)`,
  };
}

function fixStalePgConnections(): DoctorDiagnosis {
  if (!process.env.DATABASE_URL) {
    return { check: "stale_pg_connections", status: "pass", message: "No DATABASE_URL — skipping" };
  }
  try {
    const result = execSync(
      'psql "$DATABASE_URL" -c "SELECT count(*) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND state = \'idle\' AND query_start < now() - interval \'5 minutes\';" -t 2>&1 || echo "psql_failed"',
      { encoding: "utf-8", timeout: 5000 },
    );
    if (result.includes("psql_failed")) {
      return { check: "stale_pg_connections", status: "pass", message: "psql not available — skipping" };
    }
    const count = parseInt(result.trim(), 10) || 0;
    if (count > 0) {
      execSync(
        'psql "$DATABASE_URL" -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND state = \'idle\' AND query_start < now() - interval \'5 minutes\';" -t 2>&1 || true',
        { encoding: "utf-8", timeout: 5000 },
      );
      return {
        check: "stale_pg_connections",
        status: "fixed",
        message: `Terminated ${count} idle PostgreSQL connections older than 5min`,
        autoFixed: true,
      };
    }
    return {
      check: "stale_pg_connections",
      status: "pass",
      message: "No stale idle PostgreSQL connections found",
    };
  } catch {
    return {
      check: "stale_pg_connections",
      status: "pass",
      message: "PostgreSQL not reachable or psql not available — skipping",
    };
  }
}

function fixStaleLockFiles(root: string): DoctorDiagnosis {
  const masdayDir = join(root, ".masday");
  if (!existsSync(masdayDir)) {
    return { check: "stale_lock_files", status: "pass", message: "No .masday/ directory — skipping" };
  }

  const lockPatterns = [".db-journal", ".db-wal", ".db-shm"];
  const removed: string[] = [];

  try {
    const files = readdirSync(masdayDir, { recursive: true }) as string[];
    for (const file of files) {
      if (lockPatterns.some(p => file.endsWith(p))) {
        const fullPath = join(masdayDir, file);
        try {
          rmSync(fullPath, { force: true });
          removed.push(file);
        } catch { /* locked by active process, skip */ }
      }
    }
  } catch { /* directory read error, skip */ }

  if (removed.length > 0) {
    return {
      check: "stale_lock_files",
      status: "fixed",
      message: `Removed ${removed.length} stale lock file(s): ${removed.join(", ")}`,
      autoFixed: true,
    };
  }
  return { check: "stale_lock_files", status: "pass", message: "No stale lock files found" };
}

function fixCorruptJsonCache(root: string): DoctorDiagnosis {
  const stateFile = join(root, ".masday", "state", "masday.json");
  if (!existsSync(stateFile)) {
    return { check: "json_state_cache", status: "pass", message: "No JSON state cache — skipping" };
  }

  try {
    const content = readFileSync(stateFile, "utf-8");
    JSON.parse(content);
    return { check: "json_state_cache", status: "pass", message: "JSON state cache is valid" };
  } catch {
    try {
      const backup = stateFile + ".corrupt." + Date.now();
      renameSync(stateFile, backup);
      return {
        check: "json_state_cache",
        status: "fixed",
        message: `Corrupt cache renamed to ${backup}. Fresh state will be created on next write.`,
        autoFixed: true,
      };
    } catch {
      return {
        check: "json_state_cache",
        status: "fail",
        message: "JSON state cache is corrupt and could not be renamed",
      };
    }
  }
}
