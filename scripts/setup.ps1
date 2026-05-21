$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$HomeClaude = Join-Path $env:USERPROFILE ".claude"
$HomeOpencode = Join-Path (Join-Path $env:USERPROFILE ".config") "opencode"

Write-Host "=== masday-workflow-rebuild Setup ===" -ForegroundColor Cyan

# 1. Install dependencies
Write-Host "[1/9] Installing dependencies..." -ForegroundColor Yellow
pnpm install

# 2. Generate Prisma client (skip if client exists and MCP server may be running)
Write-Host "[2/9] Generating Prisma client..." -ForegroundColor Yellow
$prismaClient = Get-ChildItem -Path "$RootDir\node_modules\.pnpm\@prisma+client*" -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
if ($prismaClient -and (Test-Path (Join-Path $prismaClient.FullName "node_modules\.prisma\client\index.js"))) {
    Write-Host "  Prisma client already exists, skipping (run 'pnpm db:generate' manually to update)" -ForegroundColor Gray
} else {
    pnpm db:generate
}

# 3. Build all packages
Write-Host "[3/9] Building all packages..." -ForegroundColor Yellow
pnpm build

# 4. Build agent-runner MCP server
Write-Host "[4/9] Building agent-runner MCP server..." -ForegroundColor Yellow
pnpm --filter @mcp-rebuild/agent-runner build

# 4b. pgvector setup (PostgreSQL only — skipped for sqlite://local)
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

# 5. Sync masday-* skills to local platform directories
Write-Host "[5/9] Syncing masday-* skills to local platform directories..." -ForegroundColor Yellow

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

# 6. Install masday-* skills to global ~/.claude/skills/
Write-Host "[6/9] Installing masday-* skills to global $HomeClaude\skills\..." -ForegroundColor Yellow

$globalSkillsDir = Join-Path $HomeClaude "skills"
New-Item -ItemType Directory -Force -Path $globalSkillsDir | Out-Null

$copiedSkills = 0
foreach ($skill in $masdaySkills) {
    $dest = Join-Path $globalSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
    $copiedSkills++
}

# 7. Convert and install agents to ~/.config/opencode/agent/ AND project .opencode/agent/
Write-Host "[7/9] Converting agents to opencode format (global + project)..." -ForegroundColor Yellow
$opencodeAgentsDir = Join-Path $HomeOpencode "agent"
$projectAgentsDir = "$RootDir\.opencode\agent"
New-Item -ItemType Directory -Force -Path $opencodeAgentsDir | Out-Null
New-Item -ItemType Directory -Force -Path $projectAgentsDir | Out-Null
node "$RootDir\scripts\convert-agents.mjs" convert "$RootDir\.claude\agents"
$opencodeAgents = (Get-ChildItem "$opencodeAgentsDir\masday-*.md" -ErrorAction SilentlyContinue).Count
$projAgents = (Get-ChildItem "$projectAgentsDir\masday-*.md" -ErrorAction SilentlyContinue).Count
Write-Host "  Opencode: $opencodeAgents global + $projAgents project agents" -ForegroundColor Cyan

# 8. Install masday-* skills to ~/.config/opencode/skills/
Write-Host "[8/9] Installing masday-* skills to $HomeOpencode\skills\..." -ForegroundColor Yellow
$opencodeSkillsDir = Join-Path $HomeOpencode "skills"
New-Item -ItemType Directory -Force -Path $opencodeSkillsDir | Out-Null

foreach ($skill in $masdaySkills) {
    $dest = Join-Path $opencodeSkillsDir $skill.Name
    if (Test-Path $dest) { Remove-Item -Path $dest -Recurse -Force }
    Copy-Item -Path $skill.FullName -Destination $dest -Recurse -Force
}

# 9. Ensure .masday/ state directories exist (used by tdd-guard hook)
New-Item -ItemType Directory -Force -Path "$RootDir\.masday\cache\tasks" | Out-Null
New-Item -ItemType Directory -Force -Path "$RootDir\.masday\reports" | Out-Null

# 10. Summary
Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host "MCP server: masday (87 tools, 16 namespaces)"
$agents = (Get-ChildItem "$RootDir\.claude\agents\*.md" -ErrorAction SilentlyContinue).Count
$jsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.js" -ErrorAction SilentlyContinue).Count
$mjsHooks = (Get-ChildItem "$RootDir\.claude\hooks\*.mjs" -ErrorAction SilentlyContinue).Count
Write-Host "  Agents:  $agents registered"
Write-Host "  Hooks:   $($jsHooks + $mjsHooks) executable"
Write-Host "  TDD guard: workflow-aware (requiresTdd tasks blocked without tests)"
Write-Host "  Skills:  $copiedSkills masday-* skills -> $globalSkillsDir"
Write-Host "  Opencode: $opencodeAgents global + $projAgents project agents" -ForegroundColor Cyan
Write-Host "  Embedding: EMBEDDING_PROVIDER=$($env:EMBEDDING_PROVIDER ?? 'fastembed') (fastembed|ollama|openai)"
Write-Host "  Vector search: pnpm db:pgvector (PostgreSQL only; skipped for sqlite://local)"
Write-Host ""
Write-Host "Start: node $RootDir\apps\agent-runner\dist\runtime\mcp.js"