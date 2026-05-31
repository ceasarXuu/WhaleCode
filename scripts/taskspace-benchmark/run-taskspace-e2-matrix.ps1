param(
    [string[]]$Scenarios = @("single-file-fast-fix", "multi-file-order-pipeline", "subscription-billing-repair"),
    [string[]]$RequiredLevels = @("L1", "L2", "L3"),
    [int]$Repeats = 3,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [switch]$AllowNonE2Result
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\matrix-report.ps1")

if (-not $RunRoot) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
    $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-paired-matrix-runs\$stamp"
}
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null

$rows = New-Object System.Collections.Generic.List[object]
$runner = Join-Path $PSScriptRoot "run-taskspace-benchmark.ps1"
foreach ($scenario in $Scenarios) {
    $manifest = Read-TaskspaceScenarioManifest $repoRoot $scenario
    $stdout = Join-Path $RunRoot "$scenario.stdout.log"
    $stderr = Join-Path $RunRoot "$scenario.stderr.log"
    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
        "-Scenario", $scenario,
        "-Repeats", $Repeats,
        "-RunRoot", $RunRoot,
        "-WhaleBin", $WhaleBin,
        "-Model", $Model,
        "-TimeoutSeconds", $TimeoutSeconds,
        "-SandboxMode", $SandboxMode,
        "-EnableAggregate"
    )
    foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
    if ($AllowNonE2Result) { $args += "-AllowNonE2Result" }

    $process = Start-Process -FilePath "powershell" -ArgumentList $args -WorkingDirectory $repoRoot -NoNewWindow -Wait -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $runDir = ""
    if (Test-Path -LiteralPath $stdout) {
        $runDirLine = Get-Content -Encoding UTF8 -LiteralPath $stdout | Where-Object { $_ -like "RunDir:*" } | Select-Object -Last 1
        if ($runDirLine) { $runDir = $runDirLine.Substring("RunDir:".Length).Trim() }
    }
    $aggregate = if ($runDir) { Join-Path $runDir "aggregate-report.md" } else { "" }
    $aggregateText = if ($aggregate -and (Test-Path -LiteralPath $aggregate)) { Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregate } else { "" }
    $validPairs = 0
    $excludedPairs = 0
    $nonE2 = 0
    if ($aggregateText -match "valid_utility_pairs:\s+(\d+)") { $validPairs = [int]$Matches[1] }
    if ($aggregateText -match "excluded_pairs:\s+(\d+)") { $excludedPairs = [int]$Matches[1] }
    $nonE2 = ([regex]::Matches($aggregateText, "reported_evidence_level:\s+(?!E2\b)\S+")).Count
    $pairReports = if ($runDir -and (Test-Path -LiteralPath $runDir)) {
        @(Get-ChildItem -Path $runDir -Recurse -Filter "pair-report.md" -ErrorAction SilentlyContinue)
    } else { @() }
    $outcomes = @{}
    $warningPairs = 0
    foreach ($pairReport in $pairReports) {
        $pairText = Get-Content -Raw -Encoding UTF8 -LiteralPath $pairReport.FullName
        $outcome = ([regex]::Match($pairText, "outcome:\s+(.+)")).Groups[1].Value.Trim()
        if ([string]::IsNullOrWhiteSpace($outcome)) { $outcome = "missing" }
        if (-not $outcomes.ContainsKey($outcome)) { $outcomes[$outcome] = 0 }
        $outcomes[$outcome]++
        if ($pairText -notmatch "## Scenario Warnings\s+(\r?\n)- none") { $warningPairs++ }
    }
    $outcomeSummary = if ($outcomes.Count -eq 0) {
        ""
    } else {
        @($outcomes.Keys | Sort-Object | ForEach-Object { "$_=$($outcomes[$_])" }) -join "; "
    }
    $rows.Add([pscustomobject]@{
            scenario = $scenario
            level = $manifest.Level
            exit_code = $process.ExitCode
            run_dir = $runDir
            aggregate = $aggregate
            valid_pairs = $validPairs
            excluded_pairs = $excludedPairs
            non_e2_reports = $nonE2
            utility_outcomes = $outcomeSummary
            warning_pairs = $warningPairs
            stdout = $stdout
            stderr = $stderr
        })
}

$data = Get-TaskspaceMatrixReportData @($rows.ToArray()) $RequiredLevels $Repeats
$rowArray = $data.rows

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# TaskSpace E2 Matrix Report")
$report.Add("")
$report.Add("- e2_evidence_readiness: $($data.e2_evidence_readiness)")
$report.Add("- e2_clean_readiness: $($data.e2_clean_readiness)")
$report.Add("- scenario_count: $($rows.Count)")
$report.Add("- levels: $($data.levels -join ', ')")
$report.Add("- required_levels: $($RequiredLevels -join ', ')")
$report.Add("- repeats_per_scenario: $Repeats")
$report.Add("- matrix_runner_sha256: $((Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash.ToLowerInvariant())")
$report.Add("- whale_sha256: $((Get-FileHash -Algorithm SHA256 -LiteralPath $WhaleBin).Hash.ToLowerInvariant())")
$report.Add("")
$report.Add("## Evidence Blocking Gaps")
if (@($data.evidence_blocking).Count -eq 0) { $report.Add("- none") } else { foreach ($gap in @($data.evidence_blocking)) { $report.Add("- $gap") } }
$report.Add("")
$report.Add("## Utility / Warning Gaps")
if (@($data.warning_gaps).Count -eq 0) { $report.Add("- none") } else { foreach ($gap in @($data.warning_gaps)) { $report.Add("- $gap") } }
$report.Add("")
$report.Add("## Scenarios")
$report.Add("| scenario | level | exit | valid pairs | excluded | non-E2 | warning pairs | utility outcomes | aggregate |")
$report.Add("|---|---|---:|---:|---:|---:|---:|---|---|")
foreach ($row in $rowArray) {
    $report.Add("| $($row.scenario) | $($row.level) | $($row.exit_code) | $($row.valid_pairs) | $($row.excluded_pairs) | $($row.non_e2_reports) | $($row.warning_pairs) | $($row.utility_outcomes) | $($row.aggregate) |")
}
$reportPath = Join-Path $RunRoot "e2-matrix-report.md"
$report | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "MatrixReport: $reportPath"
if (-not $data.e2_evidence_readiness -and -not $AllowNonE2Result) { exit 1 }
