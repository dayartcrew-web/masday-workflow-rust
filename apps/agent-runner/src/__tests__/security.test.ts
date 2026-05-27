import { describe, it, expect } from "vitest";
import path from "path";
import fs from "fs";

const cwd = process.cwd();

describe("safePath", () => {
  function safePath(input: string | undefined, fallback: string = cwd): string {
    if (!input || typeof input !== "string" || input.trim() === "") return fallback;
    const resolved = path.resolve(input);
    if (!resolved.startsWith(cwd)) return fallback;
    if (!fs.existsSync(resolved)) return fallback;
    return resolved;
  }

  it("returns fallback for undefined input", () => {
    expect(safePath(undefined)).toBe(cwd);
  });

  it("returns fallback for empty string", () => {
    expect(safePath("")).toBe(cwd);
  });

  it("returns fallback for whitespace-only string", () => {
    expect(safePath("   ")).toBe(cwd);
  });

  it("returns fallback for path outside cwd", () => {
    expect(safePath("/etc/passwd")).toBe(cwd);
  });

  it("returns fallback for path traversal attempt", () => {
    expect(safePath("../../../etc/passwd")).toBe(cwd);
  });

  it("returns fallback for non-existent path inside cwd", () => {
    expect(safePath(path.join(cwd, "nonexistent_dir_xyz_abc"))).toBe(cwd);
  });

  it("returns resolved path for existing directory inside cwd", () => {
    const result = safePath(cwd);
    expect(result).toBe(cwd);
  });

  it("returns custom fallback when provided", () => {
    const customFallback = "/tmp/custom";
    expect(safePath(undefined, customFallback)).toBe(customFallback);
  });
});

describe("Input validation regexes", () => {
  const SAFE_SCRIPT_RE = /^[a-zA-Z0-9_-]+$/;
  const SAFE_IMAGE_RE = /^[a-zA-Z0-9._:/-]+$/;
  const SAFE_PIPELINE_RE = /^[a-zA-Z0-9_.-]+$/;
  const SAFE_PACKAGE_RE = /^(@[a-zA-Z0-9_-]+\/)?[a-zA-Z0-9_.-]+$/;
  const SAFE_PATTERN_RE = /^[a-zA-Z0-9_./\\*-]+$/;

  describe("SAFE_SCRIPT_RE", () => {
    it("accepts valid script names", () => {
      expect(SAFE_SCRIPT_RE.test("build")).toBe(true);
      expect(SAFE_SCRIPT_RE.test("test")).toBe(true);
      expect(SAFE_SCRIPT_RE.test("my-script")).toBe(true);
      expect(SAFE_SCRIPT_RE.test("script_name")).toBe(true);
    });

    it("rejects shell injection in script names", () => {
      expect(SAFE_SCRIPT_RE.test(";rm -rf /")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("$(whoami)")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("`id`")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("build && malicious")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("build|malicious")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("build;malicious")).toBe(false);
      expect(SAFE_SCRIPT_RE.test("build\ncurl")).toBe(false);
    });
  });

  describe("SAFE_IMAGE_RE", () => {
    it("accepts valid docker image names", () => {
      expect(SAFE_IMAGE_RE.test("node:18")).toBe(true);
      expect(SAFE_IMAGE_RE.test("my-registry.com/my-image:v1")).toBe(true);
      expect(SAFE_IMAGE_RE.test("ubuntu")).toBe(true);
    });

    it("rejects shell injection in image names", () => {
      expect(SAFE_IMAGE_RE.test("node:18;rm -rf /")).toBe(false);
      expect(SAFE_IMAGE_RE.test("$(cat /etc/passwd)")).toBe(false);
      expect(SAFE_IMAGE_RE.test("node`id`")).toBe(false);
      expect(SAFE_IMAGE_RE.test("node|malicious")).toBe(false);
    });
  });

  describe("SAFE_PIPELINE_RE", () => {
    it("accepts valid pipeline names", () => {
      expect(SAFE_PIPELINE_RE.test("ci.yml")).toBe(true);
      expect(SAFE_PIPELINE_RE.test("my-pipeline")).toBe(true);
      expect(SAFE_PIPELINE_RE.test("build_test")).toBe(true);
    });

    it("rejects shell injection in pipeline names", () => {
      expect(SAFE_PIPELINE_RE.test("ci.yml;rm -rf /")).toBe(false);
      expect(SAFE_PIPELINE_RE.test("$(whoami)")).toBe(false);
      expect(SAFE_PIPELINE_RE.test("pipeline`id`")).toBe(false);
    });
  });

  describe("SAFE_PACKAGE_RE", () => {
    it("accepts valid package names", () => {
      expect(SAFE_PACKAGE_RE.test("lodash")).toBe(true);
      expect(SAFE_PACKAGE_RE.test("@mcp-rebuild/core")).toBe(true);
      expect(SAFE_PACKAGE_RE.test("drizzle-orm")).toBe(true);
      expect(SAFE_PACKAGE_RE.test("vitest")).toBe(true);
    });

    it("rejects shell injection in package names", () => {
      expect(SAFE_PACKAGE_RE.test("lodash;rm -rf /")).toBe(false);
      expect(SAFE_PACKAGE_RE.test("$(whoami)")).toBe(false);
      expect(SAFE_PACKAGE_RE.test("pkg`id`")).toBe(false);
      expect(SAFE_PACKAGE_RE.test("pkg && malicious")).toBe(false);
    });
  });

  describe("SAFE_PATTERN_RE", () => {
    it("accepts valid test patterns", () => {
      expect(SAFE_PATTERN_RE.test("security.test")).toBe(true);
      expect(SAFE_PATTERN_RE.test("packages/store/**/*.test.*")).toBe(true);
      expect(SAFE_PATTERN_RE.test("./__tests__/")).toBe(true);
    });

    it("rejects shell injection in test patterns", () => {
      expect(SAFE_PATTERN_RE.test("test;rm -rf /")).toBe(false);
      expect(SAFE_PATTERN_RE.test("$(whoami)")).toBe(false);
      expect(SAFE_PATTERN_RE.test("test`id`")).toBe(false);
      expect(SAFE_PATTERN_RE.test("test && malicious")).toBe(false);
      expect(SAFE_PATTERN_RE.test("test|malicious")).toBe(false);
    });
  });
});

