/**
 * @mcp-rebuild/code-skills
 *
 * Plain-function wrappers around common development tool commands:
 * git, npm, docker, GitHub (gh CLI), CI/CD, test runners, and
 * semantic code search.
 */

// Shared helper
export { runCommand } from "./run-command.js";
export type { RunCommandOptions, RunCommandResult } from "./run-command.js";

// Git
export {
  runGitStatus,
  runGitDiff,
  runGitCommit,
  gitStatusSchema,
  gitDiffSchema,
  gitCommitSchema,
} from "./git.js";
export type {
  GitStatusInput,
  GitStatusOutput,
  GitDiffInput,
  GitDiffOutput,
  GitCommitInput,
  GitCommitOutput,
} from "./git.js";

// NPM
export {
  runNpmInstall,
  runNpmRun,
  npmInstallSchema,
  npmRunSchema,
} from "./npm.js";
export type {
  NpmInstallInput,
  NpmInstallOutput,
  NpmRunInput,
  NpmRunOutput,
} from "./npm.js";

// Docker
export {
  runDockerBuild,
  runDockerRun,
  runDockerPs,
  dockerBuildSchema,
  dockerRunSchema,
  dockerPsSchema,
} from "./docker.js";
export type {
  DockerBuildInput,
  DockerBuildOutput,
  DockerRunInput,
  DockerRunOutput,
  DockerPsInput,
  DockerPsOutput,
} from "./docker.js";

// CI/CD
export {
  runCicdStatus,
  runCicdTrigger,
  runCicdView,
  cicdStatusSchema,
  cicdTriggerSchema,
  cicdViewSchema,
} from "./cicd.js";
export type {
  CicdStatusInput,
  CicdStatusOutput,
  CicdTriggerInput,
  CicdTriggerOutput,
  CicdViewInput,
  CicdViewOutput,
} from "./cicd.js";

// GitHub
export {
  runGithubPrCreate,
  runGithubPrList,
  runGithubIssueList,
  githubPrCreateSchema,
  githubPrListSchema,
  githubIssueListSchema,
} from "./github.js";
export type {
  GithubPrCreateInput,
  GithubPrCreateOutput,
  GithubPrListInput,
  GithubPrListOutput,
  GithubIssueListInput,
  GithubIssueListOutput,
} from "./github.js";

// Tests
export {
  runTests,
  testsRunSchema,
} from "./tests.js";
export type {
  TestsRunInput,
  TestsRunOutput,
} from "./tests.js";

// Code search
export {
  runCodeSearch,
  codeSearchSchema,
} from "./code.js";
export type {
  CodeSearchInput,
  CodeSearchOutput,
} from "./code.js";
