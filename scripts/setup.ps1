$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$HomeClaude = Join-Path $env:USERPROFILE ".claude"

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

# 1. Install dependencies
Write-Host "[1/7] Installing dependencies..." -ForegroundColor Yellow
pnpm install

# 2. Generate Prisma client
Write-Host "[2/7] Generating Prisma client..." -ForegroundColor Yellow
pnpm db:generate

# 3. Build all packages
Write-Host "[3/7] Building all packages..." -ForegroundColor Yellow
pnpm build

# 4. Build agent-runner MCP server
Write-Host "[4/7] Building agent-runner MCP server..." -ForegroundColor Yellow
pnpm --filter @mcp-rebuild/agent-runner build

# 5. Sync commands into .claude/commands/ from .agents/commands/
Write-Host "[5/7] Syncing commands to .claude/commands/..." -ForegroundColor Yellow
$claudeCommandsDir = Join-Path $RootDir ".claude\commands"
New-Item -ItemType Directory -Force -Path $claudeCommandsDir | Out-Null
Copy-Item -Path "$RootDir\.agents\commands\*" -Destination $claudeCommandsDir -Recurse -Force -ErrorAction SilentlyContinue

# 6. Sync to local platform directories
Write-Host "[6/7] Syncing to local platform directories..." -ForegroundColor Yellow
$platforms = @(
    @{Dest = ".agents\agents"; Src = ".claude\agents" },
    @{Dest = ".agents\skills"; Src = ".claude\skills" },
    @{Dest = ".agents\commands"; Src = ".claude\commands" },
    @{Dest = ".gemini\agents"; Src = ".claude\agents" },
    @{Dest = ".gemini\skills"; Src = ".claude\skills" },
    @{Dest = ".gemini\commands"; Src = ".claude\commands" },
    @{Dest = ".continue\agents"; Src = ".claude\agents" },
    @{Dest = ".continue\skills"; Src = ".claude\skills" },
    @{Dest = ".continue\commands"; Src = ".claude\commands" }
)
foreach ($p in $platforms) {
    New-Item -ItemType Directory -Force -Path $p.Dest | Out-Null
    Copy-Item -Path "$($p.Src)\*" -Destination $p.Dest -Recurse -Force -ErrorAction SilentlyContinue
}

# 7. Install masday-* skills to global ~/.claude/skills/
Write-Host "[7/7] Installing masday-* skills to global $HomeClaude\skills\..." -ForegroundColor Yellow

$globalSkillsDir = Join-Path $HomeClaude "skills"
New-Item -ItemType Directory -Force -Path $globalSkillsDir | Out-Null

$projectSkills = Get-ChildItem -Path "$RootDir\.claude\skills" -Directory -Filter "masday-*"
$copiedSkills = 0
foreach ($skill in $projectSkills) {
    $dest = Join-Path $globalSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedSkills++
}

# Summary
Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
$agents = (Get-ChildItem "$RootDir\.claude\agents\*.md" -ErrorAction SilentlyContinue).Count
$jsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.js" -ErrorAction SilentlyContinue).Count
$mjsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.mjs" -ErrorAction SilentlyContinue).Count
Write-Host "  Agents:  $agents registered"
Write-Host "  Hooks:   $($jsHooks + $mjsHooks) executable"
Write-Host "  Skills:  $copiedSkills masday-* skills -> $globalSkillsDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "Start: node $RootDir\apps\agent-runner\dist\runtime\mcp.js"
