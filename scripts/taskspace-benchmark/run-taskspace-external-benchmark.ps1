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
    [string]$SandboxMode = "bypass",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AuditReviewRoot = "",
    [string]$RunnerPath = "",
    [string]$TaskListHash = "",
    [string]$ProfileHash = "",
    [string]$SampleSetId = "",
    [string[]]$SampleNames = @(),
    [string]$SuiteRunnerEntrypoint = "",
    [string]$ArtifactOrigin = "",
    [string]$RunnerScriptSha256 = "",
    [string]$ChildRunnerSha256 = "",
    [string]$TaskListSha256 = "",
    [string]$SuiteManifestPath = "",
    [string]$SuiteReceiptPath = "",
    [string]$SuiteReceiptSha256 = "",
    [string]$ApprovalMarkerSha256 = "",
    [string]$CodeCompleteMarkerSha256 = "",
    [string]$V005NonAgentGatesPath = "",
    [string]$V005CodeCompleteMarkerPath = "",
    [string]$V005UserApprovalMarkerPath = "",
    [ValidateSet("both", "left", "right")]
    [string]$RunSide = "both",
    [switch]$EnableAggregate,
    [switch]$AllowDiagnosticNonTargetResult,
    [switch]$ScoringMode,
    [switch]$RequireScoreValidity,
    [switch]$EnableDockerImageCache,
    [switch]$AllowStaleWhaleBin,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")

