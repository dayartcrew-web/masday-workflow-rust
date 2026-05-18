$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

Write-Host "[1/6] Installing dependencies..." -ForegroundColor Yellow
pnpm install

Write-Host "[2/6] Generating Prisma client..." -ForegroundColor Yellow
pnpm db:generate

Write-Host "[3/6] Building all packages..." -ForegroundColor Yellow
pnpm build

Write-Host "[4/6] Building agent-runner MCP server..." -ForegroundColor Yellow
pnpm --filter @mcp-rebuild/agent-runner build

Write-Host "[5/6] Syncing to platform directories..." -ForegroundColor Yellow
$platforms = @(
    @{Dest=".agents\agents"; Src=".claude\agents"},
    @{Dest=".gemini\agents"; Src=".claude\agents"},
    @{Dest=".continue\agents"; Src=".claude\agents"}
)
foreach ($p in $platforms) {
    New-Item -ItemType Directory -Force -Path $p.Dest | Out-Null
    Copy-Item -Path "$($p.Src)\*" -Destination $p.Dest -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "[6/6] Registration summary:" -ForegroundColor Yellow
$agents = (Get-ChildItem ".claude\agents\*.md").Count
$jsHooks = (Get-ChildItem ".claude\hooks\*.js" -ErrorAction SilentlyContinue).Count
$mjsHooks = (Get-ChildItem ".claude\hooks\*.mjs" -ErrorAction SilentlyContinue).Count
$executableHooks = $jsHooks + $mjsHooks
$mdHooks = (Get-ChildItem ".claude\hooks\*.md" -ErrorAction SilentlyContinue).Count
Write-Host "  Agents: $agents registered"
Write-Host "  Hooks:  $executableHooks executable + $mdHooks advisory"

Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
Write-Host "Start: node C:/Users/AQR STD/Documents/GitHub/vibe-masday-workflow/masday-workflow-rebuild/apps/agent-runner/dist/runtime/mcp.js"
