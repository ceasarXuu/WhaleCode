param(
    [string]$TestFilter = "action_map",
    [string[]]$Package = @("codex-core"),
    [string]$CodexRsRoot = "",
    [string]$TargetDir = "",
    [string]$ReportRoot = "",
    [int]$Jobs = 2,
    [switch]$SkipScriptTests,
    [switch]$IncludeTuiViewerE2E,
    [int]$TuiViewerTimeoutSeconds = 180,
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

function Read-KeyValueReport([string]$PathValue) {
    $values = @{}
    if (-not (Test-Path $PathValue)) {
        return $values
    }

    foreach ($line in Get-Content -Encoding UTF8 $PathValue) {
        if ($line -match "^- (?<key>[A-Za-z0-9_]+): (?<value>.*)$") {
            $values[$Matches.key] = $Matches.value.Trim()
        }
    }
    return $values
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

function Select-RelevantCrashEvents([datetime]$StartTime, [datetime]$EndTime) {
    $events = Get-WinEvent -FilterHashtable @{
        LogName = "Application"
        StartTime = $StartTime
        EndTime = $EndTime
        Level = 2
    } -ErrorAction SilentlyContinue

    return $events | Where-Object {
        $_.Message -match "rustc\.exe|cargo\.exe|rustup\.exe|whale\.exe|node\.exe|chrome\.exe|msedge\.exe|codex-command-runner|PowerShell"
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

$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = Resolve-FullPath (Join-Path $ReportRoot "action-map-$stamp")
$logPath = Join-Path $runDir "cargo-test.log"
$reportPath = Join-Path $runDir "report.md"

function New-CargoTestRun(
    [string]$Name,
    [string[]]$Packages,
    [string]$Filter,
    [string]$TestTarget = ""
) {
    [pscustomobject]@{
        Name = $Name
        Package = [string[]]$Packages
        TestFilter = $Filter
        TestTarget = $TestTarget
    }
}

function Get-PackageArgs([string[]]$Packages) {
    $args = @()
    foreach ($packageName in $Packages) {
        if (-not [string]::IsNullOrWhiteSpace($packageName)) {
            $args += @("-p", $packageName)
        }
    }
    return $args
}

function Get-CargoArgumentList($Run) {
    $packageArgs = Get-PackageArgs $Run.Package
    $args = @("run", "stable", "cargo", "test") + $packageArgs
    if ([string]::IsNullOrWhiteSpace($Run.TestTarget)) {
        $args += "--lib"
    } else {
        $args += @("--test", $Run.TestTarget)
    }
    if (-not [string]::IsNullOrWhiteSpace($Run.TestFilter)) {
        $args += $Run.TestFilter
    }
    return $args + @("--locked", "--jobs", [string]$Jobs)
}

function Get-CargoCommandText($Run) {
    return "rustup $((Get-CargoArgumentList $Run) -join ' ')"
}

$useDefaultMatrix = -not $PSBoundParameters.ContainsKey("Package") -and -not $PSBoundParameters.ContainsKey("TestFilter")
if ($useDefaultMatrix) {
    $testRuns = @(
        New-CargoTestRun "core-action-map" @("codex-core") "action_map"
        New-CargoTestRun "core-taskspace-trace" @("codex-core") "taskspace_trace"
        New-CargoTestRun "core-session-standard-trace" @("codex-core") "session_standard_mode_main_tool_result_does_not_record_trace"
        New-CargoTestRun "protocol-action-map-snapshot" @("codex-protocol") "action_map_snapshot"
        New-CargoTestRun "protocol-sentinel-warning" @("codex-protocol") "sentinel_warning_raised"
        New-CargoTestRun "app-server-schema-fixtures" @("codex-app-server-protocol") "" "schema_fixtures"
        New-CargoTestRun "core-legacy-spawn-agent" @("codex-core") "legacy_spawn_agent"
        New-CargoTestRun "tools-spawn-agent" @("codex-tools") "spawn_agent"
        New-CargoTestRun "tools-multi-agent-task-names" @("codex-tools") "multi_agent_v2_uses_task_names"
        New-CargoTestRun "tools-registry-plan" @("codex-tools") "tool_registry_plan"
    )
} else {
    $testRuns = @(New-CargoTestRun "custom" $Package $TestFilter)
}
$commandText = ($testRuns | ForEach-Object { Get-CargoCommandText $_ }) -join "; "
$scriptTestRuns = if ($SkipScriptTests) {
    @()
} else {
    $scriptRuns = @(
        "test-action-map-graph-health.ps1",
        "test-action-map-observability-lib.ps1",
        "test-action-map-sentinel-clearance.ps1",
        "test-action-map-reparse-containment.ps1",
        "test-action-map-real-user-e2e-lib.ps1",
        "test-r7-map-store-reference-gate.ps1"
    )
    if ($IncludeTuiViewerE2E) {
        $scriptRuns += "run-tui-taskspace-viewer-e2e.ps1"
    }
    $scriptRuns
}
$scriptCommandText = ($scriptTestRuns | ForEach-Object {
    if ($_ -eq "run-tui-taskspace-viewer-e2e.ps1") {
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\$_ -TimeoutSeconds $TuiViewerTimeoutSeconds"
    } else {
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\$_"
    }
}) -join "; "

if ($PlanOnly) {
    Write-Host "CodexRsRoot: $CodexRsRoot"
    Write-Host "TargetDir: $TargetDir"
    Write-Host "ReportPath: $reportPath"
    Write-Host "Runs:"
    foreach ($testRun in $testRuns) {
        Write-Host "- $($testRun.Name): $(Get-CargoCommandText $testRun)"
    }
    if ($scriptTestRuns.Count -gt 0) {
        Write-Host "ScriptRuns:"
        foreach ($scriptName in $scriptTestRuns) {
            Write-Host "- $scriptName"
        }
    }
    exit 0
}

$started = Get-Date

$oldTargetDir = $env:CARGO_TARGET_DIR
$oldBuildJobs = $env:CARGO_BUILD_JOBS
$env:CARGO_TARGET_DIR = $TargetDir
$env:CARGO_BUILD_JOBS = [string]$Jobs

$runResults = @()
try {
    foreach ($testRun in $testRuns) {
        $safeName = $testRun.Name -replace "[^A-Za-z0-9_.-]", "-"
        $stdoutPath = Join-Path $runDir "cargo-test-$safeName.stdout.log"
        $stderrPath = Join-Path $runDir "cargo-test-$safeName.stderr.log"
        $runStarted = Get-Date
        $argumentList = Get-CargoArgumentList $testRun
        $process = Start-Process `
            -FilePath "rustup" `
            -ArgumentList $argumentList `
            -WorkingDirectory $CodexRsRoot `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath `
            -NoNewWindow `
            -Wait `
            -PassThru
        $runFinished = Get-Date

        $stdoutText = if (Test-Path $stdoutPath) { Get-Content -Raw -Encoding UTF8 $stdoutPath } else { "" }
        $stderrText = if (Test-Path $stderrPath) { Get-Content -Raw -Encoding UTF8 $stderrPath } else { "" }
        $runLines = @(($stdoutText, $stderrText) -join [Environment]::NewLine -split "`r?`n")
        $summaries = @(New-TestSummary $runLines)
        $failureLines = @(Select-FailureLines $runLines)
        $runFailedCount = ($summaries | Measure-Object -Property Failed -Sum).Sum
        if ($null -eq $runFailedCount) {
            $runFailedCount = 0
        }
        $runPassedCount = ($summaries | Measure-Object -Property Passed -Sum).Sum
        if ($null -eq $runPassedCount) {
            $runPassedCount = 0
        }
        $runMatchedBinaries = @($summaries | Where-Object { $_.Passed -gt 0 -or $_.Failed -gt 0 -or $_.Ignored -gt 0 })
        $runOverall = if ($process.ExitCode -eq 0 -and $runFailedCount -eq 0 -and $runPassedCount -gt 0 -and $runMatchedBinaries.Count -gt 0) { "PASS" } else { "FAIL" }

        $runResults += [pscustomobject]@{
            Name = $testRun.Name
            Package = [string[]]$testRun.Package
            TestFilter = $testRun.TestFilter
            Command = Get-CargoCommandText $testRun
            ExitCode = $process.ExitCode
            Started = $runStarted
            Finished = $runFinished
            StdoutPath = $stdoutPath
            StderrPath = $stderrPath
            Lines = [string[]]$runLines
            Summaries = [object[]]$summaries
            FailureLines = [string[]]$failureLines
            Passed = $runPassedCount
            Failed = $runFailedCount
            MatchedBinaries = $runMatchedBinaries.Count
            Overall = $runOverall
        }
    }
}
finally {
    $env:CARGO_TARGET_DIR = $oldTargetDir
    $env:CARGO_BUILD_JOBS = $oldBuildJobs
}

$scriptResults = @()
foreach ($scriptName in $scriptTestRuns) {
    $safeName = $scriptName -replace "[^A-Za-z0-9_.-]", "-"
    $stdoutPath = Join-Path $runDir "script-test-$safeName.stdout.log"
    $stderrPath = Join-Path $runDir "script-test-$safeName.stderr.log"
    $scriptPath = Join-Path $PSScriptRoot $scriptName
    $scriptReportDir = Join-Path $runDir $safeName
    $scriptArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $scriptPath, "-OutputDir", $scriptReportDir)
    if ($scriptName -eq "run-tui-taskspace-viewer-e2e.ps1") {
        $scriptArgs += @("-TimeoutSeconds", [string]$TuiViewerTimeoutSeconds)
    }
    $runStarted = Get-Date
    $process = Start-Process `
        -FilePath "powershell" `
        -ArgumentList $scriptArgs `
        -WorkingDirectory (Split-Path -Parent $PSScriptRoot) `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -NoNewWindow `
        -Wait `
        -PassThru
    $runFinished = Get-Date
    $stdoutText = if (Test-Path $stdoutPath) { Get-Content -Raw -Encoding UTF8 $stdoutPath } else { "" }
    $stderrText = if (Test-Path $stderrPath) { Get-Content -Raw -Encoding UTF8 $stderrPath } else { "" }
    $scriptOverall = if ($process.ExitCode -eq 0 -and $stdoutText -match "Overall:\s+PASS") { "PASS" } elseif ($process.ExitCode -eq 0 -and $stdoutText -match "Overall:\s+SKIP") { "SKIP" } else { "FAIL" }
    $viewerReportPath = Join-Path $scriptReportDir "artifacts\report.md"
    $viewerMetrics = if ($scriptName -eq "run-tui-taskspace-viewer-e2e.ps1") { Read-KeyValueReport $viewerReportPath } else { @{} }
    $scriptResults += [pscustomobject]@{
        Name = $scriptName
        Command = "powershell $($scriptArgs -join ' ')"
        ExitCode = $process.ExitCode
        Started = $runStarted
        Finished = $runFinished
        StdoutPath = $stdoutPath
        StderrPath = $stderrPath
        ReportDir = $scriptReportDir
        Overall = $scriptOverall
        ViewerReportPath = $viewerReportPath
        ViewerMetrics = $viewerMetrics
        Output = "$stdoutText`n$stderrText"
    }
}

$finished = Get-Date

$combinedLines = New-Object System.Collections.Generic.List[string]
foreach ($run in $runResults) {
    $combinedLines.Add("===== $($run.Name): $($run.Command) =====")
    $combinedLines.AddRange([string[]]$run.Lines)
    $combinedLines.Add("")
}
foreach ($run in $scriptResults) {
    $combinedLines.Add("===== $($run.Name): $($run.Command) =====")
    $combinedLines.AddRange([string[]]($run.Output -split "`r?`n"))
    $combinedLines.Add("")
}
$combinedLines | Set-Content -Encoding UTF8 $logPath

$summaries = @($runResults | ForEach-Object { $_.Summaries })
$failureLines = @($runResults | ForEach-Object { $_.FailureLines })
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
$failedRuns = @($runResults | Where-Object { $_.Overall -ne "PASS" })
$failedScriptRuns = @($scriptResults | Where-Object { $_.Overall -eq "FAIL" })
$skippedScriptRuns = @($scriptResults | Where-Object { $_.Overall -eq "SKIP" })

$overall = if ($failedRuns.Count -eq 0 -and $failedScriptRuns.Count -eq 0 -and $failedCount -eq 0 -and $passedCount -gt 0) { "PASS" } else { "FAIL" }
$exitCode = if ($overall -eq "PASS") {
    0
} else {
    $firstNonZero = @($runResults | Where-Object { $_.ExitCode -ne 0 } | Select-Object -First 1)
    $firstScriptNonZero = @($scriptResults | Where-Object { $_.ExitCode -ne 0 } | Select-Object -First 1)
    if ($firstNonZero.Count -gt 0) { $firstNonZero[0].ExitCode }
    elseif ($firstScriptNonZero.Count -gt 0) { $firstScriptNonZero[0].ExitCode }
    else { 1 }
}

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map Regression Report")
$report.Add("")
$report.Add("- overall: $overall")
$report.Add("- exit_code: $exitCode")
$report.Add("- command_matrix: $commandText")
$report.Add("- script_matrix: $scriptCommandText")
$report.Add("- include_tui_viewer_e2e: $([bool]$IncludeTuiViewerE2E)")
$report.Add("- tui_viewer_timeout_seconds: $TuiViewerTimeoutSeconds")
$report.Add("- run_count: $($runResults.Count)")
$report.Add("- script_run_count: $($scriptResults.Count)")
$report.Add("- skipped_script_runs: $($skippedScriptRuns.Count)")
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
$report.Add("## Test Runs")
$report.Add("")
if ($runResults.Count -eq 0) {
    $report.Add("No cargo test runs were executed.")
} else {
    $report.Add("| run | overall | packages | filter | exit | passed | failed | matched binaries | stdout | stderr |")
    $report.Add("|---|---|---|---|---:|---:|---:|---:|---|---|")
    foreach ($run in $runResults) {
        $report.Add("| $(Escape-Markdown $run.Name) | $($run.Overall) | $(Escape-Markdown ($run.Package -join ', ')) | $(Escape-Markdown $run.TestFilter) | $($run.ExitCode) | $($run.Passed) | $($run.Failed) | $($run.MatchedBinaries) | $(Escape-Markdown $run.StdoutPath) | $(Escape-Markdown $run.StderrPath) |")
    }
}
$report.Add("")
$report.Add("## Script Test Runs")
$report.Add("")
if ($scriptResults.Count -eq 0) {
    $report.Add("No script tests were executed.")
} else {
    $report.Add("| run | overall | exit | report dir | stdout | stderr |")
    $report.Add("|---|---|---:|---|---|---|")
    foreach ($run in $scriptResults) {
        $report.Add("| $(Escape-Markdown $run.Name) | $($run.Overall) | $($run.ExitCode) | $(Escape-Markdown $run.ReportDir) | $(Escape-Markdown $run.StdoutPath) | $(Escape-Markdown $run.StderrPath) |")
    }
}
$report.Add("")
$report.Add("## TUI Viewer E2E")
$report.Add("")
$viewerRun = @($scriptResults | Where-Object { $_.Name -eq "run-tui-taskspace-viewer-e2e.ps1" } | Select-Object -First 1)
if ($viewerRun.Count -eq 0) {
    if ($IncludeTuiViewerE2E) {
        $report.Add("TUI viewer E2E was requested but no run was recorded.")
    } else {
        $report.Add("TUI viewer E2E was not requested. Pass -IncludeTuiViewerE2E to run the real browser path.")
    }
} else {
    $report.Add("- report: $($viewerRun[0].ViewerReportPath)")
    foreach ($key in @(
        "browser_interaction_ok",
        "browser_refresh_count",
        "browser_snapshot_first_ms",
        "browser_snapshot_last_ms",
        "browser_snapshot_status_ok",
        "browser_snapshot_active_ok",
        "detail_state_ok",
        "selection_state_ok",
        "graph_zoom_ok",
        "graph_pan_ok",
        "graph_transform_ok",
        "refresh_during_detail_ok",
        "refresh_during_graph_ok",
        "refresh_during_selection_ok",
        "snapshot_map_count",
        "snapshot_node_count",
        "snapshot_edge_count",
        "snapshot_result_count",
        "assistant_marker_observed",
        "user_prompt_contains_marker",
        "console_error_count",
        "favicon_console_error_count",
        "network_failure_count",
        "snapshot_dir"
    )) {
        $value = $viewerRun[0].ViewerMetrics[$key]
        if ($null -ne $value) {
            $report.Add("- ${key}: $value")
        }
    }
}
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
$report.Add("- Each filtered cargo run is only accepted when at least one matching test passed.")
$report.Add("- Lines with 0 passed; 0 failed; ... filtered out are diagnostic noise from crates without matching tests, not a pass signal.")

$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "Log: $logPath"
Write-Host "Overall: $overall"
if ($overall -ne "PASS" -and $exitCode -eq 0) {
    exit 1
}
exit $exitCode