describe("restrictToProject", () => {
  function restrictToProject(userPath: string): string {
    const resolved = path.resolve(userPath);
    if (!resolved.startsWith(cwd)) throw new Error(`Path "${userPath}" is outside project root`);
    return resolved;
  }

  it("accepts paths inside project root", () => {
    const result = restrictToProject("src/index.ts");
    expect(result).toContain("src");
  });

  it("accepts the project root itself", () => {
    expect(() => restrictToProject(cwd)).not.toThrow();
  });

  it("throws for absolute path outside project root", () => {
    expect(() => restrictToProject("/etc/passwd")).toThrow("outside project root");
  });

  it("throws for path traversal attempt", () => {
    expect(() => restrictToProject("../../../etc/passwd")).toThrow("outside project root");
  });

  it("throws for relative traversal beyond project root", () => {
    expect(() => restrictToProject("../../../../")).toThrow("outside project root");
  });
});

describe("Shell tool command construction", () => {
  it("npm.install should NOT use execSync with string interpolation", () => {
    const source = fs.readFileSync(
      path.join(__dirname, "..", "runtime", "mcp.ts"),
      "utf-8"
    );

    const npmInstallSection = source.substring(
      source.indexOf("npm.install"),
      source.indexOf("npm.run")
    );

    const hasUnsafeExec = /execSync\(cmd/.test(npmInstallSection) ||
      /execSync\(`pnpm add/.test(npmInstallSection);

    expect(hasUnsafeExec).toBe(false);
  });

  it("tests.run should NOT use execSync with string interpolation", () => {
    const source = fs.readFileSync(
      path.join(__dirname, "..", "runtime", "mcp.ts"),
      "utf-8"
    );

    const testsRunSection = source.substring(
      source.indexOf('"tests.run"'),
      source.indexOf("capability.ping")
    );

    const hasUnsafeExec = /execSync\(cmd/.test(testsRunSection) ||
      /execSync\(`pnpm test/.test(testsRunSection);

    expect(hasUnsafeExec).toBe(false);
  });

  it("git.commit should use execFileSync", () => {
    const source = fs.readFileSync(
      path.join(__dirname, "..", "runtime", "mcp.ts"),
      "utf-8"
    );

    const gitCommitSection = source.substring(
      source.indexOf('"git.commit"'),
      source.indexOf("npm.install")
    );

    expect(gitCommitSection).toContain("execFileSync");
    expect(gitCommitSection).not.toContain("execSync(`git commit");
  });

  it("docker tools should use execFileSync", () => {
    const source = fs.readFileSync(
      path.join(__dirname, "..", "runtime", "mcp.ts"),
      "utf-8"
    );

    const dockerSection = source.substring(
      source.indexOf('"docker.build"'),
      source.indexOf('"cicd.')
    );

    expect(dockerSection).toContain("execFileSync");
  });

  it("cicd.pipeline_trigger should use execFileSync", () => {
    const source = fs.readFileSync(
      path.join(__dirname, "..", "runtime", "mcp.ts"),
      "utf-8"
    );

    const cicdTriggerSection = source.substring(
      source.indexOf('"cicd.pipeline_trigger"'),
      source.indexOf('"cicd.runs_view"')
    );

    expect(cicdTriggerSection).toContain("execFileSync");
  });
});
