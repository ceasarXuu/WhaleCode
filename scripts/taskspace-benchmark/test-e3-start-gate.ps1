param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")
. (Join-Path $PSScriptRoot "lib\e3-identity.ps1")
. (Join-Path $PSScriptRoot "lib\runtime-reconstruction.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-start-gate-selftest" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
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

function New-CalibrationFixtures {
    param([string]$Root, [string]$TaskListHash = "task-list-a", [string]$SourceVersion = "source-a", [string]$ProfileHash = "profile-a")
    $onePairRoot = Join-Path $Root "one-pair"
    $serialRoot = Join-Path $Root "serial"
    New-Item -ItemType Directory -Force -Path $onePairRoot, $serialRoot | Out-Null
    [pscustomobject]@{ score_valid = $true; run_validity = "valid"; clean_comparable_pair_count = 1 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "aggregate.json") -Encoding UTF8
    $onePairTimingPath = Join-Path $onePairRoot "pair-timing.json"
    $onePairReportPath = Join-Path $onePairRoot "runtime-bottleneck.md"
    $onePairReportJsonPath = Join-Path $onePairRoot "runtime-bottleneck.json"
    [pscustomobject]@{
        agent_duration_ms = 1000
        public_validation_duration_ms = 2000
        bottleneck_classification = "validator_bound"
        runtime_optimization_status = "ready"
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $onePairTimingPath -Encoding UTF8
    [pscustomobject]@{ sample_id = "calibration-one-pair" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Encoding UTF8
    "# Runtime Bottleneck`n" | Set-Content -LiteralPath $onePairReportPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = 1
        timing_path = $onePairTimingPath
        report_path = $onePairReportPath
        score_valid = $true
        speedup_evidence_valid = $true
        speedup_decision = "speedup_candidate_validator_or_docker"
        timing_quality = "complete"
        runtime_optimization_status = "ready"
        wait_attribution_status = "complete"
        generated_at = "2026-06-15T00:00:00.0000000Z"
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $onePairReportJsonPath -Encoding UTF8
    $suiteTimingPath = Join-Path $serialRoot "suite-timing.json"
    $serialReportPath = Join-Path $serialRoot "runtime-calibration-report.md"
    $serialReportJsonPath = Join-Path $serialRoot "runtime-calibration-report.json"
    [pscustomobject]@{
        sample_count = 3
        timing_quality = "complete"
        runtime_optimization_status = "ready"
        bottleneck_classification = "mixed_or_unclassified"
        wait_attribution_status = "complete"
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $suiteTimingPath -Encoding UTF8
    "# Runtime Calibration`n" | Set-Content -LiteralPath $serialReportPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = 1
        report_path = $serialReportPath
        score_valid = $true
        speedup_evidence_valid = $true
        speedup_decision = "speedup_candidate_parallelism"
        timing_path = $suiteTimingPath
        timing_quality = "complete"
        runtime_optimization_status = "ready"
        wait_attribution_status = "complete"
        generated_at = "2026-06-15T00:00:00.0000000Z"
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $serialReportJsonPath -Encoding UTF8
    $equivalencePath = Join-Path $Root "serial-vs-parallel-equivalence.json"
    [pscustomobject]@{
        comparable = $true
        parallel_smoke_score_drift = $false
        drift_count = 0
        compared_sample_ids = @("sample-a", "sample-b", "sample-c")
        required_sample_fields = @("sample_id", "run_validity")
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $equivalencePath -Encoding UTF8
    [pscustomobject]@{ one_pair_root = $onePairRoot; serial_root = $serialRoot; equivalence_path = $equivalencePath }
}

function New-V005MarkerFixtures {
    param([string]$Root, [string]$TaskListHash = "task-list-a", [string]$SourceVersion = "source-a", [string]$ProfileHash = "profile-a")
    New-Item -ItemType Directory -Force -Path $Root | Out-Null
    $nonAgentPath = Join-Path $Root "non-agent-gates.json"
    $codeCompletePath = Join-Path $Root "code-complete.json"
    $userApprovalPath = Join-Path $Root "user-approval.json"
    $head = (& git -C $repoRoot rev-parse HEAD)
    $now = (Get-Date).ToString("o")
    $identity = @{
        schema_version = 1
        status = "pass"
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
        generated_at = $now
        producer = "test-e3-start-gate"
    }
    $gateObject = {
        param([string]$Name)
        [pscustomobject]@{ status = "pass"; evidence_path = "selftest://$Name"; command = "selftest"; generated_at = $now }
    }
    [pscustomobject]($identity + @{
            gates = [pscustomobject]@{
                provider_request_hook = (& $gateObject "provider_request_hook")
                runtime_budget_response = (& $gateObject "runtime_budget_response")
                active_context_replacement = (& $gateObject "active_context_replacement")
                state_commit_displacement = (& $gateObject "state_commit_displacement")
                spawn_node_budget = (& $gateObject "spawn_node_budget")
                request_phase_attribution = (& $gateObject "request_phase_attribution")
                release_decision_fixture = (& $gateObject "release_decision_fixture")
                start_gate_fixture = (& $gateObject "start_gate_fixture")
            }
        }) | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $nonAgentPath -Encoding UTF8
    [pscustomobject]($identity + @{
            git_commit = [string]$head
            unfinished_p0_items = @()
            test_outputs = @("selftest")
        }) | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $codeCompletePath -Encoding UTF8
    [pscustomobject]($identity + @{
            approved_command_category = "full_e3"
            approved_sample_set_id = "terminal-bench_E3-P0_3_5"
            approval_source = "selftest"
            approval_timestamp = $now
        }) | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $userApprovalPath -Encoding UTF8
    [pscustomobject]@{ non_agent_path = $nonAgentPath; code_complete_path = $codeCompletePath; user_approval_path = $userApprovalPath }
}

$oldMinFreeBytes = $env:TASKSPACE_MIN_FREE_BYTES
$oldMinFreeGib = $env:TASKSPACE_MIN_FREE_GIB
try {
    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue

    $scenarioDir = Join-Path $runDir "scenario-pass"
    New-GateScenario $scenarioDir
    $gate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-pass") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$gate.status -eq "pass" -and [int]$gate.exit_code -eq 0) "start gate did not pass clean fixture"
    Assert-True (Test-Path -LiteralPath $gate.json_path) "start gate did not write json artifact"
    Assert-True (Test-Path -LiteralPath $gate.markdown_path) "start gate did not write markdown artifact"

    $noSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-smoke") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$noSmokeGate.status -eq "fail" -and @($noSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "fail" -and [string]$_.stable_code -eq "one_pair_smoke_not_provided" }).Count -eq 1) "start gate allowed missing one-pair smoke without explicit allow"

    $smokeRoot = Join-Path $runDir "one-pair-smoke"
    New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null
    [pscustomobject]@{ score_valid = $true; run_validity = "valid"; clean_comparable_pair_count = 1 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $smokeRoot "aggregate.json") -Encoding UTF8
    $smokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $smokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$smokeGate.status -eq "pass" -and @($smokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate did not accept valid one-pair smoke artifact when calibration gate is explicitly skipped"
    $aggregateOnlyCalibrationGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-aggregate-only-calibration-fail") -ScenarioPath $scenarioDir -OnePairSmokeRoot $smokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$aggregateOnlyCalibrationGate.status -eq "fail" -and @($aggregateOnlyCalibrationGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "start gate allowed aggregate-only one-pair root without timing calibration artifacts"
    $aggregateOnlyDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregateOnlyCalibrationGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$aggregateOnlyDecision.status -eq "blocked" -and [string]$aggregateOnlyDecision.next_allowed_command_category -eq "serial_calibration") "gate-decision did not route missing calibration artifacts back to calibration"

    $calibration = New-CalibrationFixtures (Join-Path $runDir "calibration-fixtures")
    $v005MarkerRoot = Join-Path $runDir "v005-markers"
    $v005Markers = New-V005MarkerFixtures $v005MarkerRoot
    $v005NonAgentPath = $v005Markers.non_agent_path
    $v005CodeCompletePath = $v005Markers.code_complete_path
    $v005UserApprovalPath = $v005Markers.user_approval_path
    $calibratedGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$calibratedGate.status -eq "pass" -and @($calibratedGate.gates | Where-Object { [string]$_.name -eq "calibration_parallel_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate did not pass complete calibration evidence"
    Assert-True (Test-Path -LiteralPath $calibratedGate.gate_decision_path) "start gate did not write gate-decision artifact"
    $gateDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $calibratedGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$gateDecision.status -eq "pass" -and [string]$gateDecision.task_list_hash -eq "task-list-a" -and [string]$gateDecision.profile_hash -eq "profile-a") "gate-decision did not preserve expected identity"
    Assert-True ([string]$gateDecision.next_allowed_command_category -eq "targeted_diagnostic" -and -not [bool]$gateDecision.full_e3_allowed -and -not [bool]$gateDecision.v005_markers_passed) "gate-decision authorized full E3 without v0.0.5 markers"
    $calibratedWithMarkersGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-markers-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $v005NonAgentPath -V005CodeCompleteMarkerPath $v005CodeCompletePath -V005UserApprovalMarkerPath $v005UserApprovalPath -AllowSkippedSelfTests -SelfTestCommands @()
    $gateDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $calibratedWithMarkersGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$gateDecision.next_allowed_command_category -eq "full_e3" -and [bool]$gateDecision.full_e3_allowed -and [bool]$gateDecision.speed_claim_allowed -and [bool]$gateDecision.calibration_gate_passed -and [bool]$gateDecision.v005_markers_passed) "gate-decision did not authorize full E3 after complete calibration and v0.0.5 markers"
    $calibratedMarkdown = Get-Content -Raw -Encoding UTF8 -LiteralPath $calibratedWithMarkersGate.markdown_path
    Assert-True ($calibratedMarkdown -match "next_allowed_command_category: full_e3" -and $calibratedMarkdown -match "full_e3_allowed: True" -and $calibratedMarkdown -match "speed_claim_allowed: True") "start gate markdown did not expose gate-decision summary"
    $spoofedMarkerPath = Join-Path $v005MarkerRoot "spoofed-marker.json"
    "user-approved" | Set-Content -LiteralPath $spoofedMarkerPath -Encoding UTF8
    $spoofedGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-spoofed-marker") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $v005NonAgentPath -V005CodeCompleteMarkerPath $spoofedMarkerPath -V005UserApprovalMarkerPath $v005UserApprovalPath -AllowSkippedSelfTests -SelfTestCommands @()
    $spoofedDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $spoofedGate.gate_decision_path | ConvertFrom-Json
    Assert-True (-not [bool]$spoofedDecision.full_e3_allowed -and @($spoofedGate.gates | Where-Object { [string]$_.name -eq "v005_code_complete" -and [string]$_.stable_code -eq "v005_code_complete_malformed" }).Count -eq 1) "start gate accepted spoofed arbitrary marker text"
    $mismatchedMarkers = New-V005MarkerFixtures (Join-Path $runDir "v005-mismatched-markers") -TaskListHash "task-list-b"
    $mismatchedGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-mismatched-marker") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $mismatchedMarkers.non_agent_path -V005CodeCompleteMarkerPath $mismatchedMarkers.code_complete_path -V005UserApprovalMarkerPath $mismatchedMarkers.user_approval_path -AllowSkippedSelfTests -SelfTestCommands @()
    $mismatchedDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $mismatchedGate.gate_decision_path | ConvertFrom-Json
    Assert-True (-not [bool]$mismatchedDecision.full_e3_allowed -and @($mismatchedGate.gates | Where-Object { [string]$_.stable_code -like "*task_list_hash_mismatch" }).Count -gt 0) "start gate accepted mismatched marker identity"
    $staleMarkers = New-V005MarkerFixtures (Join-Path $runDir "v005-stale-markers")
    $staleCodeMarker = Get-Content -Raw -Encoding UTF8 -LiteralPath $staleMarkers.code_complete_path | ConvertFrom-Json
    $staleCodeMarker.generated_at = (Get-Date).AddDays(-3).ToString("o")
    $staleCodeMarker | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $staleMarkers.code_complete_path -Encoding UTF8
    $staleGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-stale-marker") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $staleMarkers.non_agent_path -V005CodeCompleteMarkerPath $staleMarkers.code_complete_path -V005UserApprovalMarkerPath $staleMarkers.user_approval_path -AllowSkippedSelfTests -SelfTestCommands @()
    $staleDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $staleGate.gate_decision_path | ConvertFrom-Json
    Assert-True (-not [bool]$staleDecision.full_e3_allowed -and @($staleGate.gates | Where-Object { [string]$_.stable_code -eq "v005_code_complete_generated_at_stale" }).Count -eq 1) "start gate accepted stale v0.0.5 marker"
    $skippedCalibrationDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $smokeGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$skippedCalibrationDecision.next_allowed_command_category -eq "targeted_diagnostic" -and -not [bool]$skippedCalibrationDecision.full_e3_allowed -and -not [bool]$skippedCalibrationDecision.speed_claim_allowed -and -not [bool]$skippedCalibrationDecision.v005_markers_passed) "gate-decision authorized full E3 when calibration or v0.0.5 markers were skipped"
    $blockedCalibration = New-CalibrationFixtures (Join-Path $runDir "calibration-speedup-evidence-fail")
    $blockedCalibrationReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $blockedCalibration.serial_root "runtime-calibration-report.json") | ConvertFrom-Json
    $blockedCalibrationReport.speedup_evidence_valid = $false
    $blockedCalibrationReport.speedup_decision = "speedup_blocked_instrumentation"
    $blockedCalibrationReport | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $blockedCalibration.serial_root "runtime-calibration-report.json") -Encoding UTF8
    $blockedCalibrationGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-speedup-evidence-fail") -ScenarioPath $scenarioDir -OnePairSmokeRoot $blockedCalibration.one_pair_root -SerialCalibrationRoot $blockedCalibration.serial_root -ParallelEquivalencePath $blockedCalibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $v005NonAgentPath -V005CodeCompleteMarkerPath $v005CodeCompletePath -V005UserApprovalMarkerPath $v005UserApprovalPath -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$blockedCalibrationGate.status -eq "pass" -and @($blockedCalibrationGate.gates | Where-Object { [string]$_.name -eq "calibration_serial_calibration" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate blocked full E3 on instrumentation-only speed evidence failure"
    $blockedCalibrationDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $blockedCalibrationGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$blockedCalibrationDecision.status -eq "pass" -and [bool]$blockedCalibrationDecision.full_e3_allowed -and -not [bool]$blockedCalibrationDecision.speed_claim_allowed) "gate-decision did not decouple speed claim from full E3 eligibility"
    $blockedOnePairCalibration = New-CalibrationFixtures (Join-Path $runDir "calibration-one-pair-blocked-decision")
    $blockedOnePairReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $blockedOnePairCalibration.one_pair_root "runtime-bottleneck.json") | ConvertFrom-Json
    $blockedOnePairReport.speedup_decision = "speedup_blocked_instrumentation"
    $blockedOnePairReport | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $blockedOnePairCalibration.one_pair_root "runtime-bottleneck.json") -Encoding UTF8
    $blockedOnePairStartGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-one-pair-blocked-decision") -ScenarioPath $scenarioDir -OnePairSmokeRoot $blockedOnePairCalibration.one_pair_root -SerialCalibrationRoot $blockedOnePairCalibration.serial_root -ParallelEquivalencePath $blockedOnePairCalibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -V005NonAgentGatesPath $v005NonAgentPath -V005CodeCompleteMarkerPath $v005CodeCompletePath -V005UserApprovalMarkerPath $v005UserApprovalPath -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$blockedOnePairStartGate.status -eq "pass" -and @($blockedOnePairStartGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate blocked full E3 on one-pair instrumentation-only speed decision"
    $blockedOnePairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $blockedOnePairStartGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$blockedOnePairDecision.status -eq "pass" -and [bool]$blockedOnePairDecision.full_e3_allowed -and -not [bool]$blockedOnePairDecision.speed_claim_allowed) "gate-decision did not decouple one-pair speed claim from full E3 eligibility"
    $invalidRunCalibration = New-CalibrationFixtures (Join-Path $runDir "calibration-invalid-run-fail")
    $invalidRunReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $invalidRunCalibration.serial_root "runtime-calibration-report.json") | ConvertFrom-Json
    $invalidRunReport.score_valid = $false
    $invalidRunReport.speedup_evidence_valid = $false
    $invalidRunReport.speedup_decision = "speedup_blocked_invalid_run"
    $invalidRunReport.runtime_optimization_status = "blocked"
    $invalidRunReport | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $invalidRunCalibration.serial_root "runtime-calibration-report.json") -Encoding UTF8
    $invalidRunStartGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-invalid-run-fail") -ScenarioPath $scenarioDir -OnePairSmokeRoot $invalidRunCalibration.one_pair_root -SerialCalibrationRoot $invalidRunCalibration.serial_root -ParallelEquivalencePath $invalidRunCalibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$invalidRunStartGate.status -eq "fail" -and @($invalidRunStartGate.gates | Where-Object { [string]$_.name -eq "calibration_serial_calibration" -and [string]$_.stable_code -eq "serial_calibration_field_invalid:score_valid" }).Count -eq 1) "start gate allowed invalid-run calibration evidence"
    $invalidRunDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $invalidRunStartGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$invalidRunDecision.status -eq "blocked" -and -not [bool]$invalidRunDecision.full_e3_allowed -and [string]$invalidRunDecision.next_allowed_command_category -eq "serial_calibration") "gate-decision did not route invalid-run calibration evidence back to calibration"
    $missingTimingPathCalibration = New-CalibrationFixtures (Join-Path $runDir "calibration-missing-timing-path")
    $missingTimingPathReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingTimingPathCalibration.serial_root "runtime-calibration-report.json") | ConvertFrom-Json
    $missingTimingPathReport.PSObject.Properties.Remove("timing_path")
    $missingTimingPathReport | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $missingTimingPathCalibration.serial_root "runtime-calibration-report.json") -Encoding UTF8
    $missingTimingPathStartGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-missing-timing-path") -ScenarioPath $scenarioDir -OnePairSmokeRoot $missingTimingPathCalibration.one_pair_root -SerialCalibrationRoot $missingTimingPathCalibration.serial_root -ParallelEquivalencePath $missingTimingPathCalibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-a" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$missingTimingPathStartGate.status -eq "fail" -and @($missingTimingPathStartGate.gates | Where-Object { [string]$_.name -eq "calibration_serial_calibration" -and [string]$_.stable_code -eq "serial_calibration_field_missing:timing_path" }).Count -eq 1) "start gate allowed serial calibration report without timing_path"
    $missingTimingPathDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath $missingTimingPathStartGate.gate_decision_path | ConvertFrom-Json
    Assert-True ([string]$missingTimingPathDecision.status -eq "blocked" -and [string]$missingTimingPathDecision.next_allowed_command_category -eq "serial_calibration") "gate-decision did not route missing timing_path calibration failure back to calibration"
    $identityMismatchStartGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-identity-fail") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -ExpectedTaskListHash "task-list-b" -SourceVersion "source-a" -ExpectedProfileHash "profile-a" -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$identityMismatchStartGate.status -eq "fail" -and @($identityMismatchStartGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.stable_code -eq "one_pair_smoke_identity_mismatch:task_list_hash" }).Count -eq 1) "start gate did not fail closed on calibration identity mismatch"

    $classifiedSmokeRoot = Join-Path $runDir "one-pair-classified-smoke"
    New-Item -ItemType Directory -Force -Path $classifiedSmokeRoot | Out-Null
    [pscustomobject]@{ run_validity = "invalid_harness"; abort_signature = "harness_materialization_failure/docker_run_failure" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $classifiedSmokeRoot "sample-status.json") -Encoding UTF8
    $classifiedSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-classified-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $classifiedSmokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$classifiedSmokeGate.status -eq "pass" -and @($classifiedSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.reason -eq "classified_invalid_harness" }).Count -eq 1) "start gate did not accept classified invalid one-pair sample-status artifact"

    $noSelfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-selftests") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$noSelfTestGate.status -eq "fail" -and [string]$noSelfTestGate.first_failure_stable_code -eq "self_tests_not_run") "start gate allowed skipped self-tests without explicit allow"

    $relativeScenario = Join-Path $runDir "scenario-relative"
    New-GateScenario $relativeScenario $true
    $relativeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-relative") -ScenarioPath $relativeScenario -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$relativeGate.status -eq "fail" -and [int]$relativeGate.exit_code -eq 3) "start gate did not fail relative path contract"
    Assert-True (@($relativeGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "fail" }).Count -eq 1) "start gate did not identify path_contract failure"

    $env:TASKSPACE_MIN_FREE_BYTES = ([int64]::MaxValue).ToString()
    $diskGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-disk") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$diskGate.status -eq "fail" -and [string]$diskGate.run_validity -eq "invalid_harness") "start gate did not fail impossible disk threshold"

    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    $selfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-selftest") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -RunSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @("exit 7")
    Assert-True ([string]$selfTestGate.status -eq "fail" -and [int]$selfTestGate.exit_code -eq 3) "start gate did not fail failing self-test command"
    Assert-True ([string]$selfTestGate.first_failure_gate -eq "cheap_self_tests" -and [string]$selfTestGate.first_failure_command -eq "exit 7") "start gate did not record first failing self-test command"

    $missingScenarioGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-missing-scenario") -ScenarioPath (Join-Path $runDir "missing-scenario") -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$missingScenarioGate.status -eq "fail" -and [int]$missingScenarioGate.exit_code -eq 3 -and (Test-Path -LiteralPath $missingScenarioGate.json_path)) "start gate did not write artifacts for missing scenario"

    $taskListMissingGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-missing") -TaskListPath (Join-Path $runDir "missing-tasks.jsonl") -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListMissingGate.status -eq "fail" -and [string]$taskListMissingGate.first_failure_stable_code -eq "task_list_missing") "start gate did not fail missing task list"

    $emptyTaskList = Join-Path $runDir "empty-tasks.jsonl"
    "" | Set-Content -LiteralPath $emptyTaskList -Encoding UTF8
    $taskListEmptyGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-empty") -TaskListPath $emptyTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListEmptyGate.status -eq "fail" -and [string]$taskListEmptyGate.first_failure_stable_code -eq "task_list_empty") "start gate did not fail empty task list"

    $badTaskList = Join-Path $runDir "bad-tasks.jsonl"
    "{not-json" | Set-Content -LiteralPath $badTaskList -Encoding UTF8
    $taskListBadGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-bad") -TaskListPath $badTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListBadGate.status -eq "fail" -and [string]$taskListBadGate.first_failure_stable_code -eq "task_list_malformed") "start gate did not fail malformed task list"

    $taskList = Join-Path $runDir "tasks.jsonl"
    ([pscustomobject]@{ task_dir = $scenarioDir; source_version = "fixture-source" } | ConvertTo-Json -Compress) | Set-Content -LiteralPath $taskList -Encoding UTF8
    $taskListGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-pass") -TaskListPath $taskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListGate.status -eq "pass" -and @($taskListGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "skipped_allowed" }).Count -eq 1) "start gate did not require explicit skipped path-contract allow"

    $suiteGateRoot = Join-Path $runDir "suite-start-gate"
    $suiteGateOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot $suiteGateRoot -ScoringMode 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite start gate did not fail closed before scoring run"
    $suiteRootLine = @($suiteGateOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteRunRoot = ([string]$suiteRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteHealthPath = Join-Path $suiteRunRoot "suite-health.json"
    $suiteTimingPath = Join-Path $suiteRunRoot "suite-timing.json"
    $suiteStartGatePath = Join-Path $suiteRunRoot "start-gate\e3-start-gate.json"
    Assert-True ((Test-Path -LiteralPath $suiteHealthPath) -and (Test-Path -LiteralPath $suiteTimingPath) -and (Test-Path -LiteralPath $suiteStartGatePath)) "suite start gate did not write health, timing, and gate artifacts"
    $suiteHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteHealthPath | ConvertFrom-Json
    $suiteStartGate = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteStartGatePath | ConvertFrom-Json
    $sampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $suiteRunRoot "samples") -Directory -ErrorAction SilentlyContinue)
    Assert-True ([string]$suiteHealth.status -eq "invalid_harness" -and -not [bool]$suiteHealth.suite_score_valid) "suite start gate health did not mark invalid_harness"
    Assert-True ([string]$suiteStartGate.status -eq "fail" -and @($suiteStartGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "suite start gate did not preserve one-pair smoke failure"
    Assert-True (@($suiteStartGate.gates | Where-Object { [string]$_.name -eq "cheap_self_tests" -and [string]$_.status -eq "skipped" -and [string]$_.stable_code -eq "self_tests_skipped_after_previous_failure" }).Count -eq 1) "suite start gate ran self-tests after an earlier gate failure"
    Assert-True ($sampleDirs.Count -eq 0) "suite start gate created sample runs after gate failure"
    $suiteReconstruction = Write-TaskspaceRuntimeReconstruction -SuiteRoot $suiteRunRoot -OutputRoot (Join-Path $suiteRunRoot "runtime-reconstruction\selftest")
    Assert-True (@($suiteReconstruction.artifact.missing_fields | Where-Object { [string]$_ -eq "suite-timing.json" }).Count -eq 0) "early start-gate abort reconstruction still misses suite-timing.json"
    Assert-True ([int]$suiteReconstruction.artifact.first_invalid_sample_index -eq 0) "early start-gate abort reconstruction did not identify first invalid sample"
    Assert-True (@($suiteReconstruction.artifact.sample_rows).Count -eq 1) "early start-gate abort reconstruction did not preserve suite sample row"

    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $skipScoringOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $runDir "suite-skip-scoring") -ScoringMode -SkipStartGate 2>&1
    $skipScoringExit = $LASTEXITCODE
    $skipRequireOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $runDir "suite-skip-require") -RequireScoreValidity -SkipStartGate 2>&1
    $skipRequireExit = $LASTEXITCODE
    $ErrorActionPreference = $oldErrorActionPreference
    Assert-True ($skipScoringExit -eq 4 -and ($skipScoringOutput -join "`n") -match "SkipStartGate is not allowed") "suite allowed SkipStartGate for ScoringMode run"
    Assert-True ($skipRequireExit -eq 4 -and ($skipRequireOutput -join "`n") -match "SkipStartGate is not allowed") "suite allowed SkipStartGate for RequireScoreValidity run"

    $suiteTaskListHash = Get-TaskspaceFileSha256 $taskList
    $suiteProfileIdentity = New-TaskspaceE3ProfileIdentity `
        -Benchmark terminal-bench `
        -SourceVersion selftest `
        -Model "deepseek-v4-flash" `
        -Repeats 5 `
        -TimeoutSeconds 900 `
        -ValidationTimeoutSeconds 420 `
        -ValidationPretestTimeoutSeconds 120 `
        -ValidationTestTimeoutSeconds 420 `
        -SandboxMode "full-auto" `
        -ConfigOverride @('model_reasoning_effort="max"') `
        -EnableDockerImageCache $false `
        -MaxParallelSamples 1 `
        -MaxParallelPairsPerSample 1 `
        -MaxParallelValidationsPerPair 1 `
        -MaxDockerConcurrency 1 `
        -MaxModelConcurrency 1
    $suiteProfileHash = [string]$suiteProfileIdentity.profile_hash
    $suiteCompleteCalibration = New-CalibrationFixtures (Join-Path $runDir "suite-complete-calibration-missing-markers") -TaskListHash $suiteTaskListHash -SourceVersion "selftest" -ProfileHash $suiteProfileHash
    $suiteMissingMarkersRoot = Join-Path $runDir "suite-missing-v005-markers"
    $suiteMissingMarkersOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot $suiteMissingMarkersRoot -ScoringMode -OnePairSmokeRoot $suiteCompleteCalibration.one_pair_root -SerialCalibrationRoot $suiteCompleteCalibration.serial_root -ParallelEquivalencePath $suiteCompleteCalibration.equivalence_path 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite runner scheduled full E3 when v0.0.5 markers were missing"
    $suiteMissingMarkersRootLine = @($suiteMissingMarkersOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteMissingMarkersRunRoot = ([string]$suiteMissingMarkersRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteMissingMarkersStartGate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteMissingMarkersRunRoot "start-gate\e3-start-gate.json") | ConvertFrom-Json
    $suiteMissingMarkersSampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $suiteMissingMarkersRunRoot "samples") -Directory -ErrorAction SilentlyContinue)
    Assert-True ([bool]$suiteMissingMarkersStartGate.gate_decision.calibration_gate_passed -and -not [bool]$suiteMissingMarkersStartGate.gate_decision.v005_markers_passed -and -not [bool]$suiteMissingMarkersStartGate.gate_decision.full_e3_allowed) "suite missing-marker start gate did not preserve blocked full_e3 decision"
    Assert-True ($suiteMissingMarkersSampleDirs.Count -eq 0) "suite runner created sample runs despite full_e3_allowed=false"

    $suiteCalibrationRoot = Join-Path $runDir "suite-calibration-gate"
    $suiteCalibrationOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot $suiteCalibrationRoot -ScoringMode -OnePairSmokeRoot $smokeRoot 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite start gate did not fail closed when calibration artifacts were missing"
    $suiteCalibrationRootLine = @($suiteCalibrationOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteCalibrationRunRoot = ([string]$suiteCalibrationRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteCalibrationStartGate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteCalibrationRunRoot "start-gate\e3-start-gate.json") | ConvertFrom-Json
    Assert-True (Test-Path -LiteralPath (Join-Path $suiteCalibrationRunRoot "suite-timing.json")) "suite calibration gate failure did not write suite-timing.json"
    $suiteCalibrationSampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $suiteCalibrationRunRoot "samples") -Directory -ErrorAction SilentlyContinue)
    Assert-True (@($suiteCalibrationStartGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "suite start gate did not preserve calibration one-pair timing failure"
    Assert-True ($suiteCalibrationSampleDirs.Count -eq 0) "suite calibration gate created sample runs after gate failure"
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
