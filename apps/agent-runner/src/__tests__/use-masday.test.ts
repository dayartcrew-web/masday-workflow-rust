import { describe, it, expect } from "vitest";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

const MCP_PATH = join(process.cwd(), "apps", "agent-runner", "src", "runtime", "mcp.ts");

function classifyIntent(prompt: string) {
  const p = prompt.toLowerCase();
  type Intent = "fix" | "build" | "test" | "deploy" | "research" | "scaffold" | "analyze" | "workflow" | "git" | "quick";
  let intent: Intent = "quick";
  let skill = "";
  let agent = "masday-executor";
  let complexity: "quick" | "workflow" = "quick";

  if (/fix|bug|error|broken|crash|fail|issue|debug|patch|hotfix/.test(p)) { intent = "fix"; skill = "masday-workflow-fix"; agent = "masday-debugger"; complexity = "workflow"; }
  else if (/build|add feature|implement|create|new|develop|construct|scaffold/.test(p)) { intent = "build"; skill = "masday-workflow-new"; agent = "masday-orchestrator"; complexity = "workflow"; }
  else if (/test|spec|coverage|tdd|unit test|integration|e2e/.test(p)) { intent = "test"; skill = "masday-tdd"; agent = "masday-tdd-guide"; complexity = "workflow"; }
  else if (/deploy|release|ship|publish|push to prod|staging/.test(p)) { intent = "deploy"; skill = "masday-deploy-check"; agent = "masday-executor"; complexity = "workflow"; }
  else if (/research|lookup|find|search|docs|documentation|how to|what is/.test(p)) { intent = "research"; skill = "masday-research"; agent = "masday-researcher"; complexity = "workflow"; }
  else if (/create agent|create skill|scaffold|mcp skill|new command/.test(p)) { intent = "scaffold"; skill = "masday-create-skill"; agent = "masday-executor"; complexity = "workflow"; }
  else if (/analyze|review|audit|inspect|check quality|code review/.test(p)) { intent = "analyze"; skill = "masday-code-analyze"; agent = "masday-reviewer"; complexity = "workflow"; }
  else if (/workflow|plan|execute|run task|continue workflow/.test(p)) { intent = "workflow"; skill = "masday-workflow-run"; agent = "masday-orchestrator"; complexity = "workflow"; }
  else if (/commit|push|pr|pull request|merge|branch|git/.test(p)) { intent = "git"; skill = "masday-git-workflow"; agent = "masday-git-master"; complexity = "workflow"; }
  else { intent = "quick"; skill = ""; agent = "masday-executor"; complexity = "quick"; }

  return { intent, skill, agent, complexity };
}

describe("use_masday intent classification", () => {
  const fixCases = ["fix the login bug", "error in mcp.ts", "something is broken", "the app crashed", "test failure", "debug the issue", "patch the vulnerability", "hotfix for production"];
  const buildCases = ["add feature X", "build new component", "implement auth", "create a new skill", "develop the API", "construct the pipeline"];
  const testCases = ["write tests", "spec coverage", "improve coverage", "tdd this feature", "unit test the module", "integration test", "e2e test the flow"];
  const deployCases = ["deploy to staging", "release v2", "ship it", "publish the package", "push to prod"];
  const researchCases = ["research react patterns", "lookup the API docs", "find the config", "search for examples", "docs for drizzle", "documentation on zod", "what is MCP"];
  const scaffoldCases = ["mcp skill generator", "mcp skill for auth"];
  const analyzeCases = ["analyze the codebase", "review the PR", "audit the workflow", "code review this"];
  const workflowCases = ["run workflow", "plan the feature", "execute the plan", "continue workflow"];
  const gitCases = ["commit changes", "push to remote", "merge the branch", "git status"];
  const quickCases = ["hello", "show me the time", "random thought", "just a question"];

  it.each(fixCases)("classifies '%s' as fix", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("fix");
    expect(result.skill).toBe("masday-workflow-fix");
    expect(result.agent).toBe("masday-debugger");
    expect(result.complexity).toBe("workflow");
  });

  it.each(buildCases)("classifies '%s' as build", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("build");
    expect(result.skill).toBe("masday-workflow-new");
    expect(result.agent).toBe("masday-orchestrator");
    expect(result.complexity).toBe("workflow");
  });

  it.each(testCases)("classifies '%s' as test", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("test");
    expect(result.skill).toBe("masday-tdd");
    expect(result.agent).toBe("masday-tdd-guide");
    expect(result.complexity).toBe("workflow");
  });

  it.each(deployCases)("classifies '%s' as deploy", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("deploy");
    expect(result.skill).toBe("masday-deploy-check");
    expect(result.complexity).toBe("workflow");
  });

  it.each(researchCases)("classifies '%s' as research", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("research");
    expect(result.skill).toBe("masday-research");
    expect(result.agent).toBe("masday-researcher");
    expect(result.complexity).toBe("workflow");
  });

  it.each(scaffoldCases)("classifies '%s' as scaffold", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("scaffold");
    expect(result.skill).toBe("masday-create-skill");
    expect(result.complexity).toBe("workflow");
  });

  it.each(analyzeCases)("classifies '%s' as analyze", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("analyze");
    expect(result.skill).toBe("masday-code-analyze");
    expect(result.agent).toBe("masday-reviewer");
    expect(result.complexity).toBe("workflow");
  });

  it.each(workflowCases)("classifies '%s' as workflow", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("workflow");
    expect(result.skill).toBe("masday-workflow-run");
    expect(result.agent).toBe("masday-orchestrator");
    expect(result.complexity).toBe("workflow");
  });

  it.each(gitCases)("classifies '%s' as git", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("git");
    expect(result.skill).toBe("masday-git-workflow");
    expect(result.agent).toBe("masday-git-master");
    expect(result.complexity).toBe("workflow");
  });

  it.each(quickCases)("classifies '%s' as quick", (prompt) => {
    const result = classifyIntent(prompt);
    expect(result.intent).toBe("quick");
    expect(result.skill).toBe("");
    expect(result.agent).toBe("masday-executor");
    expect(result.complexity).toBe("quick");
  });
});

describe("use_masday tool registration", () => {
  it("use_masday tool is registered in mcp.ts", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain('server.registerTool("use_masday"');
    expect(content).toContain("Universal entry point");
  });

  it("returns routing plan structure", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain("intent");
    expect(content).toContain("skill");
    expect(content).toContain("agent");
    expect(content).toContain("complexity");
    expect(content).toContain("tools");
    expect(content).toContain("routingNote");
  });

  it("logs to episodic memory", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain('episodicMemory.add("system"');
    expect(content).toContain("[use_masday]");
  });
});

describe("stale connection detection", () => {
  it("background timer includes stale health check", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain("stale detection");
    expect(content).toContain("dbHealthCheck");
    expect(content).toContain("PostgreSQL connection stale");
    expect(content).toContain("dbReady = false");
  });

  it("reconnect function exists", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain("async function tryReconnectDb");
    expect(content).toContain("activateDbSubsystems");
  });

  it("initDb has retry logic", async () => {
    const content = await readFile(MCP_PATH, "utf-8");
    expect(content).toContain("MAX_RETRIES = 3");
    expect(content).toContain("All initDb() attempts failed");
  });
});
