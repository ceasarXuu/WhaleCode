param(
    [string]$TargetRoot = "",
    [string[]]$KeepName = @(),
    [ValidateRange(0, 36500)][int]$MinimumAgeDays = 0,
    [string]$ReportPath = "",
    [switch]$Apply
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/artifact-storage.ps1")

if ([string]::IsNullOrWhiteSpace($TargetRoot)) { $TargetRoot = Join-Path $repoRoot "target" }
if (-not [IO.Path]::IsPathRooted($TargetRoot)) { $TargetRoot = Join-Path $repoRoot $TargetRoot }
if (-not (Test-Path -LiteralPath $TargetRoot -PathType Container)) { throw "Target root does not exist: $TargetRoot" }
if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $reportRoot = Join-Path $TargetRoot ".storage-reports"
    New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
    $ReportPath = Join-Path $reportRoot ("cleanup-{0}.json" -f (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
} elseif (-not [IO.Path]::IsPathRooted($ReportPath)) {
    $ReportPath = Join-Path $repoRoot $ReportPath
}

$plan = New-TaskspaceArtifactCleanupPlan $TargetRoot $KeepName $MinimumAgeDays
$results = [Collections.Generic.List[object]]::new()
foreach ($entry in @($plan.entries)) {
    $status = if ([string]$entry.action -eq "remove") { if ($Apply) { "pending" } else { "planned" } } else { "unchanged" }
    $errorMessage = ""
    if ($Apply -and [string]$entry.action -eq "remove") {
        try {
            Remove-Item -Force -Recurse -LiteralPath ([string]$entry.path)
            $status = "removed"
        } catch {
            $status = "failed"
            $errorMessage = [string]$_.Exception.Message
        }
    }
    $results.Add([pscustomobject]@{
            name = [string]$entry.name
            path = [string]$entry.path
            bytes = [int64]$entry.bytes
            action = [string]$entry.action
            status = $status
            reason = [string]$entry.reason
            error = $errorMessage
        })
}

$failed = @($results | Where-Object status -eq "failed")
$removedBytes = [int64](($results | Where-Object status -eq "removed" | Measure-Object bytes -Sum).Sum)
$report = [pscustomobject]@{
    schema_version = 1
    mode = if ($Apply) { "apply" } else { "plan" }
    status = if ($failed.Count -gt 0) { "partial_failure" } else { "ok" }
    target_root = $plan.target_root
    minimum_age_days = $MinimumAgeDays
    keep_names = @($KeepName)
    removable_bytes = [int64]$plan.removable_bytes
    removed_bytes = $removedBytes
    entries = @($results.ToArray())
    generated_at = (Get-Date).ToString("o")
}
$parent = Split-Path -Parent $ReportPath
New-Item -ItemType Directory -Force -Path $parent | Out-Null
[IO.File]::WriteAllText($ReportPath, (($report | ConvertTo-Json -Depth 20) + "`n"), [Text.UTF8Encoding]::new($false))

Write-Output "ArtifactCleanupMode: $($report.mode)"
Write-Output "ArtifactCleanupStatus: $($report.status)"
Write-Output "ArtifactCleanupRemovableBytes: $($report.removable_bytes)"
Write-Output "ArtifactCleanupRemovedBytes: $($report.removed_bytes)"
Write-Output "ArtifactCleanupReport: $ReportPath"
if ($failed.Count -gt 0) { exit 1 }
