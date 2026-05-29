import { describe, it, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import path from "path";
import os from "os";
import { runDoctor } from "../doctor.js";

function createTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "doctor-test-"));
}

function createMasdayDir(root: string): string {
  const masdayDir = path.join(root, ".masday");
  fs.mkdirSync(masdayDir, { recursive: true });
  return masdayDir;
}

describe("runDoctor", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("returns a valid DoctorReport structure", () => {
    const report = runDoctor();
    expect(report).toHaveProperty("timestamp");
    expect(report).toHaveProperty("diagnoses");
    expect(report).toHaveProperty("fixedCount");
    expect(report).toHaveProperty("failCount");
    expect(report).toHaveProperty("allPassed");
    expect(typeof report.timestamp).toBe("string");
    expect(Array.isArray(report.diagnoses)).toBe(true);
    expect(typeof report.fixedCount).toBe("number");
    expect(typeof report.failCount).toBe("number");
    expect(typeof report.allPassed).toBe("boolean");
  });

  it("always includes event_emitter_listeners diagnosis", () => {
    const report = runDoctor();
    const diag = report.diagnoses.find(d => d.check === "event_emitter_listeners");
    expect(diag).toBeDefined();
    expect(diag!.status).toMatch(/^(pass|fixed|fail)$/);
  });

  it("always includes stale_pg_connections diagnosis", () => {
    const report = runDoctor();
    const diag = report.diagnoses.find(d => d.check === "stale_pg_connections");
    expect(diag).toBeDefined();
  });

  it("skips project-specific checks without projectRoot", () => {
    const report = runDoctor();
    const projectChecks = report.diagnoses.filter(
      d => d.check === "stale_lock_files" || d.check === "json_state_cache",
    );
    expect(projectChecks).toHaveLength(0);
  });

  it("includes stale_lock_files when projectRoot is provided", () => {
    createMasdayDir(tempDir);
    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag).toBeDefined();
    expect(diag!.status).toBe("pass");
    expect(diag!.message).toContain("No stale lock files");
  });

  it("includes json_state_cache when projectRoot is provided", () => {
    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "json_state_cache");
    expect(diag).toBeDefined();
    expect(diag!.status).toBe("pass");
    expect(diag!.message).toContain("No JSON state cache");
  });

  it("allPassed is true when no failures", () => {
    const report = runDoctor(tempDir);
    if (report.failCount === 0) {
      expect(report.allPassed).toBe(true);
    }
  });
});

describe("fixStaleLockFiles", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("removes .db-journal files", () => {
    const masdayDir = createMasdayDir(tempDir);
    const stateDir = path.join(masdayDir, "state");
    fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(path.join(stateDir, "masday.db-journal"), "stale");

    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag).toBeDefined();
    expect(diag!.autoFixed).toBe(true);
    expect(diag!.status).toBe("fixed");
    expect(diag!.message).toContain("1 stale lock file");
  });

  it("removes .db-wal files", () => {
    const masdayDir = createMasdayDir(tempDir);
    fs.writeFileSync(path.join(masdayDir, "cache.db-wal"), "stale");

    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag).toBeDefined();
    expect(diag!.autoFixed).toBe(true);
  });

  it("removes .db-shm files", () => {
    const masdayDir = createMasdayDir(tempDir);
    fs.writeFileSync(path.join(masdayDir, "cache.db-shm"), "stale");

    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag).toBeDefined();
    expect(diag!.autoFixed).toBe(true);
  });

  it("passes when no lock files exist", () => {
    createMasdayDir(tempDir);
    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag!.status).toBe("pass");
  });

  it("passes when no .masday directory exists", () => {
    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "stale_lock_files");
    expect(diag!.status).toBe("pass");
    expect(diag!.message).toContain("No .masday/");
  });
});

describe("fixCorruptJsonCache", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = createTempDir();
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("passes when state file has valid JSON", () => {
    const stateDir = path.join(tempDir, ".masday", "state");
    fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(path.join(stateDir, "masday.json"), JSON.stringify({ ok: true }));

    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "json_state_cache");
    expect(diag!.status).toBe("pass");
    expect(diag!.message).toContain("valid");
  });

  it("auto-fixes corrupt JSON by renaming to backup", () => {
    const stateDir = path.join(tempDir, ".masday", "state");
    fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(path.join(stateDir, "masday.json"), "{ invalid json !!!");

    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "json_state_cache");
    expect(diag!.autoFixed).toBe(true);
    expect(diag!.status).toBe("fixed");
    expect(diag!.message).toContain("Corrupt cache renamed");

    const backupFiles = fs.readdirSync(stateDir).filter(f => f.startsWith("masday.json.corrupt."));
    expect(backupFiles.length).toBe(1);
  });

  it("passes when no state file exists", () => {
    const report = runDoctor(tempDir);
    const diag = report.diagnoses.find(d => d.check === "json_state_cache");
    expect(diag!.status).toBe("pass");
    expect(diag!.message).toContain("No JSON state cache");
  });
});
