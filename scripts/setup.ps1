$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$HomeClaude = Join-Path $env:USERPROFILE ".claude"
$HomeGemini = Join-Path $env:USERPROFILE ".gemini"
$HomeOpencode = Join-Path (Join-Path $env:USERPROFILE ".config") "opencode"

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

# 1. Install dependencies
Write-Host "[1/10] Installing dependencies..." -ForegroundColor Yellow
pnpm install

# 2. Generate Drizzle client
Write-Host "[2/10] Generating Drizzle client..." -ForegroundColor Yellow
if ((Test-Path "$RootDir\node_modules\drizzle-orm\index.js") -or (Test-Path "$RootDir\packages\db\node_modules\drizzle-orm\index.js")) {
    Write-Host "  Drizzle ORM already installed, skipping (run 'pnpm db:generate' manually to update)" -ForegroundColor Gray
} else {
    pnpm db:generate
}

# 3. Build all packages
Write-Host "[3/10] Building all packages..." -ForegroundColor Yellow
pnpm build

# 4. Build agent-runner MCP server
Write-Host "[4/10] Building agent-runner MCP server..." -ForegroundColor Yellow
pnpm --filter @mcp-rebuild/agent-runner build

# 4b. pgvector setup (PostgreSQL only)
$databaseUrl = $env:DATABASE_URL
if ($databaseUrl -and $databaseUrl -ne "sqlite://local") {
    $dimensions = if ($env:EMBEDDING_DIMENSIONS) { $env:EMBEDDING_DIMENSIONS } else { "768" }
    Write-Host "  Running pgvector column setup (EMBEDDING_DIMENSIONS=$dimensions)..." -ForegroundColor DarkGray
    node "$RootDir\scripts\setup-pgvector.mjs"
    Write-Host "  pgvector ready." -ForegroundColor DarkGray
} else {
    Write-Host "  DATABASE_URL is sqlite://local or unset — skipping pgvector setup." -ForegroundColor DarkGray
    Write-Host "  Set DATABASE_URL to a PostgreSQL URL and run 'pnpm db:pgvector' to enable vector search." -ForegroundColor DarkGray
}

# 5. Create .env if missing
Write-Host "[5/10] Checking .env file..." -ForegroundColor Yellow
if ((-not (Test-Path "$RootDir\.env")) -and (Test-Path "$RootDir\.env.example")) {
    Copy-Item "$RootDir\.env.example" "$RootDir\.env"
    Write-Host "  Created .env from .env.example — fill in your values before starting." -ForegroundColor Gray
} elseif (-not (Test-Path "$RootDir\.env")) {
    Write-Host "  No .env or .env.example found — skipping." -ForegroundColor Gray
} else {
    Write-Host "  .env already exists." -ForegroundColor Gray
}

# 6. Sync masday-* skills to local platform directories
Write-Host "[6/10] Syncing masday-* skills to local platform directories..." -ForegroundColor Yellow

$masdaySkills = Get-ChildItem -Path "$RootDir\.claude\skills" -Directory -Filter "masday-*"

# Preserve .gemini/settings.json before cleaning
$geminiSettingsPath = Join-Path $RootDir ".gemini\settings.json"
$geminiSettingsBak = $null
if (Test-Path $geminiSettingsPath) {
    $geminiSettingsBak = Get-Content $geminiSettingsPath -Raw
}

