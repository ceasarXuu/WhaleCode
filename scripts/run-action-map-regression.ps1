param(
    [string]$TestFilter = "action_map",
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

function Escape-Markdown([string]$Value) {
    if ($null -eq $Value) {
        return ""
    }
    return $Value.Replace("|", "\|")
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
        if ($line -match "FAILED|failures:|error: test failed|panicked at|thread '.*' panicked") {
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

function Select-RelevantCrashEvents([datetime]$StartTime, [datetime]$EndTime) {
    $events = Get-WinEvent -FilterHashtable @{
        LogName = "Application"
        StartTime = $StartTime
        EndTime = $EndTime
        Level = 2
    } -ErrorAction SilentlyContinue

    return $events | Where-Object {
        $_.Message -match "rustc\.exe|cargo\.exe|rustup\.exe|Astropath\.exe|codex-command-runner|PowerShell"
    } | Select-Object -First 20 TimeCreated, ProviderName, Id, Message
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

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Resolve-FullPath (Join-Path $ReportRoot "action-map-$stamp")
$logPath = Join-Path $runDir "cargo-test.log"
$reportPath = Join-Path $runDir "report.md"
$commandText = "rustup run stable cargo test --lib $TestFilter --locked --jobs $Jobs"

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
Push-Location $CodexRsRoot
try {
    $output = & rustup run stable cargo test --lib $TestFilter --locked --jobs $Jobs 2>&1
    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
    $env:CARGO_TARGET_DIR = $oldTargetDir
    $env:CARGO_BUILD_JOBS = $oldBuildJobs
}
$finished = Get-Date

$lines = @($output | ForEach-Object { $_.ToString() })
$lines | Set-Content -Encoding UTF8 $logPath

$summaries = @(New-TestSummary $lines)
$failureLines = @(Select-FailureLines $lines)
$crashEvents = @(Select-RelevantCrashEvents $started.AddSeconds(-5) $finished.AddSeconds(5))
$failedCount = ($summaries | Measure-Object -Property Failed -Sum).Sum
if ($null -eq $failedCount) {
    $failedCount = 0
}
$passedCount = ($summaries | Measure-Object -Property Passed -Sum).Sum
if ($null -eq $passedCount) {
    $passedCount = 0
}
$matchedBinaries = @($summaries | Where-Object { $_.Passed -gt 0 -or $_.Failed -gt 0 -or $_.Ignored -gt 0 })

$overall = if ($exitCode -eq 0 -and $failedCount -eq 0) { "PASS" } else { "FAIL" }

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map Regression Report")
$report.Add("")
$report.Add("- overall: $overall")
$report.Add("- exit_code: $exitCode")
$report.Add("- command: $commandText")
$report.Add("- cwd: $CodexRsRoot")
$report.Add("- target_dir: $TargetDir")
$report.Add("- started: $($started.ToString("o"))")
$report.Add("- finished: $($finished.ToString("o"))")
$report.Add("- log: $logPath")
$report.Add("- matched_test_binaries: $($matchedBinaries.Count)")
$report.Add("- total_passed_tests: $passedCount")
$report.Add("- total_failed_tests: $failedCount")
$report.Add("- relevant_crash_events: $($crashEvents.Count)")
$report.Add("")
$report.Add("## Test Result Lines")
$report.Add("")
if ($summaries.Count -eq 0) {
    $report.Add("No test result lines were parsed. Inspect the full log.")
} else {
    $report.Add("| status | passed | failed | ignored | measured | filtered | duration |")
    $report.Add("|---|---:|---:|---:|---:|---:|---|")
    foreach ($summary in $summaries) {
        $report.Add("| $(Escape-Markdown $summary.Status) | $($summary.Passed) | $($summary.Failed) | $($summary.Ignored) | $($summary.Measured) | $($summary.Filtered) | $(Escape-Markdown $summary.Duration) |")
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
$report.Add("")
$report.Add("## Relevant Windows Crash Events")
$report.Add("")
if ($crashEvents.Count -eq 0) {
    $report.Add("No relevant Application Error events were found during this run window.")
} else {
    foreach ($event in $crashEvents) {
        $firstLine = (($event.Message -split "`r?`n") | Select-Object -First 1)
        $report.Add("- $($event.TimeCreated.ToString("o")) [$($event.ProviderName)#$($event.Id)] $firstLine")
    }
}
$report.Add("")
$report.Add("## Notes")
$report.Add("")
$report.Add("- Lines with 0 passed; 0 failed; ... filtered out are normal for crates that do not contain tests matching the filter.")
$report.Add("- Treat exit_code = 0 plus no failure markers as the authoritative pass signal for this filtered cargo run.")

$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "Log: $logPath"
Write-Host "Overall: $overall"
exit $exitCode
