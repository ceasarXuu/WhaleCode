param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("deepswe", "terminal-bench")]
    [string]$Benchmark,
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [string]$SampleId = "",
    [string]$SourceVersion = "",
    [int]$Repeats = 1,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [int]$ValidationTimeoutSeconds = 420,
    [int]$ValidationPretestTimeoutSeconds = 120,
    [int]$ValidationTestTimeoutSeconds = 420,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AuditReviewRoot = "",
    [string]$RunnerPath = "",
    [string]$TaskListHash = "",
    [string]$ProfileHash = "",
    [switch]$EnableAggregate,
    [switch]$AllowDiagnosticNonTargetResult,
    [switch]$ScoringMode,
    [switch]$RequireScoreValidity,
    [switch]$EnableDockerImageCache,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "SourceVersion must pin the external benchmark source revision." }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-external-bench-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$scenarioRoot = Join-Path $RunRoot "materialized-scenarios"
$preMaterializationHealthPath = Join-Path $RunRoot "external-materialization-preflight-health.json"
$preMaterializationHealth = New-TaskspaceDiskHealth @($RunRoot, $scenarioRoot, $TaskDir) "external_materialization_preflight"
Write-TaskspaceHarnessHealth $preMaterializationHealthPath $preMaterializationHealth
if ([string]$preMaterializationHealth.status -eq "fail") {
    $firstFinding = @($preMaterializationHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "external_materialization_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $preMaterializationHealthPath
    $abortSummaryPath = Join-Path $RunRoot "abort-summary.md"
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace External Materialization Abort" "external_materialization_preflight" $firstFinding $signature $preMaterializationHealthPath | Set-Content -LiteralPath $abortSummaryPath -Encoding UTF8
    [pscustomobject]@{ schema_version = 1; phase = "invalid_harness"; run_validity = "invalid_harness"; diagnostic_comparison_enabled = $false; exit_code = 3; resume_allowed = $false; force_rerun_required = $true; invalid_run_reason = [string]$firstFinding.stable_code; first_failure_artifact = $preMaterializationHealthPath } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $RunRoot "run-status.json") -Encoding UTF8
    [pscustomobject]@{ schema_version = 1; sample_id = if ($SampleId) { $SampleId } else { Split-Path -Leaf $TaskDir }; phase = "invalid_harness"; run_validity = "invalid_harness"; diagnostic_comparison_enabled = $false; exit_code = 3; resume_allowed = $false; force_rerun_required = $true; abort_scope = "sample"; abort_phase = "external_materialization_preflight"; abort_signature = $signature.key; abort_reason = [string]$firstFinding.stable_code; first_failure_artifact = $preMaterializationHealthPath } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $RunRoot "sample-status.json") -Encoding UTF8
    Write-Host "ExternalMaterializationHealth: $preMaterializationHealthPath"
    Write-Host "AbortSummary: $abortSummaryPath"
    exit 3
}
$adapter = switch ($Benchmark) {
    "deepswe" { Join-Path $PSScriptRoot "adapters\deepswe-adapter.ps1" }
    "terminal-bench" { Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1" }
}
try {
    $materialized = & $adapter -TaskDir $TaskDir -OutputRoot $scenarioRoot -SampleId $SampleId -SourceVersion $SourceVersion
} catch {
    $message = [string]$_.Exception.Message
    $code = if ($message -match "uv") { "uv_cache_missing" } elseif ($message -match "validator") { "validator_source_missing" } elseif ($message -match "Resolve-Path|Cannot find path") { "path_unresolvable" } else { "adapter_materialization_failed" }
    $signature = [pscustomobject]@{
        schema_version = 1
        category = "harness_materialization_failure"
        stage = "external_materialization"
        stable_code = $code
        normalized_message = $message
        side = ""
        artifact = ""
        key = "harness_materialization_failure/$code"
    }
    $healthPath = Join-Path $RunRoot "external-materialization-health.json"
    $runStatusPath = Join-Path $RunRoot "run-status.json"
    $sampleStatusPath = Join-Path $RunRoot "sample-status.json"
    $abortSummaryPath = Join-Path $RunRoot "abort-summary.md"
    [pscustomobject]@{
        schema_version = 1
        status = "fail"
        run_validity = "invalid_harness"
        findings = @([pscustomobject]@{ severity = "fail"; stable_code = $code; message = $message; path = $TaskDir })
        checked_paths = @([pscustomobject]@{ name = "task_dir"; path = $TaskDir; exists = (Test-Path -LiteralPath $TaskDir); fully_qualified = ([System.IO.Path]::IsPathRooted($TaskDir) -and -not [string]::IsNullOrWhiteSpace([System.IO.Path]::GetPathRoot($TaskDir))) })
        infra_signature = $signature
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $healthPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = 1
        phase = "invalid_harness"
        run_validity = "invalid_harness"
        diagnostic_comparison_enabled = $false
        exit_code = 3
        resume_allowed = $false
        force_rerun_required = $true
        invalid_run_reason = $code
        first_failure_artifact = $healthPath
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $runStatusPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = 1
        sample_id = if ($SampleId) { $SampleId } else { Split-Path -Leaf $TaskDir }
        phase = "invalid_harness"
        run_validity = "invalid_harness"
        diagnostic_comparison_enabled = $false
        exit_code = 3
        resume_allowed = $false
        force_rerun_required = $true
        abort_scope = "sample"
        abort_phase = "external_materialization"
        abort_signature = $signature.key
        abort_reason = $code
        first_failure_artifact = $healthPath
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $sampleStatusPath -Encoding UTF8
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace External Materialization Abort" "external_materialization" ([pscustomobject]@{ stable_code = $code; message = $message; path = $TaskDir; root = ""; free_bytes = 0; required_free_bytes = 0 }) $signature $healthPath | Set-Content -LiteralPath $abortSummaryPath -Encoding UTF8
    Write-Host "ExternalMaterializationHealth: $healthPath"
    Write-Host "AbortSummary: $abortSummaryPath"
    exit 3
}
$scenarioDir = [string]($materialized | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
if ([string]::IsNullOrWhiteSpace($scenarioDir)) {
    $healthPath = Join-Path $RunRoot "external-materialization-health.json"
    [pscustomobject]@{ schema_version = 1; status = "fail"; run_validity = "invalid_harness"; findings = @([pscustomobject]@{ severity = "fail"; stable_code = "adapter_materialization_failed"; message = "Adapter did not return a scenario_dir."; path = $TaskDir }); generated_at = (Get-Date).ToString("o") } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $healthPath -Encoding UTF8
    [pscustomobject]@{ schema_version = 1; sample_id = if ($SampleId) { $SampleId } else { Split-Path -Leaf $TaskDir }; phase = "invalid_harness"; run_validity = "invalid_harness"; exit_code = 3; abort_scope = "sample"; abort_phase = "external_materialization"; abort_signature = "harness_materialization_failure/adapter_materialization_failed"; first_failure_artifact = $healthPath } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $RunRoot "sample-status.json") -Encoding UTF8
    exit 3
}
$runner = if ([string]::IsNullOrWhiteSpace($RunnerPath)) { Join-Path $repoRoot "scripts\taskspace-benchmark\run-taskspace-benchmark.ps1" } else { $RunnerPath }
$args = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
    "-ScenarioPath", $scenarioDir,
    "-Repeats", $Repeats,
    "-WhaleBin", $WhaleBin,
    "-Model", $Model,
    "-RunRoot", (Join-Path $RunRoot "runs"),
    "-TimeoutSeconds", $TimeoutSeconds,
    "-ValidationTimeoutSeconds", $ValidationTimeoutSeconds,
    "-ValidationPretestTimeoutSeconds", $ValidationPretestTimeoutSeconds,
    "-ValidationTestTimeoutSeconds", $ValidationTestTimeoutSeconds,
    "-SandboxMode", $SandboxMode
)
if (-not [string]::IsNullOrWhiteSpace($TaskListHash)) { $args += @("-TaskListHash", $TaskListHash) }
if (-not [string]::IsNullOrWhiteSpace($ProfileHash)) { $args += @("-ProfileHash", $ProfileHash) }
foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
if (-not [string]::IsNullOrWhiteSpace($AuditReviewRoot)) { $args += @("-AuditReviewRoot", $AuditReviewRoot) }
if ($EnableAggregate) { $args += "-EnableAggregate" }
if ($AllowDiagnosticNonTargetResult) { $args += "-AllowNonE2Result" }
if ($ScoringMode) { $args += "-ScoringMode" }
if ($RequireScoreValidity) { $args += "-RequireScoreValidity" }
if ($EnableDockerImageCache) { $args += "-EnableDockerImageCache" }
if ($PlanOnly) { $args += "-PlanOnly" }
& powershell @args
$exitCode = $LASTEXITCODE
if ($AllowDiagnosticNonTargetResult -and -not $PlanOnly) {
    Write-Host "DiagnosticNonTargetResultAllowed: True"
    Write-Host "Requested target may be unsatisfied; inspect RunSummary and PairReport."
}
exit $exitCode
