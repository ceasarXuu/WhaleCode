param(
    [string]$ScenarioId = "real-model-bugfix",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "",
    [switch]$Launch
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $PSScriptRoot "..\target\scenario-runs"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $RunRoot "$ScenarioId\$stamp"
$repoDir = Join-Path $runDir "repo"
$whaleHome = Join-Path $runDir "whale-home"
$artifacts = Join-Path $runDir "artifacts"

New-Item -ItemType Directory -Force -Path $repoDir, $whaleHome, $artifacts | Out-Null

$srcDir = Join-Path $repoDir "src"
$testDir = Join-Path $repoDir "tests"
New-Item -ItemType Directory -Force -Path $srcDir, $testDir | Out-Null

@'
def cache_key(namespace, key):
    return f"{namespace}:{key.lower()}"
'@ | Set-Content -Encoding UTF8 (Join-Path $srcDir "cache.py")

@'
from src.cache import cache_key

def test_cache_key_normalizes_key():
    assert cache_key("Users", "ABC") == "Users:abc"

def test_cache_key_normalizes_namespace():
    assert cache_key("Users", "ABC") == "users:abc"
'@ | Set-Content -Encoding UTF8 (Join-Path $testDir "test_cache.py")

@'
# Exploratory Action Map Scenario

This sandbox intentionally contains a small failing cache-key regression.
'@ | Set-Content -Encoding UTF8 (Join-Path $repoDir "README.md")

$scenarioPrompt = @(
    "Solve this sandbox project in Action Map experiment mode:",
    "1. Confirm scope and failing evidence first.",
    "2. Use multi-agent only when useful; every subagent must be bound to an Action Map node.",
    "3. Fix the cache-key test failure.",
    "4. Keep or add a regression test for the bug.",
    "5. Report the validation commands that were run."
) -join [Environment]::NewLine
$promptPath = Join-Path $artifacts "prompt.txt"
$scenarioPrompt | Set-Content -Encoding UTF8 $promptPath

$env:WHALE_HOME = (Resolve-Path $whaleHome).Path
Remove-Item Env:\CODEX_HOME -ErrorAction SilentlyContinue

$commandLine = @(
    "`"$WhaleBin`"",
    "-C", "`"$repoDir`"",
    "--skip-git-repo-check"
)
if (-not [string]::IsNullOrWhiteSpace($Model)) {
    $commandLine += @("-m", "`"$Model`"")
}

$instructions = @(
    "Run directory: $runDir",
    "Repo: $repoDir",
    "WHALE_HOME: $whaleHome",
    "",
    "Manual exploratory path:",
    "1. Start Whale:",
    "   $($commandLine -join ' ')",
    "2. In the TUI, run:",
    "   /taskspace",
    "3. Paste the prompt from:",
    "   $promptPath",
    "4. After Whale exits, rerun this script without -Launch only if you need a fresh sandbox.",
    "",
    "This path is intentionally not CI-stable and uses your local real-model configuration."
) -join [Environment]::NewLine
$instructions | Set-Content -Encoding UTF8 (Join-Path $artifacts "report.md")

if ($Launch) {
    if (-not (Test-Path $WhaleBin)) {
        throw "Whale binary not found: $WhaleBin"
    }
    Push-Location $repoDir
    try {
        & $WhaleBin --skip-git-repo-check
    }
    finally {
        Pop-Location
    }
}

$rollouts = Get-ChildItem -Path $whaleHome -Recurse -Filter "rollout-*.jsonl" -ErrorAction SilentlyContinue
if ($rollouts) {
    $latest = $rollouts | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    Copy-Item -LiteralPath $latest.FullName -Destination (Join-Path $artifacts "rollout.jsonl") -Force
}

Write-Host $instructions
