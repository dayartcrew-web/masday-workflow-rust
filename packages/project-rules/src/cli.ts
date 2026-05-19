import { resolve } from "node:path";
import { validateProject, formatReport, getFailedCritical } from "./validator.js";

const projectRoot = resolve(process.argv[2] ?? process.cwd());

console.log(`Checking project rules at: ${projectRoot}\n`);

const report = validateProject(projectRoot);
console.log(formatReport(report));

const critical = getFailedCritical(report);
if (critical.length > 0) {
  console.error(`\n${critical.length} CRITICAL issue(s) found. Fix before committing.`);
  process.exit(1);
} else if (report.failed > 0) {
  console.warn(`\n${report.failed} non-critical issue(s) found.`);
}
