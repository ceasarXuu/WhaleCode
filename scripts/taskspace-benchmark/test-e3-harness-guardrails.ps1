param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\run-state.ps1")
. (Join-Path $PSScriptRoot "lib\failure-taxonomy.ps1")
. (Join-Path $PSScriptRoot "lib\aggregate-report.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-guardrails-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

$stdout = Join-Path $runDir "validation.stdout.log"
$stderr = Join-Path $runDir "validation.stderr.log"
@"
validator_lifecycle_stage=entry_started
validator_lifecycle_stage=tests_started
validator_tests_started=true
validator_lifecycle_stage=tests_completed
validator_tests_completed=true
"@ | Set-Content -LiteralPath $stdout -Encoding UTF8
"" | Set-Content -LiteralPath $stderr -Encoding UTF8
$validation = [pscustomobject]@{ exit_code = 7; stdout_path = $stdout; stderr_path = $stderr }
$lifecycle = Get-TaskspaceValidationLifecycle $validation
Assert-True ([bool]$lifecycle.tests_started_seen) "lifecycle parser missed tests_started marker"
Assert-True ([bool]$lifecycle.tests_completed_seen) "lifecycle parser missed tests_completed marker"
Assert-True ([string]$lifecycle.validation_lifecycle_stage -eq "tests_completed") "lifecycle parser did not keep last stage"

$pretestStderr = Join-Path $runDir "pretest.stderr.log"
"Resolve-Path : Cannot find path 'target\bad-uv-cache'" | Set-Content -LiteralPath $pretestStderr -Encoding UTF8
$sig = Get-TaskspaceHarnessTextSignature (Get-Content -Raw -Encoding UTF8 -LiteralPath $pretestStderr) "validator_pretest" "left" $pretestStderr
Assert-True ($sig -and [string]$sig.stable_code -eq "path_unresolvable") "stderr fallback did not classify path_unresolvable"

$wslStderr = Join-Path $runDir "wsl.stderr.log"
"external-validator.ps1 : <3>WSL (1148 - Relay) ERROR: CreateProcessParseCommon:1014: getpwnam(root) failed 5" | Set-Content -LiteralPath $wslStderr -Encoding UTF8
$wslSig = Get-TaskspaceHarnessTextSignature (Get-Content -Raw -Encoding UTF8 -LiteralPath $wslStderr) "validator_pretest" "left" $wslStderr
Assert-True ($wslSig -and [string]$wslSig.stable_code -eq "docker_backend_unavailable") "stderr fallback did not classify WSL root lookup as docker_backend_unavailable"

$standardMetrics = [pscustomobject]@{
    mode = "left"
    logical_mode = "standard"
    public_validation_exit_code = 1
    pretest_failure = $true
    validation_stderr_path = $pretestStderr
    validator_environment_failures = @("path_unresolvable")
    infra_signature = $sig
}
$taskspaceMetrics = [pscustomobject]@{
    mode = "right"
    logical_mode = "taskspace"
    public_validation_exit_code = 1
    pretest_failure = $true
    validation_stderr_path = $pretestStderr
    validator_environment_failures = @("path_unresolvable")
    infra_signature = $sig
}
$sentinel = Get-TaskspaceSentinelAbortDecision $standardMetrics $taskspaceMetrics
Assert-True ([bool]$sentinel.abort -and [string]$sentinel.reason -eq "same_infra_signature_both_sides") "sentinel did not abort same infra signature"

$afterTestsMetrics = [pscustomobject]@{
    mode = "right"
    logical_mode = "taskspace"
    public_validation_exit_code = 1
    pretest_failure = $false
    tests_started_seen = $true
    validator_environment_failures = @()
    infra_signature = $null
}
$standardAfterTestsMetrics = [pscustomobject]@{
    mode = "left"
    logical_mode = "standard"
    public_validation_exit_code = 1
    pretest_failure = $false
    tests_started_seen = $true
    validator_environment_failures = @()
    infra_signature = $null
}
$noAbort = Get-TaskspaceSentinelAbortDecision $standardAfterTestsMetrics $afterTestsMetrics
Assert-True (-not [bool]$noAbort.abort) "sentinel aborted a failure that reached tests_started"

$manifest = [pscustomobject]@{
    PromptPath = Join-Path $runDir "prompt.txt"
    FixtureDir = Join-Path $runDir "fixture"
    ExternalBenchmark = [pscustomobject]@{
        adapter_metadata = [pscustomobject]@{
            uv_cache_root = "relative\uv-cache"
            validator_source_dir = Join-Path $runDir "validator-source"
            fixture_source = Join-Path $runDir "fixture"
        }
    }
}
"prompt" | Set-Content -LiteralPath $manifest.PromptPath -Encoding UTF8
New-Item -ItemType Directory -Force -Path $manifest.FixtureDir | Out-Null
New-Item -ItemType Directory -Force -Path $manifest.ExternalBenchmark.adapter_metadata.validator_source_dir | Out-Null
$health = Get-TaskspaceHarnessHealth $manifest $runDir $runDir
Assert-True ([string]$health.status -eq "fail") "preflight health did not fail relative materialized path"
Assert-True (@($health.findings | Where-Object { [string]$_.stable_code -eq "relative_materialized_path" }).Count -gt 0) "preflight health did not name relative_materialized_path"

$stateRun = Join-Path $runDir "invalid-state"
New-Item -ItemType Directory -Force -Path $stateRun | Out-Null
Initialize-TaskspaceBenchmarkRunState $stateRun "sample" 5 "E3" "selftest" | Out-Null
Set-TaskspaceInvalidHarnessStatus $stateRun "sample" "sentinel_pair" "same_infra_signature_both_sides" $sig $pretestStderr "selftest" 1 1 | Out-Null
$runStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stateRun "run-status.json") | ConvertFrom-Json
$sampleStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stateRun "sample-status.json") | ConvertFrom-Json
Assert-True ([string]$runStatus.run_validity -eq "invalid_harness" -and [int]$runStatus.exit_code -eq 3) "invalid run status did not record exit code 3"
Assert-True ([string]$sampleStatus.run_validity -eq "invalid_harness" -and [int]$sampleStatus.exit_code -eq 3) "invalid sample status did not record invalid_harness exit code 3"
Assert-True (-not [bool]$sampleStatus.resume_allowed -and [string]$sampleStatus.abort_phase -eq "sentinel_pair") "invalid sample status did not block resume"

