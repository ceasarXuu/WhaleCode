param(
    [string]$TestFilter = "realistic_user_bugfix_runs_agent_actions_with_action_map",
    [string]$CodexRsRoot = "",
    [string]$TargetDir = "",
    [string]$ReportRoot = "",
    [int]$Jobs = 2,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

function Resolve-FullPath([string]$PathValue) {
    $created = New-Item -ItemType Directory -Force -Path $PathValue
    return $created.FullName
}

function Select-LatestScenarioReport([string]$CodexRsRootValue, [string]$ScenarioId, [datetime]$StartedAt) {
    $scenarioRoot = Join-Path $CodexRsRootValue "target\scenario-runs\$ScenarioId"
    if (-not (Test-Path $scenarioRoot)) {
        return $null
    }
    $latestRun = Get-ChildItem -Path $scenarioRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latestRun) {
        return $null
    }
    $report = Join-Path $latestRun.FullName "artifacts\report.md"
    if ((Test-Path $report) -and (Get-Item $report).LastWriteTime -ge $StartedAt.AddSeconds(-2)) {
        return Get-Item $report
    }
    return $null
}

function New-TestSummary($Lines) {
    $results = @()
    foreach ($line in $Lines) {
        if ($line -match "test result: (?<status>[^.]+)\. (?<passed>\d+) passed; (?<failed>\d+) failed; (?<ignored>\d+) ignored; (?<measured>\d+) measured; (?<filtered>\d+) filtered out; finished in (?<duration>.+)$") {
            $results += [pscustomobject]@{
                Status = $Matches.status.Trim()
                Passed = [int]$Matches.passed
                Failed = [int]$Matches.failed
                Ignored = [int]$Matches.ignored
                Measured = [int]$Matches.measured
                Filtered = [int]$Matches.filtered
                Duration = $Matches.duration.Trim()
                Raw = $line
            }
        }
    }
    return $results
}

function Select-FailureLines($Lines) {
    $selected = @()
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        $line = [string]$Lines[$i]
        if ($line -cmatch "FAILED|failures:|error: test failed|panicked at|thread '.*' panicked") {
            $start = [Math]::Max(0, $i - 3)
            $end = [Math]::Min($Lines.Count - 1, $i + 12)
            for ($j = $start; $j -le $end; $j++) {
                $selected += [string]$Lines[$j]
            }
            $selected += ""
        }
    }
    return $selected | Select-Object -Unique
}

if ([string]::IsNullOrWhiteSpace($CodexRsRoot)) {
    $CodexRsRoot = Join-Path $PSScriptRoot "..\third_party\codex-cli\codex-rs"
}
$CodexRsRoot = (Resolve-Path $CodexRsRoot).Path

if ([string]::IsNullOrWhiteSpace($TargetDir)) {
    $TargetDir = Join-Path $PSScriptRoot "..\target-test"
}
$TargetDir = Resolve-FullPath $TargetDir

if ([string]::IsNullOrWhiteSpace($ReportRoot)) {
    $ReportRoot = Join-Path $PSScriptRoot "..\target\test-reports"
}
$ReportRoot = Resolve-FullPath $ReportRoot

$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = Resolve-FullPath (Join-Path $ReportRoot "action-map-e2e-$stamp")
$stdoutPath = Join-Path $runDir "cargo-test.stdout.log"
$stderrPath = Join-Path $runDir "cargo-test.stderr.log"
$logPath = Join-Path $runDir "cargo-test.log"
$reportPath = Join-Path $runDir "report.md"
$commandArgs = @(
    "run", "stable", "cargo", "test",
    "-p", "codex-core",
    "--test", "all",
    $TestFilter,
    "--locked",
    "--jobs", [string]$Jobs
)
$commandText = "rustup $($commandArgs -join ' ')"

if ($PlanOnly) {
    Write-Host "CodexRsRoot: $CodexRsRoot"
    Write-Host "TargetDir: $TargetDir"
    Write-Host "ReportPath: $reportPath"
    Write-Host "Command: $commandText"
    exit 0
}

$oldTargetDir = $env:CARGO_TARGET_DIR
$oldBuildJobs = $env:CARGO_BUILD_JOBS
$env:CARGO_TARGET_DIR = $TargetDir
$env:CARGO_BUILD_JOBS = [string]$Jobs
$started = Get-Date

try {
    $process = Start-Process `
        -FilePath "rustup" `
        -ArgumentList $commandArgs `
        -WorkingDirectory $CodexRsRoot `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -NoNewWindow `
        -Wait `
        -PassThru
    $exitCode = $process.ExitCode
}
finally {
    $env:CARGO_TARGET_DIR = $oldTargetDir
    $env:CARGO_BUILD_JOBS = $oldBuildJobs
}

$finished = Get-Date
$stdoutText = if (Test-Path $stdoutPath) { Get-Content -Raw -Encoding UTF8 $stdoutPath } else { "" }
$stderrText = if (Test-Path $stderrPath) { Get-Content -Raw -Encoding UTF8 $stderrPath } else { "" }
$lines = @(($stdoutText, $stderrText) -join [Environment]::NewLine -split "`r?`n")
$lines | Set-Content -Encoding UTF8 $logPath

$summaries = @(New-TestSummary $lines)
$failureLines = @(Select-FailureLines $lines)
$passedCount = ($summaries | Measure-Object -Property Passed -Sum).Sum
if ($null -eq $passedCount) { $passedCount = 0 }
$failedCount = ($summaries | Measure-Object -Property Failed -Sum).Sum
if ($null -eq $failedCount) { $failedCount = 0 }
$scenarioReport = Select-LatestScenarioReport $CodexRsRoot "action-map-realistic-user-bugfix" $started
$overall = if ($exitCode -eq 0 -and $failedCount -eq 0 -and $passedCount -gt 0 -and $scenarioReport) { "PASS" } else { "FAIL" }

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map E2E Scenario Report")
$report.Add("")
$report.Add("- overall: $overall")
$report.Add("- exit_code: $exitCode")
$report.Add("- command: $commandText")
$report.Add("- cwd: $CodexRsRoot")
$report.Add("- target_dir: $TargetDir")
$report.Add("- started: $($started.ToString("o"))")
$report.Add("- finished: $($finished.ToString("o"))")
$report.Add("- log: $logPath")
$report.Add("- total_passed_tests: $passedCount")
$report.Add("- total_failed_tests: $failedCount")
$report.Add("- scenario_report: $($scenarioReport.FullName)")
$report.Add("")
$report.Add("## Test Result Lines")
$report.Add("")
if ($summaries.Count -eq 0) {
    $report.Add("No test result lines were parsed. Inspect the full log.")
} else {
    $report.Add("| status | passed | failed | ignored | measured | filtered | duration |")
    $report.Add("|---|---:|---:|---:|---:|---:|---|")
    foreach ($summary in $summaries) {
        $report.Add("| $($summary.Status) | $($summary.Passed) | $($summary.Failed) | $($summary.Ignored) | $($summary.Measured) | $($summary.Filtered) | $($summary.Duration) |")
    }
}
$report.Add("")
$report.Add("## Failure Extract")
$report.Add("")
if ($failureLines.Count -eq 0) {
    $report.Add("No failure markers were found in the cargo output.")
} else {
    $report.Add("~~~text")
    $report.AddRange([string[]]$failureLines)
    $report.Add("~~~")
}

$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "Log: $logPath"
if ($scenarioReport) {
    Write-Host "ScenarioReport: $($scenarioReport.FullName)"
}
Write-Host "Overall: $overall"
if ($overall -ne "PASS" -and $exitCode -eq 0) {
    exit 1
}
exit $exitCode
