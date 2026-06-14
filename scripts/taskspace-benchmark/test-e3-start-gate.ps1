param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-start-gate-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

function New-GateScenario {
    param([string]$Root, [bool]$RelativeUv = $false)
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "fixture") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "validator") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "uv-cache") | Out-Null
    "prompt" | Set-Content -LiteralPath (Join-Path $Root "prompt.txt") -Encoding UTF8
    $uv = if ($RelativeUv) { "relative\uv-cache" } else { Join-Path $Root "uv-cache" }
    [pscustomobject]@{
        id = "gate-fixture"
        level = "E3"
        evidence_target = "E3"
        prompt_file = "prompt.txt"
        fixture_dir = "fixture"
        narrative_contract = "fixture contract"
        mode_delta_contract = "fixture delta"
        oracle = [pscustomobject]@{ hidden_strategy = "fixture"; public_validation = [pscustomobject]@{} }
        expected = [pscustomobject]@{ max_taskspace_nodes = 10 }
        thresholds = [pscustomobject]@{ taskspace_tool_call_ratio_warn = 10 }
        external_benchmark = [pscustomobject]@{
            adapter_metadata = [pscustomobject]@{
                uv_cache_root = $uv
                validator_source_dir = Join-Path $Root "validator"
                fixture_source = Join-Path $Root "fixture"
            }
        }
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $Root "scenario.json") -Encoding UTF8
}

$oldMinFreeBytes = $env:TASKSPACE_MIN_FREE_BYTES
$oldMinFreeGib = $env:TASKSPACE_MIN_FREE_GIB
try {
    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue

    $scenarioDir = Join-Path $runDir "scenario-pass"
    New-GateScenario $scenarioDir
    $gate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-pass") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$gate.status -eq "pass" -and [int]$gate.exit_code -eq 0) "start gate did not pass clean fixture"
    Assert-True (Test-Path -LiteralPath $gate.json_path) "start gate did not write json artifact"
    Assert-True (Test-Path -LiteralPath $gate.markdown_path) "start gate did not write markdown artifact"

    $noSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-smoke") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$noSmokeGate.status -eq "fail" -and @($noSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "fail" -and [string]$_.stable_code -eq "one_pair_smoke_not_provided" }).Count -eq 1) "start gate allowed missing one-pair smoke without explicit allow"

    $smokeRoot = Join-Path $runDir "one-pair-smoke"
    New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null
    [pscustomobject]@{ score_valid = $true; run_validity = "valid"; clean_comparable_pair_count = 1 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $smokeRoot "aggregate.json") -Encoding UTF8
    $smokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $smokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$smokeGate.status -eq "pass" -and @($smokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate did not accept valid one-pair smoke artifact"

    $classifiedSmokeRoot = Join-Path $runDir "one-pair-classified-smoke"
    New-Item -ItemType Directory -Force -Path $classifiedSmokeRoot | Out-Null
    [pscustomobject]@{ run_validity = "invalid_harness"; abort_signature = "harness_materialization_failure/docker_run_failure" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $classifiedSmokeRoot "sample-status.json") -Encoding UTF8
    $classifiedSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-classified-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $classifiedSmokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$classifiedSmokeGate.status -eq "pass" -and @($classifiedSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.reason -eq "classified_invalid_harness" }).Count -eq 1) "start gate did not accept classified invalid one-pair sample-status artifact"

    $noSelfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-selftests") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$noSelfTestGate.status -eq "fail" -and [string]$noSelfTestGate.first_failure_stable_code -eq "self_tests_not_run") "start gate allowed skipped self-tests without explicit allow"

    $relativeScenario = Join-Path $runDir "scenario-relative"
    New-GateScenario $relativeScenario $true
    $relativeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-relative") -ScenarioPath $relativeScenario -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$relativeGate.status -eq "fail" -and [int]$relativeGate.exit_code -eq 3) "start gate did not fail relative path contract"
    Assert-True (@($relativeGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "fail" }).Count -eq 1) "start gate did not identify path_contract failure"

    $env:TASKSPACE_MIN_FREE_BYTES = ([int64]::MaxValue).ToString()
    $diskGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-disk") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$diskGate.status -eq "fail" -and [string]$diskGate.run_validity -eq "invalid_harness") "start gate did not fail impossible disk threshold"

    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    $selfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-selftest") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -RunSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @("exit 7")
    Assert-True ([string]$selfTestGate.status -eq "fail" -and [int]$selfTestGate.exit_code -eq 3) "start gate did not fail failing self-test command"
    Assert-True ([string]$selfTestGate.first_failure_gate -eq "cheap_self_tests" -and [string]$selfTestGate.first_failure_command -eq "exit 7") "start gate did not record first failing self-test command"

    $missingScenarioGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-missing-scenario") -ScenarioPath (Join-Path $runDir "missing-scenario") -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$missingScenarioGate.status -eq "fail" -and [int]$missingScenarioGate.exit_code -eq 3 -and (Test-Path -LiteralPath $missingScenarioGate.json_path)) "start gate did not write artifacts for missing scenario"

    $taskListMissingGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-missing") -TaskListPath (Join-Path $runDir "missing-tasks.jsonl") -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$taskListMissingGate.status -eq "fail" -and [string]$taskListMissingGate.first_failure_stable_code -eq "task_list_missing") "start gate did not fail missing task list"

    $emptyTaskList = Join-Path $runDir "empty-tasks.jsonl"
    "" | Set-Content -LiteralPath $emptyTaskList -Encoding UTF8
    $taskListEmptyGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-empty") -TaskListPath $emptyTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$taskListEmptyGate.status -eq "fail" -and [string]$taskListEmptyGate.first_failure_stable_code -eq "task_list_empty") "start gate did not fail empty task list"

    $badTaskList = Join-Path $runDir "bad-tasks.jsonl"
    "{not-json" | Set-Content -LiteralPath $badTaskList -Encoding UTF8
    $taskListBadGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-bad") -TaskListPath $badTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$taskListBadGate.status -eq "fail" -and [string]$taskListBadGate.first_failure_stable_code -eq "task_list_malformed") "start gate did not fail malformed task list"

    $taskList = Join-Path $runDir "tasks.jsonl"
    ([pscustomobject]@{ task_dir = $scenarioDir; source_version = "fixture-source" } | ConvertTo-Json -Compress) | Set-Content -LiteralPath $taskList -Encoding UTF8
    $taskListGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-pass") -TaskListPath $taskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -SelfTestCommands @()
    Assert-True ([string]$taskListGate.status -eq "pass" -and @($taskListGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "skipped_allowed" }).Count -eq 1) "start gate did not require explicit skipped path-contract allow"
} finally {
    if ($null -eq $oldMinFreeBytes) { Remove-Item Env:TASKSPACE_MIN_FREE_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_BYTES = $oldMinFreeBytes }
    if ($null -eq $oldMinFreeGib) { Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_GIB = $oldMinFreeGib }
}

if ($failures.Count -gt 0) {
    Write-Host "E3 start gate self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 start gate self-test: PASS"
Write-Host "RunRoot: $runDir"
