$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

Write-Host "[1/5] Installing dependencies..." -ForegroundColor Yellow
pnpm install

Write-Host "[2/5] Generating Prisma client..." -ForegroundColor Yellow
pnpm db:generate

Write-Host "[3/5] Building all packages..." -ForegroundColor Yellow
pnpm build

Write-Host "[4/5] Syncing to platform directories..." -ForegroundColor Yellow
$platforms = @(
    @{Dest=".agents\agents"; Src=".claude\agents"},
    @{Dest=".agents\skills"; Src=".claude\skills"},
    @{Dest=".gemini\agents"; Src=".claude\agents"},
    @{Dest=".gemini\skills"; Src=".claude\skills"},
    @{Dest=".continue\agents"; Src=".claude\agents"}
)
foreach ($p in $platforms) {
    New-Item -ItemType Directory -Force -Path $p.Dest | Out-Null
    Copy-Item -Path "$($p.Src)\*" -Destination $p.Dest -Recurse -Force
}

Write-Host "[5/5] Registration summary:" -ForegroundColor Yellow
$agents = (Get-ChildItem ".claude\agents\*.md").Count
$skills = (Get-ChildItem ".claude\skills" -Directory).Count
$jsHooks = (Get-ChildItem ".claude\hooks\*.js" -ErrorAction SilentlyContinue).Count
$mjsHooks = (Get-ChildItem ".claude\hooks\*.mjs" -ErrorAction SilentlyContinue).Count
$executableHooks = $jsHooks + $mjsHooks
$mdHooks = (Get-ChildItem ".claude\hooks\*.md" -ErrorAction SilentlyContinue).Count
Write-Host "  Agents: $agents registered"
Write-Host "  Skills: $skills registered"
Write-Host "  Hooks:  $executableHooks executable + $mdHooks advisory"

Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "MCP servers: workflow-orchestrator(26), memory(9), semantic-search(2), policy(6), capability(10), unified(70)"
Write-Host "Start: npx tsx apps/unified-mcp/src/index.ts"