function ConvertTo-TaskspaceSampleNameList {
    param([string[]]$Names)
    @($Names | ForEach-Object { ([string]$_) -split "," } | ForEach-Object { ([string]$_).Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
}

if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "SourceVersion must pin the external benchmark source revision." }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-external-bench-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$materializationHashInput = [System.Text.Encoding]::UTF8.GetBytes("$RunRoot`n$TaskDir`n$SampleId`n$SourceVersion")
$materializationSha = [System.Security.Cryptography.SHA256]::Create()
try {
    $materializationKey = (([System.BitConverter]::ToString($materializationSha.ComputeHash($materializationHashInput)) -replace "-", "").ToLowerInvariant()).Substring(0, 16)
} finally {
    $materializationSha.Dispose()
}
$scenarioRoot = Join-Path $repoRoot "target\external-materialized\$materializationKey"
New-Item -ItemType Directory -Path $scenarioRoot -Force | Out-Null
[pscustomobject]@{
    schema_version = 1
    run_root = $RunRoot
    scenario_root = $scenarioRoot
    task_dir = $TaskDir
    sample_id = $SampleId
    source_version = $SourceVersion
    generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $RunRoot "materialized-scenarios-pointer.json") -Encoding UTF8
$binaryHealthPath = Join-Path $RunRoot "whale-binary-preflight-health.json"
$binaryHealth = New-TaskspaceWhaleBinaryHealth $WhaleBin $repoRoot -AllowStale:$AllowStaleWhaleBin
Write-TaskspaceHarnessHealth $binaryHealthPath $binaryHealth
if ([string]$binaryHealth.status -eq "fail") {
    $firstFinding = @($binaryHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "whale_binary_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $binaryHealthPath
    $abortSummaryPath = Join-Path $RunRoot "abort-summary.md"
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace Whale Binary Abort" "whale_binary_preflight" $firstFinding $signature $binaryHealthPath | Set-Content -LiteralPath $abortSummaryPath -Encoding UTF8
    [pscustomobject]@{ schema_version = 1; phase = "invalid_harness"; run_validity = "invalid_harness"; diagnostic_comparison_enabled = $false; exit_code = 3; resume_allowed = $false; force_rerun_required = $true; invalid_run_reason = [string]$firstFinding.stable_code; first_failure_artifact = $binaryHealthPath } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $RunRoot "run-status.json") -Encoding UTF8
    [pscustomobject]@{ schema_version = 1; sample_id = if ($SampleId) { $SampleId } else { Split-Path -Leaf $TaskDir }; phase = "invalid_harness"; run_validity = "invalid_harness"; diagnostic_comparison_enabled = $false; exit_code = 3; resume_allowed = $false; force_rerun_required = $true; abort_scope = "sample"; abort_phase = "whale_binary_preflight"; abort_signature = $signature.key; abort_reason = [string]$firstFinding.stable_code; first_failure_artifact = $binaryHealthPath } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $RunRoot "sample-status.json") -Encoding UTF8
    Write-Host "WhaleBinaryHealth: $binaryHealthPath"
    Write-Host "AbortSummary: $abortSummaryPath"
    exit 3
}
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
$args += @("-SourceVersion", $SourceVersion)
if (-not [string]::IsNullOrWhiteSpace($ProfileHash)) { $args += @("-ProfileHash", $ProfileHash) }
if (-not [string]::IsNullOrWhiteSpace($SampleSetId)) { $args += @("-SampleSetId", $SampleSetId) }
$nonEmptySampleNames = @(ConvertTo-TaskspaceSampleNameList $SampleNames)
if ($nonEmptySampleNames.Count -gt 0) {
    $args += @("-SampleNames", (@($nonEmptySampleNames) -join ","))
}
$args += @("-BenchmarkFamily", $Benchmark)
if (-not [string]::IsNullOrWhiteSpace($SuiteRunnerEntrypoint)) { $args += @("-RunnerEntrypoint", $SuiteRunnerEntrypoint) }
if (-not [string]::IsNullOrWhiteSpace($ArtifactOrigin)) { $args += @("-ArtifactOrigin", $ArtifactOrigin) }
if (-not [string]::IsNullOrWhiteSpace($RunnerScriptSha256)) { $args += @("-RunnerScriptSha256", $RunnerScriptSha256) }
if (-not [string]::IsNullOrWhiteSpace($ChildRunnerSha256)) { $args += @("-ChildRunnerSha256", $ChildRunnerSha256) }
if (-not [string]::IsNullOrWhiteSpace($TaskListSha256)) { $args += @("-TaskListSha256", $TaskListSha256) }
if (-not [string]::IsNullOrWhiteSpace($SuiteManifestPath)) { $args += @("-SuiteManifestPath", $SuiteManifestPath) }
if (-not [string]::IsNullOrWhiteSpace($SuiteReceiptPath)) { $args += @("-SuiteReceiptPath", $SuiteReceiptPath) }
if (-not [string]::IsNullOrWhiteSpace($SuiteReceiptSha256)) { $args += @("-SuiteReceiptSha256", $SuiteReceiptSha256) }
if (-not [string]::IsNullOrWhiteSpace($ApprovalMarkerSha256)) { $args += @("-ApprovalMarkerSha256", $ApprovalMarkerSha256) }
if (-not [string]::IsNullOrWhiteSpace($CodeCompleteMarkerSha256)) { $args += @("-CodeCompleteMarkerSha256", $CodeCompleteMarkerSha256) }
if (-not [string]::IsNullOrWhiteSpace($V005NonAgentGatesPath)) { $args += @("-V005NonAgentGatesPath", $V005NonAgentGatesPath) }
if (-not [string]::IsNullOrWhiteSpace($V005CodeCompleteMarkerPath)) { $args += @("-V005CodeCompleteMarkerPath", $V005CodeCompleteMarkerPath) }
if (-not [string]::IsNullOrWhiteSpace($V005UserApprovalMarkerPath)) { $args += @("-V005UserApprovalMarkerPath", $V005UserApprovalMarkerPath) }
$args += @("-RunSide", $RunSide)
foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
if (-not [string]::IsNullOrWhiteSpace($AuditReviewRoot)) { $args += @("-AuditReviewRoot", $AuditReviewRoot) }
if ($EnableAggregate) { $args += "-EnableAggregate" }
if ($AllowDiagnosticNonTargetResult) { $args += "-AllowNonE2Result" }
if ($ScoringMode) { $args += "-ScoringMode" }
if ($RequireScoreValidity) { $args += "-RequireScoreValidity" }
if ($EnableDockerImageCache) { $args += "-EnableDockerImageCache" }
if ($AllowStaleWhaleBin) { $args += "-AllowStaleWhaleBin" }
if ($PlanOnly) { $args += "-PlanOnly" }
& powershell @args
$exitCode = $LASTEXITCODE
if ($AllowDiagnosticNonTargetResult -and -not $PlanOnly) {
    Write-Host "DiagnosticNonTargetResultAllowed: True"
    Write-Host "Requested target may be unsatisfied; inspect RunSummary and PairReport."
}
exit $exitCode
