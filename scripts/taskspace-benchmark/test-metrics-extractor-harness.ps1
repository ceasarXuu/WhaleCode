param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\metrics-extractor-selftest" }
$runDir = New-Dir (Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function New-TestFile([string]$Path, [string]$Text = "artifact") {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    Set-Content -LiteralPath $Path -Encoding UTF8 -Value $Text
}

$repo = New-Dir (Join-Path $runDir "repo")
Push-Location $repo
try {
    git init | Out-Null
    git config user.email "taskspace-test@example.local" | Out-Null
    git config user.name "TaskSpace Test" | Out-Null
    New-TestFile (Join-Path $repo "src\main.py") "print('base')"
    git add . | Out-Null
    git commit -m "base" | Out-Null
    New-TestFile (Join-Path $repo "src\main.py") "print('changed')"
    New-TestFile (Join-Path $repo ".tbench-testing\lib\python3.11\site-packages\pyarrow\ignored.py") "ignored"
    New-TestFile (Join-Path $repo ".tbench-testing\external-validator-source\probe.txt") "ignored"
    New-TestFile (Join-Path $repo "notes.txt") "real untracked"
} finally {
    Pop-Location
}

$diffPath = Join-Path $runDir "git-diff.patch"
$diffText = Get-TaskspaceDiffText $repo $diffPath
$inventory = @(Get-TaskspaceChangedFileInventory $repo $diffText)
$paths = @($inventory | ForEach-Object { [string]$_.path })
Assert-True ($paths -contains "src/main.py") "tracked source change was not reported"
Assert-True ($paths -contains "notes.txt") "real untracked file was not reported"
Assert-True (@($paths | Where-Object { $_ -like ".tbench-testing/*" }).Count -eq 0) "runtime .tbench-testing files leaked into changed inventory"
Assert-True (@($paths | Where-Object { $_ -like "*external-validator-source*" }).Count -eq 0) "ignored runtime validator-looking files leaked into changed inventory"

$artifactDir = New-Dir (Join-Path $runDir "large-rollout-artifacts")
$jsonlPath = Join-Path $artifactDir "whale-exec.jsonl"
New-TestFile $jsonlPath ""
$rolloutPath = Join-Path $artifactDir "rollout.jsonl"
[System.IO.File]::WriteAllText($rolloutPath, ("x" * 2097152), [System.Text.UTF8Encoding]::new($false))
$oldThreshold = $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES
try {
    $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES = "1048576"
    $cost = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $artifactDir -JsonlPath $jsonlPath -ObservabilityJsonPath ""
} finally {
    if ($null -eq $oldThreshold) { Remove-Item Env:\TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES = $oldThreshold }
}
Assert-True ([string]$cost.cost_scan_policy.rollout_scan_mode -eq "skipped_large_rollout") "large rollout was not guarded by cost scan policy"
Assert-True (Test-Path -LiteralPath $cost.cost_scan_policy_path) "cost scan policy artifact was not written"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace metrics extractor harness self-test: FAIL"
    $failures | ForEach-Object { Write-Host " - $_" }
    Write-Host "RunRoot: $runDir"
    exit 1
}

Write-Host "TaskSpace metrics extractor harness self-test: PASS"
Write-Host "RunRoot: $runDir"