$aggregatePath = Join-Path $runDir "aggregate-report.md"
$evidence = [pscustomobject]@{
    reported_evidence_level = "E3-candidate"
    included_in_utility_aggregate = $false
    included_in_e3_aggregate = $false
    evidence_gate_failures = @()
    e3_gate_failures = @("path_unresolvable")
    failure_taxonomy = @("harness_materialization_failure")
    utility_direction = "inconclusive"
    human_review_completed = $false
    human_review_decision = ""
    human_review_disagreement = $false
}
Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @([pscustomobject]@{ repeat = 1; pair_dir = $runDir; pair_report = "pair-report.md"; evidence_target = "E3"; evidence = $evidence })
$aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "aggregate.json") | ConvertFrom-Json
$aggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath
Assert-True ([string]$aggregate.run_validity -eq "invalid_harness") "aggregate did not mark invalid_harness"
Assert-True (-not [bool]$aggregate.diagnostic_comparison_enabled) "aggregate did not disable diagnostic comparison"
Assert-True ($aggregateText -match "diagnostic_comparison_enabled: False") "aggregate report did not render comparison-disabled status"
Assert-True ($aggregateText -notmatch "taskspace_better|standard_better|regressed|worse") "invalid harness aggregate rendered directional comparison wording"

if ($failures.Count -gt 0) {
    Write-Host "E3 harness guardrails self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 harness guardrails self-test: PASS"
Write-Host "RunRoot: $runDir"
