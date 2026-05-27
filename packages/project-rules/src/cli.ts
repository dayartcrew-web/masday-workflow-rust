import { resolve } from "node:path";
import { validateProject, formatReport, getFailedCritical } from "./validator.js";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("project-rules");

const projectRoot = resolve(process.argv[2] ?? process.cwd());

logger.info(`Checking project rules at: ${projectRoot}\n`);

const report = validateProject(projectRoot);
logger.info(formatReport(report));

const critical = getFailedCritical(report);
if (critical.length > 0) {
  logger.error(`\n${critical.length} CRITICAL issue(s) found. Fix before committing.`);
  process.exit(1);
} else if (report.failed > 0) {
  logger.warn(`\n${report.failed} non-critical issue(s) found.`);
}
