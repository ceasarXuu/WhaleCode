$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/artifact-storage.ps1")

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function New-SparseFixture([string]$Path, [int64]$Bytes) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.SetLength($Bytes) } finally { $stream.Dispose() }
}

$root = Join-Path ([IO.Path]::GetTempPath()) "taskspace-artifact-storage-$([guid]::NewGuid().ToString('N'))"
$fixtureRepo = Join-Path $root "repo"
$fixtureTarget = Join-Path $fixtureRepo "target"
$runRoot = Join-Path $fixtureTarget "current-run"
$oldRunLimitBytes = $env:TASKSPACE_MAX_RUN_ARTIFACT_BYTES
$oldRepoLimitBytes = $env:TASKSPACE_MAX_REPO_ARTIFACT_BYTES
try {
    New-SparseFixture (Join-Path $fixtureTarget "debug/current-build.bin") 8MB
    New-SparseFixture (Join-Path $runRoot "result.jsonl") 1MB
    $env:TASKSPACE_MAX_RUN_ARTIFACT_BYTES = ([int64](3MB)).ToString()
    $env:TASKSPACE_MAX_REPO_ARTIFACT_BYTES = ([int64](3MB)).ToString()

    $healthy = New-TaskspaceArtifactStorageHealth $fixtureRepo $runRoot "test_pass"
    Assert-True ([string]$healthy.status -eq "pass") "protected build cache incorrectly consumed benchmark quota"
    Assert-True (@($healthy.repository_usage.protected_entries | Where-Object name -eq "debug").Count -eq 1) "debug build cache was not reported as protected"

    $nested = Join-Path $runRoot "source/third_party/codex-cli/codex-rs/target/debug"
    New-Item -ItemType Directory -Force -Path $nested | Out-Null
    $nestedHealth = New-TaskspaceArtifactStorageHealth $fixtureRepo $runRoot "test_nested"
    Assert-True (@($nestedHealth.findings | Where-Object stable_code -eq "benchmark_nested_build_cache_detected").Count -gt 0) "nested Cargo target was not rejected"
    Remove-Item -Force -Recurse -LiteralPath (Join-Path $runRoot "source")

    New-SparseFixture (Join-Path $runRoot "oversized.bin") 3MB
    $runLimitHealth = New-TaskspaceArtifactStorageHealth $fixtureRepo $runRoot "test_run_limit"
    Assert-True (@($runLimitHealth.findings | Where-Object stable_code -eq "benchmark_run_artifact_limit_exceeded").Count -eq 1) "run artifact limit was not enforced"
    Remove-Item -Force -LiteralPath (Join-Path $runRoot "oversized.bin")

    New-SparseFixture (Join-Path $fixtureTarget "old-run/result.bin") 3MB
    $repoLimitHealth = New-TaskspaceArtifactStorageHealth $fixtureRepo $runRoot "test_repo_limit"
    Assert-True (@($repoLimitHealth.findings | Where-Object stable_code -eq "benchmark_repository_artifact_limit_exceeded").Count -eq 1) "repository artifact limit was not enforced"

    $recentNestedFile = Join-Path $fixtureTarget "recent-tree/nested/result.json"
    New-SparseFixture $recentNestedFile 1
    [IO.Directory]::SetLastWriteTimeUtc((Join-Path $fixtureTarget "recent-tree"), (Get-Date).ToUniversalTime().AddDays(-30))
    $agePlan = New-TaskspaceArtifactCleanupPlan $fixtureTarget @("current-run") 7
    $recentEntry = @($agePlan.entries | Where-Object name -eq "recent-tree")[0]
    Assert-True ([string]$recentEntry.action -eq "retain_recent") "cleanup age ignored a recently updated descendant"

    $dryReport = Join-Path $root "dry-run.json"
    & (Join-Path $PSScriptRoot "cleanup-taskspace-artifacts.ps1") -TargetRoot $fixtureTarget -KeepName "current-run" -ReportPath $dryReport | Out-Null
    Assert-True (Test-Path -LiteralPath (Join-Path $fixtureTarget "old-run")) "dry-run deleted an artifact"
    $dry = Get-Content -Raw -Encoding UTF8 -LiteralPath $dryReport | ConvertFrom-Json
    Assert-True ([string]$dry.mode -eq "plan" -and [int64]$dry.removed_bytes -eq 0) "dry-run report did not remain non-destructive"

    $applyReport = Join-Path $root "apply.json"
    & (Join-Path $PSScriptRoot "cleanup-taskspace-artifacts.ps1") -TargetRoot $fixtureTarget -KeepName "current-run" -ReportPath $applyReport -Apply | Out-Null
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixtureTarget "old-run"))) "apply did not remove selected benchmark artifact"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixtureTarget "debug/current-build.bin")) "apply removed protected build cache"
    Assert-True (Test-Path -LiteralPath $runRoot) "apply removed explicit keep path"

    Write-Output "TaskSpace artifact storage tests passed."
} finally {
    if ($null -eq $oldRunLimitBytes) { Remove-Item Env:TASKSPACE_MAX_RUN_ARTIFACT_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MAX_RUN_ARTIFACT_BYTES = $oldRunLimitBytes }
    if ($null -eq $oldRepoLimitBytes) { Remove-Item Env:TASKSPACE_MAX_REPO_ARTIFACT_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MAX_REPO_ARTIFACT_BYTES = $oldRepoLimitBytes }
    if (Test-Path -LiteralPath $root) { Remove-Item -Force -Recurse -LiteralPath $root }
}
