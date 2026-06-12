param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("deepswe", "terminal-bench")]
    [string]$Benchmark,
    [Parameter(Mandatory = $true)][string]$TaskListPath,
    [string]$SourceVersion = "",
    [int]$Repeats = 5,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [int]$ValidationTimeoutSeconds = 420,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AuditReviewRoot = "",
    [switch]$PlanOnly,
    [switch]$ContinueAfterInvalidHarness
)

$ErrorActionPreference = "Stop"
if ($Repeats -lt 5) { throw "E3 suite requires Repeats >= 5." }
if (-not (Test-Path -LiteralPath $TaskListPath)) { Write-Error "TaskListPath not found: $TaskListPath"; exit 4 }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-e3-suite-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$suiteRoot = Join-Path $RunRoot ("suite-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $suiteRoot -Force | Out-Null
$samplesRoot = Join-Path $suiteRoot "samples"
New-Item -ItemType Directory -Path $samplesRoot -Force | Out-Null
$suiteHealthPath = Join-Path $suiteRoot "suite-health.json"
$skippedPath = Join-Path $suiteRoot "skipped-samples.jsonl"

function Read-TaskspaceSuiteList {
    param([string]$Path)
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    if ($raw.TrimStart().StartsWith("[")) { return @($raw | ConvertFrom-Json) }
    @(Get-Content -Encoding UTF8 -LiteralPath $Path | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
}

function Write-SuiteHealth {
    param($Status, $SampleStatuses, $SignatureCounts, [string]$AbortReason = "")
    [pscustomobject]@{
        schema_version = 1
        status = $Status
        suite_root = $suiteRoot
        signature_counts = $SignatureCounts
        sample_statuses = @($SampleStatuses)
        suite_abort_reason = $AbortReason
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteHealthPath -Encoding UTF8
}

try {
    $tasks = @(Read-TaskspaceSuiteList $TaskListPath)
} catch {
    Write-Error "Malformed TaskListPath: $($_.Exception.Message)"
    exit 4
}
if ($tasks.Count -eq 0) { Write-Error "TaskListPath contains no samples."; exit 4 }

$runner = Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1"
$sampleStatuses = New-Object System.Collections.Generic.List[object]
$signatureCounts = @{}
$suiteAbort = ""
$exitCode = 0

for ($index = 0; $index -lt $tasks.Count; $index++) {
    $task = $tasks[$index]
    $taskDir = if ($task.PSObject.Properties.Name -contains "task_dir") { [string]$task.task_dir } else { "" }
    $sampleId = if ($task.PSObject.Properties.Name -contains "sample_id") { [string]$task.sample_id } else { "sample-$($index + 1)" }
    $recordSourceVersion = if ($task.PSObject.Properties.Name -contains "source_version" -and -not [string]::IsNullOrWhiteSpace([string]$task.source_version)) { [string]$task.source_version } else { $SourceVersion }
    if ([string]::IsNullOrWhiteSpace($taskDir) -or [string]::IsNullOrWhiteSpace($recordSourceVersion)) {
        Write-Error "Suite sample requires task_dir and source_version/default SourceVersion: $sampleId"
        exit 4
    }
    if ($suiteAbort) {
        $row = [pscustomobject]@{
            sample_id = $sampleId
            task_dir = $taskDir
            run_validity = "invalid_harness"
            abort_scope = "suite"
            abort_phase = "suite_circuit_breaker"
            abort_signature = $suiteAbort
            skipped_reason = "suite_repeated_infra_signature"
        }
        ($row | ConvertTo-Json -Compress) | Add-Content -LiteralPath $skippedPath -Encoding UTF8
        $sampleStatuses.Add($row)
        continue
    }
    $sampleRoot = Join-Path $samplesRoot $sampleId
    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
        "-Benchmark", $Benchmark,
        "-TaskDir", $taskDir,
        "-SampleId", $sampleId,
        "-SourceVersion", $recordSourceVersion,
        "-Repeats", $Repeats,
        "-RunRoot", $sampleRoot,
        "-WhaleBin", $WhaleBin,
        "-Model", $Model,
        "-TimeoutSeconds", $TimeoutSeconds,
        "-ValidationTimeoutSeconds", $ValidationTimeoutSeconds,
        "-SandboxMode", $SandboxMode,
        "-EnableAggregate"
    )
    foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
    if ($AuditReviewRoot) { $args += @("-AuditReviewRoot", $AuditReviewRoot) }
    if ($PlanOnly) { $args += "-PlanOnly" }
    & powershell @args
    $childExit = $LASTEXITCODE
    $statusPath = Get-ChildItem -LiteralPath $sampleRoot -Filter "sample-status.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $status = if ($statusPath) { Get-Content -Raw -Encoding UTF8 -LiteralPath $statusPath.FullName | ConvertFrom-Json } else { [pscustomobject]@{ sample_id = $sampleId; run_validity = if ($childExit -eq 3) { "invalid_harness" } else { "unknown" }; exit_code = $childExit } }
    $sampleStatuses.Add($status)
    if ($childExit -eq 1 -and $exitCode -eq 0) { $exitCode = 1 }
    if ($childExit -eq 2 -and $exitCode -eq 0) { $exitCode = 2 }
    if ($childExit -eq 3 -or ($status.PSObject.Properties.Name -contains "run_validity" -and [string]$status.run_validity -eq "invalid_harness")) {
        $sig = if ($status.PSObject.Properties.Name -contains "abort_signature" -and -not [string]::IsNullOrWhiteSpace([string]$status.abort_signature)) { [string]$status.abort_signature } else { "harness_materialization_failure/unknown" }
        if (-not $signatureCounts.ContainsKey($sig)) { $signatureCounts[$sig] = 0 }
        $signatureCounts[$sig]++
        if ($exitCode -eq 0) { $exitCode = 3 }
        $global = $sig -match "docker_backend_unavailable|uv_cache_missing|validator_source_missing"
        if (-not $ContinueAfterInvalidHarness -and ($global -or $signatureCounts[$sig] -ge 2)) {
            $suiteAbort = $sig
            $exitCode = 3
        }
    }
}

$statusText = if ($suiteAbort) { "aborted" } else { "completed" }
Write-SuiteHealth $statusText @($sampleStatuses.ToArray()) $signatureCounts $suiteAbort
Write-Host "SuiteRoot: $suiteRoot"
Write-Host "SuiteHealth: $suiteHealthPath"
if (Test-Path -LiteralPath $skippedPath) { Write-Host "SkippedSamples: $skippedPath" }
exit $exitCode
