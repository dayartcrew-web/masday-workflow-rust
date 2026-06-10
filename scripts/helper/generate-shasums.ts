/**
 * SHA256 Checksum Generator for CLI binaries.
 *
 * Reads all binary files from dist/cli/, generates SHA256 checksums,
 * writes individual .sha256 files next to each binary, and produces
 * an aggregated SHASUMS256.txt compatible with `sha256sum --check`.
 *
 * Usage:
 *   npx tsx scripts/generate-shasums.ts
 *   npx tsx scripts/generate-shasums.ts --dir dist/bin
 */

import * as crypto from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";
import { resolveProjectRoot, formatBytes } from "./utils.js";

// --- Types ---

interface ChecksumEntry {
  readonly filename: string;
  readonly hash: string;
  readonly sizeBytes: number;
}

// --- Constants ---

const DEFAULT_DIR = "dist/cli";
const AGGREGATED_FILE = "SHASUMS256.txt";

// --- Helpers ---

/** Parse CLI arguments. */
function parseArgs(args: readonly string[]): { dir: string } {
  const dirIdx = args.indexOf("--dir");
  const dir =
    dirIdx !== -1 && args[dirIdx + 1] ? args[dirIdx + 1] : DEFAULT_DIR;
  return { dir };
}

/** Check if a filename looks like a checksum or metadata file we should skip. */
function isSkippableFile(filename: string): boolean {
  return (
    filename.endsWith(".sha256") ||
    filename === AGGREGATED_FILE ||
    filename.endsWith(".txt") ||
    filename.endsWith(".json")
  );
}

/**
 * Compute SHA256 hash of a file, reading as a binary Buffer.
 * Returns the lowercase hex digest.
 */
function computeSha256(filePath: string): string {
  const buffer: Buffer = fs.readFileSync(filePath);
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

/** Write a single .sha256 sidecar file next to the binary. */
function writeSidecar(
  dir: string,
  filename: string,
  hash: string
): void {
  const sidecarPath = path.join(dir, `${filename}.sha256`);
  // sha256sum format: <hash>  <filename>  (two spaces)
  fs.writeFileSync(sidecarPath, `${hash}  ${filename}\n`, "utf-8");
}

/** Write the aggregated SHASUMS256.txt file. */
function writeAggregated(
  dir: string,
  entries: readonly ChecksumEntry[]
): void {
  const lines = entries.map((e) => `${e.hash}  ${e.filename}`);
  const outPath = path.join(dir, AGGREGATED_FILE);
  fs.writeFileSync(outPath, lines.join("\n") + "\n", "utf-8");
}

// --- Main ---

function main(): void {
  const projectRoot = resolveProjectRoot();
  const { dir: relativeDir } = parseArgs(process.argv.slice(2));
  const distDir = path.resolve(projectRoot, relativeDir);

  console.log("SHA256 Checksum Generator");
  console.log(`Project root: ${projectRoot}`);
  console.log(`Target dir:   ${relativeDir}`);

  // Validate directory exists
  if (!fs.existsSync(distDir)) {
    console.error(`ERROR: Directory not found: ${distDir}`);
    console.error("Run the CLI binary build first (e.g., build:bin or build:sea).");
    process.exit(1);
  }

  const distStat = fs.statSync(distDir);
  if (!distStat.isDirectory()) {
    console.error(`ERROR: Not a directory: ${distDir}`);
    process.exit(1);
  }

  // Collect binary files (skip .sha256, .txt, .json sidecars)
  const files = fs
    .readdirSync(distDir)
    .filter((name) => {
      const fullPath = path.join(distDir, name);
      return fs.statSync(fullPath).isFile() && !isSkippableFile(name);
    })
    .sort();

  if (files.length === 0) {
    console.error(`ERROR: No binary files found in ${distDir}`);
    process.exit(1);
  }

  console.log(`Found ${files.length} file(s) to checksum\n`);

  // Process each file
  const entries: ChecksumEntry[] = files.map((filename) => {
    const fullPath = path.join(distDir, filename);
    const sizeBytes = fs.statSync(fullPath).size;
    const hash = computeSha256(fullPath);

    writeSidecar(distDir, filename, hash);

    console.log(`  ${hash}  ${filename}  (${formatBytes(sizeBytes)})`);

    return { filename, hash, sizeBytes };
  });

  // Write aggregated file
  writeAggregated(distDir, entries);

  console.log(
    `\nWrote ${entries.length} checksum(s) + ${AGGREGATED_FILE}`
  );
  console.log(
    `Verify with: cd ${relativeDir} && sha256sum -c ${AGGREGATED_FILE}`
  );
}

main();