$platforms = @(
    @{Dest = ".agents"; Agents = ".agents\agents"; Skills = ".agents\skills" },
    @{Dest = ".gemini"; Agents = ".gemini\agents"; Skills = ".gemini\skills" },
    @{Dest = ".continue"; Agents = ".continue\agents"; Skills = ".continue\skills" }
)
foreach ($p in $platforms) {
    if (Test-Path $p.Dest) { Remove-Item -Path $p.Dest -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $p.Agents | Out-Null
    New-Item -ItemType Directory -Force -Path $p.Skills | Out-Null
    Copy-Item -Path "$RootDir\.claude\agents\*" -Destination $p.Agents -Recurse -Force -ErrorAction SilentlyContinue
    foreach ($skill in $masdaySkills) {
        Copy-Item -Path $skill.FullName -Destination $p.Skills -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Restore .gemini/settings.json
if ($geminiSettingsBak) {
    Set-Content -Path $geminiSettingsPath -Value $geminiSettingsBak -NoNewline
} elseif (Test-Path "$RootDir\scripts\.gemini\settings.json") {
    Copy-Item "$RootDir\scripts\.gemini\settings.json" $geminiSettingsPath -Force
}

# Sync rules to all platforms
Write-Host "  Syncing .claude/rules/ to all platform directories..." -ForegroundColor DarkGray
$rulesSource = Join-Path $RootDir ".claude\rules"
$platDirs = @(".agents", ".gemini", ".continue", ".opencode", ".codex")
foreach ($platDir in $platDirs) {
    $destRules = Join-Path $RootDir "$platDir\rules"
    New-Item -ItemType Directory -Force -Path $destRules | Out-Null
    if (Test-Path $destRules) { Remove-Item -Path "$destRules\*" -Recurse -Force -ErrorAction SilentlyContinue }
    if (Test-Path $rulesSource) {
        Copy-Item -Path "$rulesSource\*" -Destination $destRules -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 7. Install masday-* skills to global directories
Write-Host "[7/10] Installing masday-* skills to global directories..." -ForegroundColor Yellow

# Claude Code: ~/.claude/skills/
$claudeSkillsDir = Join-Path $HomeClaude "skills"
New-Item -ItemType Directory -Force -Path $claudeSkillsDir | Out-Null
$copiedClaude = 0
foreach ($skill in $masdaySkills) {
    $dest = Join-Path $claudeSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedClaude++
}
Write-Host "  Claude Code: $copiedClaude skills -> $claudeSkillsDir" -ForegroundColor Cyan

# Gemini: ~/.gemini/config/skills/
$geminiSkillsDir = Join-Path (Join-Path $HomeGemini "config") "skills"
New-Item -ItemType Directory -Force -Path $geminiSkillsDir | Out-Null
$copiedGemini = 0
foreach ($skill in $masdaySkills) {
    $dest = Join-Path $geminiSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedGemini++
}
Write-Host "  Gemini CLI:  $copiedGemini skills -> $geminiSkillsDir" -ForegroundColor Cyan

# OpenCode: ~/.config/opencode/skills/
$opencodeSkillsDir = Join-Path $HomeOpencode "skills"
New-Item -ItemType Directory -Force -Path $opencodeSkillsDir | Out-Null
$copiedOpencode = 0
foreach ($skill in $masdaySkills) {
    $dest = Join-Path $opencodeSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedOpencode++
}
Write-Host "  OpenCode:    $copiedOpencode skills -> $opencodeSkillsDir" -ForegroundColor Cyan

# 8. Convert and install agents to OpenCode
Write-Host "[8/10] Converting agents to opencode format (global + project)..." -ForegroundColor Yellow
$opencodeAgentsDir = Join-Path $HomeOpencode "agent"
$projectAgentsDir = "$RootDir\.opencode\agent"
New-Item -ItemType Directory -Force -Path $opencodeAgentsDir | Out-Null
New-Item -ItemType Directory -Force -Path $projectAgentsDir | Out-Null
if (Test-Path "$RootDir\scripts\convert-agents.mjs") {
    node "$RootDir\scripts\convert-agents.mjs" convert "$RootDir\.claude\agents"
    Write-Host "  OpenCode agents converted." -ForegroundColor Cyan
} else {
    Write-Host "  scripts/convert-agents.mjs not found — skipping." -ForegroundColor DarkGray
}

# 9. MCP config + Copilot agents/hooks for each platform
Write-Host "[9/10] Setting up MCP configs + Copilot customization..." -ForegroundColor Yellow

$McpJs = "apps/agent-runner/dist/runtime/mcp.js"

# .mcp.json (Claude Code)
$mcpJson = @{
    mcpServers = @{
        masday = @{
            type = "stdio"
            command = "node"
            args = @("apps/agent-runner/dist/runtime/mcp.js")
        }
    }
} | ConvertTo-Json -Depth 5
Set-Content -Path "$RootDir\.mcp.json" -Value $mcpJson -NoNewline
Write-Host "  .mcp.json (Claude Code)" -ForegroundColor Cyan

# .gemini/settings.json (already restored or copied in step 6)
if (-not (Test-Path $geminiSettingsPath)) {
    $geminiSettings = @{
        context = "This project uses masday-workflow-rebuild MCP server for workflow management."
        mcpServers = @{
            masday = @{
                type = "stdio"
                command = "node"
                args = @("--no-warnings", "apps/agent-runner/dist/runtime/mcp.js")
            }
        }
    } | ConvertTo-Json -Depth 5
    Set-Content -Path $geminiSettingsPath -Value $geminiSettings -NoNewline
}
Write-Host "  .gemini/settings.json (Gemini CLI)" -ForegroundColor Cyan

# .vscode/mcp.json (VS Code Copilot)
# Docs: https://code.visualstudio.com/docs/copilot/customization/mcp-servers
# No "type" field needed — stdio is inferred when "command" is present.
New-Item -ItemType Directory -Force -Path "$RootDir\.vscode" | Out-Null
$vscodeMcp = @{
    servers = @{
        masday = @{
            command = "node"
            args = @("--no-warnings", "apps/agent-runner/dist/runtime/mcp.js")
        }
    }
} | ConvertTo-Json -Depth 5
Set-Content -Path "$RootDir\.vscode\mcp.json" -Value $vscodeMcp -NoNewline
Write-Host "  .vscode\mcp.json (VS Code Copilot) — node + built JS" -ForegroundColor Cyan

# .github/agents/masday.agent.md (VS Code Copilot custom agent)
# Docs: https://code.visualstudio.com/docs/copilot/customization/custom-agents
New-Item -ItemType Directory -Force -Path "$RootDir\.github\agents" | Out-Null
$agentContent = @"
---
name: masday
description: masday-workflow-rebuild workflow orchestration agent with 87 MCP tools
tools: ['*']
model: ['Claude Sonnet 4.6', 'GPT-5.2']
handoffs:
  - label: Implement Plan
    agent: agent
    prompt: Implement the plan outlined above.
    send: false
---

# masday Agent

You are the masday-workflow-rebuild orchestration agent. You have access to 87 MCP tools across 16 namespaces.

## Mandatory Protocol

1. **Check masday MCP tools first** - use MCP tools before falling back to shell commands.
2. **Follow the workflow lifecycle** - INIT > ANALYZE > PLAN > EXECUTE > VERIFY > DONE
3. **Enforce review pipeline** - after completing work, run review_submit > policy_validate_completion > workflow_completeTask
4. **Use underscore tool names** - all MCP tools use underscore format (e.g., ``workflow_create``, ``memory_store``)

## Priority Order

1. masday MCP tools (workflow, memory, search, policy, capability)
2. Agent orchestrator for task routing
3. Code skills for implementation

## Pre-Commit Checks

Before marking any task complete:
- Run ``pnpm typecheck`` - must pass with zero errors
- Run ``pnpm test`` - all tests must pass
- No hardcoded secrets or credentials
- No console.log statements in production code
"@
Set-Content -Path "$RootDir\.github\agents\masday.agent.md" -Value $agentContent -NoNewline
Write-Host "  .github\agents\masday.agent.md (VS Code Copilot custom agent)" -ForegroundColor Cyan

# .github/hooks/masday-hooks.json (VS Code Copilot hooks)
# Docs: https://code.visualstudio.com/docs/copilot/customization/hooks
New-Item -ItemType Directory -Force -Path "$RootDir\.github\hooks" | Out-Null
$hooksJson = @"
{
  "hooks": {
    "SessionStart": [
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs masday-mem-context", "timeout": 15 }
    ],
    "PreToolUse": [
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs pre-tool-use", "timeout": 30 },
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs workflow-lock", "timeout": 30 },
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs tdd-guard", "timeout": 30 }
    ],
    "PostToolUse": [
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs post-tool-use", "timeout": 30 }
    ],
    "Stop": [
      { "type": "command", "command": "node .claude/hooks/run-hook.mjs on-stop", "timeout": 60 }
    ]
  }
}
"@
Set-Content -Path "$RootDir\.github\hooks\masday-hooks.json" -Value $hooksJson -NoNewline
Write-Host "  .github\hooks\masday-hooks.json (VS Code Copilot hooks)" -ForegroundColor Cyan

# Copilot user-level MCP registration (optional, uses `code` CLI)
if (Get-Command code -ErrorAction SilentlyContinue) {
    $absMcpJs = Join-Path $RootDir $McpJs
    $addMcpJson = "{`"name`":`"masday`",`"command`":`"node`",`"args`":[`"--no-warnings`",`"$absMcpJs`"]}"
    try {
        code --add-mcp $addMcpJson 2>$null
        Write-Host "  User-level MCP registered via 'code --add-mcp'" -ForegroundColor Cyan
    } catch {
        Write-Host "  'code --add-mcp' skipped (VS Code not running or not available)" -ForegroundColor DarkGray
    }
} else {
    Write-Host "  'code' CLI not found - skipping user-level MCP registration" -ForegroundColor DarkGray
}

# 10. Git hooks + .masday state dirs
Write-Host "[10/10] Installing git hooks..." -ForegroundColor Yellow
if (Test-Path "$RootDir\.git\hooks") {
    foreach ($hook in @("pre-commit", "pre-push")) {
        $src = Join-Path $RootDir "scripts\git-hooks\$hook"
        if (Test-Path $src) {
            Copy-Item $src "$RootDir\.git\hooks\$hook" -Force
        }
    }
    Write-Host "  Git hooks installed (pre-commit + pre-push)" -ForegroundColor Cyan
}
New-Item -ItemType Directory -Force -Path "$RootDir\.masday\cache\tasks" | Out-Null
New-Item -ItemType Directory -Force -Path "$RootDir\.masday\reports" | Out-Null

# Summary
Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "  Claude Code:  .claude/settings.json (hooks) + .mcp.json (MCP)"
Write-Host "  Gemini CLI:   .gemini/settings.json (MCP via node)"
Write-Host "                $geminiSkillsDir ($copiedGemini skills)"
Write-Host "  VS Code:      .vscode\mcp.json (Copilot MCP)"
Write-Host "                .github\agents\masday.agent.md (custom agent)"
Write-Host "                .github\hooks\masday-hooks.json (Copilot hooks)"
Write-Host "  GitHub:       .github\agents\ (coding agent)"
Write-Host "  OpenCode:     .opencode\agent\ ($copiedOpencode agents converted)"
Write-Host "  Git hooks:    .git\hooks\pre-commit + pre-push (ALL platforms)"
Write-Host "  Skills:       $copiedClaude masday-* skills installed"
Write-Host ""
Write-Host "Start: node apps\agent-runner\dist\runtime\mcp.js"
