$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$HomeClaude = Join-Path $env:USERPROFILE ".claude"
$HomeOpencode = Join-Path $env:USERPROFILE ".config" "opencode"

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

# 1. Install dependencies
Write-Host "[1/8] Installing dependencies..." -ForegroundColor Yellow
pnpm install

# 2. Generate Prisma client
Write-Host "[2/8] Generating Prisma client..." -ForegroundColor Yellow
pnpm db:generate

# 3. Build all packages
Write-Host "[3/8] Building all packages..." -ForegroundColor Yellow
pnpm build

# 4. Build agent-runner MCP server
Write-Host "[4/8] Building agent-runner MCP server..." -ForegroundColor Yellow
pnpm --filter @mcp-rebuild/agent-runner build

# 5. Sync masday-* skills to local platform directories
Write-Host "[5/8] Syncing masday-* skills to local platform directories..." -ForegroundColor Yellow

# Collect masday-* skill names once
$masdaySkills = Get-ChildItem -Path "$RootDir\.claude\skills" -Directory -Filter "masday-*"

# Clean and recreate platform directories (prevents stale copies)
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

# 6. Install masday-* skills to global ~/.claude/skills/
Write-Host "[6/8] Installing masday-* skills to global $HomeClaude\skills\..." -ForegroundColor Yellow

$globalSkillsDir = Join-Path $HomeClaude "skills"
New-Item -ItemType Directory -Force -Path $globalSkillsDir | Out-Null

$copiedSkills = 0
foreach ($skill in $masdaySkills) {
    $dest = Join-Path $globalSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedSkills++
}

# 7. Convert and install agents to ~/.config/opencode/agent/
Write-Host "[7/8] Converting agents to opencode format and installing to $HomeOpencode\agent\..." -ForegroundColor Yellow

$opencodeAgentsDir = Join-Path $HomeOpencode "agent"
New-Item -ItemType Directory -Force -Path $opencodeAgentsDir | Out-Null

node "$RootDir\scripts\convert-agents.mjs" convert-to-dir "$RootDir\.claude\agents" $opencodeAgentsDir

# 8. Install masday-* skills to ~/.config/opencode/skills/
Write-Host "[8/8] Installing masday-* skills to $HomeOpencode\skills\..." -ForegroundColor Yellow

$opencodeSkillsDir = Join-Path $HomeOpencode "skills"
New-Item -ItemType Directory -Force -Path $opencodeSkillsDir | Out-Null

foreach ($skill in $masdaySkills) {
    $dest = Join-Path $opencodeSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
}

# Summary
Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "MCP server: masday (83 tools, 14 namespaces)"
$agents = (Get-ChildItem "$RootDir\.claude\agents\*.md" -ErrorAction SilentlyContinue).Count
$jsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.js" -ErrorAction SilentlyContinue).Count
$mjsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.mjs" -ErrorAction SilentlyContinue).Count
$opencodeAgents = (Get-ChildItem "$opencodeAgentsDir\masday-*.md" -ErrorAction SilentlyContinue).Count
Write-Host "  Agents:  $agents registered"
Write-Host "  Hooks:   $($jsHooks + $mjsHooks) executable"
Write-Host "  Skills:  $copiedSkills masday-* skills -> $globalSkillsDir"
Write-Host "  Opencode: $opencodeAgents agents + $($masdaySkills.Count) skills -> $opencodeAgentsDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "Start: node $RootDir\apps\agent-runner\dist\runtime\mcp.js"