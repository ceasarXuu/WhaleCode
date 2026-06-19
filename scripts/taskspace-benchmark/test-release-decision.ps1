param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\release-decision-selftest" }
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { [void]$script:failures.Add($Message) }
}

function Write-Json($Value, [string]$Path) {
    $dir = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function New-FixtureRun([string]$Name, [string]$CostStatus, [bool]$ScoreValid, [int]$RoutingMistakes) {
    $dir = Join-Path $RunRoot $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Write-Json ([pscustomobject]@{
            status = $CostStatus
            ratios = [pscustomobject]@{
                direct_input_output_ratio = 1.5
                walltime_ratio = 1.2
                model_request_count_ratio = 1
            }
        }) (Join-Path $dir "suite-cost-gate.json")
    Write-Json ([pscustomobject]@{ availability = "measured" }) (Join-Path $dir "token-summary.json")
    Write-Json ([pscustomobject]@{ availability = "measured" }) (Join-Path $dir "request-summary.json")
    Write-Json ([pscustomobject]@{ availability = "measured" }) (Join-Path $dir "taskspace-control-usage.json")
    Set-Content -LiteralPath (Join-Path $dir "projection-events.jsonl") -Encoding UTF8 -Value "{}"
    ([pscustomobject]@{
        schema_version = "taskspace-output-ref-event-v1"
        source = "observability_timeline"
        kind = "output_ref.created"
        artifact_ref = "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        call_id = "call-fixture"
        timestamp_ms = 1
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "output-ref-events.jsonl") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $dir "compaction-events.jsonl") -Encoding UTF8 -Value "{}"
    ([pscustomobject]@{
        schema_version = "taskspace-provider-request-event-v1"
        request_id = "provider-request-1"
        request_phase = "model_sampling"
        task_id = "task-1"
        map_id = "map-1"
        node_id = "node-1"
        status = "completed"
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "provider-request-events.jsonl") -Encoding UTF8
    ([pscustomobject]@{
        schema_version = "taskspace-budget-event-v1"
        status = "pass"
        budget_response_action_taken = $true
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "budget-events.jsonl") -Encoding UTF8
    Write-Json ([pscustomobject]@{
            provider_request_hook_coverage = 99
            request_phase_attribution_coverage = 95
            unknown_request_phase_ratio = 0
        }) (Join-Path $dir "request-phase-summary.json")
    ([pscustomobject]@{
        schema_version = "taskspace-exact-payload-scan-event-v1"
        scan_event_id = "scan-1"
        request_id = "provider-request-1"
        provider_payload_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        passed = $true
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "exact-payload-scan-events.jsonl") -Encoding UTF8
    Write-Json ([pscustomobject]@{
            provider_payload_available = $true
            provider_payload_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            exact_payload_scan_passed = $true
            exact_payload_scan_event_id = "scan-1"
            replacement_confirmed = $true
            legacy_taskspace_history_present = $false
            large_raw_output_tokens = 0
            protected_items_present = $true
        }) (Join-Path $dir "active-context-replacement-report.json")
    Write-Json ([pscustomobject]@{
            status = "pass"
            legacy_state_action_count = 0
            legacy_state_action_budget = 0
            state_commit_count = 1
        }) (Join-Path $dir "state-commit-displacement.json")
    Write-Json ([pscustomobject]@{ status = "pass"; spawn_agent_call_count = 0; max_spawn_agent_calls = 0 }) (Join-Path $dir "spawn-node-budget-summary.json")
    Write-Json ([pscustomobject]@{ status = "pass"; schema_version = 1 }) (Join-Path $dir "v005-non-agent-gates.json")
    Write-Json ([pscustomobject]@{
            schema_version = "TaskShapeRouterV1"
            recommended_mode = "thin"
            status = "report_only"
        }) (Join-Path $dir "routing-decision.json")
    Write-Json ([pscustomobject]@{
            schema_version = "taskspace-cost-diagnostics-v1"
            root_cause = "active_profile_repeats_compact_taskspace_context_across_many_model_turns"
            drivers = @("rollout_request_count_over_partial_budget")
            ratios = [pscustomobject]@{
                rollout_trace_model_request_count_ratio = 18
                uncached_input_ratio = 11.2
                projection_token_share_of_taskspace_input = 0.0087
            }
        }) (Join-Path $dir "cost-diagnostics.json")
    Write-Json ([pscustomobject]@{
            run_validity = "valid"
            score_valid = $ScoreValid
            both_success = 1
            both_failed = 0
            excluded_pairs = 0
            excluded_by_reason = [pscustomobject]@{}
        }) (Join-Path $dir "aggregate.json")
    Write-Json ([pscustomobject]@{
            taskspace_projection_count = 1
            missing_taskspace_projection_count = 0
            taskspace_projection_protected_miss_count = 0
            active_projection_count = 1
            shadow_projection_count = 0
        }) (Join-Path $dir "context-projection-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            protected_miss_count = 0
        }) (Join-Path $dir "suite-map-management-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            routing_mistake_count = $RoutingMistakes
            recommended_mode = "thin"
            router_status = "report_only"
            verification_first_expected_format_count = 0
        }) (Join-Path $dir "suite-routing-summary.json")
    Write-Json ([pscustomobject]@{
            logical_mode = "standard"
            large_output_replay_count = 0
            runtime_output_ref_created_count = 0
        }) (Join-Path $dir "pair-001\left\artifacts\metrics.json")
    Write-Json ([pscustomobject]@{
            logical_mode = "taskspace"
            large_output_replay_count = 0
            runtime_output_ref_created_count = 1
        }) (Join-Path $dir "pair-001\right\artifacts\metrics.json")
    ([pscustomobject]@{
        schema_version = "taskspace-output-ref-event-v1"
        source = "observability_timeline"
        kind = "output_ref.created"
        artifact_ref = "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        call_id = "call-fixture"
        timestamp_ms = 1
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "pair-001\right\artifacts\output-ref-events.jsonl") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $dir "pair-001\pair-report.md") -Encoding UTF8 -Value "# Pair Report"
    Write-Json ([pscustomobject]@{
            schema_version = 1
            evidence_target = "E3"
            run_validity = "valid"
            diagnostic_comparison_enabled = $true
            final_aggregate_ready = $true
            completed_pairs = 1
        }) (Join-Path $dir "run-status.json")
    $pairReportPath = Join-Path $dir "pair-001\pair-report.md"
    @(
        ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
        ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $dir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
        ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $pairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
    ) | Set-Content -LiteralPath (Join-Path $dir "events.jsonl") -Encoding UTF8
    $dir
}

$passDir = New-FixtureRun "pass" "PASS" $true 0
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $passDir *> $null
Assert-True ($LASTEXITCODE -eq 0) "PASS fixture did not exit 0"
$passDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $passDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$passDecision.decision -eq "release_pass") "PASS fixture did not write release_pass decision"
Assert-True ([bool]$passDecision.closeable) "PASS fixture did not write closeable=true"

$partialDir = New-FixtureRun "partial" "PARTIAL" $true 0
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $partialDir *> $null
Assert-True ($LASTEXITCODE -eq 2) "PARTIAL fixture did not exit 2"
$partialDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $partialDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$partialDecision.decision -eq "blocked_partial") "PARTIAL fixture did not write blocked_partial decision"
Assert-True (-not [bool]$partialDecision.closeable) "PARTIAL fixture incorrectly wrote closeable=true"
$partialMd = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $partialDir "release-decision.md")
Assert-True ($partialMd.Contains("cannot close v0.0.5")) "PARTIAL markdown did not mark blocked_partial as non-closeable"

$failDir = New-FixtureRun "fail" "FAIL" $true 1
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $failDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "FAIL fixture did not exit 1"
$failDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $failDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$failDecision.decision -eq "fail") "FAIL fixture did not write fail decision"
Assert-True (-not [bool]$failDecision.closeable) "FAIL fixture incorrectly wrote closeable=true"
Assert-True (@($failDecision.blockers) -contains "cost_gate_failed") "FAIL fixture did not report cost blocker"
Assert-True (@($failDecision.blockers) -contains "routing_gate_failed") "FAIL fixture did not report routing blocker"
Assert-True ([string]$failDecision.cost_root_cause -eq "active_profile_repeats_compact_taskspace_context_across_many_model_turns") "FAIL fixture did not preserve cost root cause"
$failMd = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $failDir "release-decision.md")
Assert-True ($failMd.Contains("rollout_trace_model_request_count_ratio: 18")) "FAIL markdown did not include cost diagnostics"

$missingArtifactDir = New-FixtureRun "missing-artifact" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingArtifactDir "output-ref-events.jsonl") -Destination (Join-Path $missingArtifactDir "output-ref-events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingArtifactDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing artifact fixture did not exit 1"
$missingArtifactDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingArtifactDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingArtifactDecision.blockers) -contains "required_artifact_missing:output-ref-events.jsonl") "missing artifact fixture did not report required artifact blocker"

$missingProviderEventDir = New-FixtureRun "missing-provider-event" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingProviderEventDir "provider-request-events.jsonl") -Destination (Join-Path $missingProviderEventDir "provider-request-events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProviderEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provider event fixture did not exit 1"
$missingProviderEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProviderEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingProviderEventDecision.blockers) -contains "provider_request_event_missing") "missing provider event fixture did not report provider blocker"

$hashOnlyReplacementDir = New-FixtureRun "hash-only-active-replacement" "PASS" $true 0
$hashOnlyReplacement = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $hashOnlyReplacementDir "active-context-replacement-report.json") | ConvertFrom-Json
$hashOnlyReplacement.exact_payload_scan_passed = $false
$hashOnlyReplacement.exact_payload_scan_event_id = ""
$hashOnlyReplacement | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $hashOnlyReplacementDir "active-context-replacement-report.json") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $hashOnlyReplacementDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "hash-only active replacement fixture did not exit 1"
$hashOnlyReplacementDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $hashOnlyReplacementDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($hashOnlyReplacementDecision.blockers) -contains "active_context_replacement_gate_failed") "hash-only fixture did not report active replacement blocker"

$emptyOutputRefDir = New-FixtureRun "empty-output-ref-events" "PASS" $true 0
Set-Content -LiteralPath (Join-Path $emptyOutputRefDir "output-ref-events.jsonl") -Encoding UTF8 -Value "{}"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $emptyOutputRefDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "empty output-ref fixture did not exit 1"
$emptyOutputRefDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $emptyOutputRefDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$emptyOutputRefDecision.output_ref_gate_pass) "empty output-ref fixture incorrectly passed output-ref gate"

$missingMetricsDir = New-FixtureRun "missing-taskspace-metrics" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingMetricsDir "pair-001\right\artifacts\metrics.json") -Destination (Join-Path $missingMetricsDir "pair-001\right\artifacts\metrics.json.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingMetricsDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing taskspace metrics fixture did not exit 1"
$missingMetricsDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingMetricsDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingMetricsDecision.run_provenance_gate_pass) "missing metrics fixture incorrectly passed provenance gate"

$missingStandardMetricsDir = New-FixtureRun "missing-standard-metrics" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingStandardMetricsDir "pair-001\left\artifacts\metrics.json") -Destination (Join-Path $missingStandardMetricsDir "pair-001\left\artifacts\metrics.json.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingStandardMetricsDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing standard metrics fixture did not exit 1"
$missingStandardMetricsDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingStandardMetricsDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingStandardMetricsDecision.run_provenance_gate_pass) "missing standard metrics fixture incorrectly passed provenance gate"

$missingProvenanceDir = New-FixtureRun "missing-provenance" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingProvenanceDir "events.jsonl") -Destination (Join-Path $missingProvenanceDir "events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProvenanceDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provenance fixture did not exit 1"
$missingProvenanceDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProvenanceDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingProvenanceDecision.run_provenance_gate_pass) "missing provenance fixture incorrectly passed provenance gate"

$minimalPairEventDir = New-FixtureRun "minimal-pair-event" "PASS" $true 0
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $minimalPairEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $minimalPairEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "minimal pair event fixture did not exit 1"
$minimalPairEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $minimalPairEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$minimalPairEventDecision.run_provenance_gate_pass) "minimal pair event fixture incorrectly passed provenance gate"

$minimalRoutingEventDir = New-FixtureRun "minimal-routing-event" "PASS" $true 0
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = (Join-Path $minimalRoutingEventDir "pair-001\pair-report.md"); reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $minimalRoutingEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $minimalRoutingEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "minimal routing event fixture did not exit 1"
$minimalRoutingEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $minimalRoutingEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$minimalRoutingEventDecision.run_provenance_gate_pass) "minimal routing event fixture incorrectly passed provenance gate"

$duplicatePairEventDir = New-FixtureRun "duplicate-pair-event" "PASS" $true 0
Write-Json ([pscustomobject]@{
        schema_version = 1
        evidence_target = "E3"
        run_validity = "valid"
        diagnostic_comparison_enabled = $true
        final_aggregate_ready = $true
        completed_pairs = 2
    }) (Join-Path $duplicatePairEventDir "run-status.json")
$duplicatePairReportPath = Join-Path $duplicatePairEventDir "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 2; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $duplicatePairEventDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $duplicatePairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:03.0000000Z"; repeat = 2; pair_report = $duplicatePairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $duplicatePairEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $duplicatePairEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "duplicate pair event fixture did not exit 1"
$duplicatePairEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $duplicatePairEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$duplicatePairEventDecision.run_provenance_gate_pass) "duplicate pair event fixture incorrectly passed provenance gate"

$nestedPairDir = New-FixtureRun "nested-pair-dir" "PASS" $true 0
$nestedPairRoot = Join-Path $nestedPairDir "backup"
New-Item -ItemType Directory -Path $nestedPairRoot -Force | Out-Null
Move-Item -LiteralPath (Join-Path $nestedPairDir "pair-001") -Destination (Join-Path $nestedPairRoot "pair-001") -Force
$nestedPairReportPath = Join-Path $nestedPairRoot "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $nestedPairDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $nestedPairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $nestedPairDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $nestedPairDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "nested pair dir fixture did not exit 1"
$nestedPairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $nestedPairDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$nestedPairDecision.run_provenance_gate_pass) "nested pair dir fixture incorrectly passed provenance gate"

$nonE3PairDir = New-FixtureRun "non-e3-pair-level" "PASS" $true 0
$nonE3PairReportPath = Join-Path $nonE3PairDir "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $nonE3PairDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $nonE3PairReportPath; reported_evidence_level = "E1" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $nonE3PairDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $nonE3PairDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "non-E3 pair level fixture did not exit 1"
$nonE3PairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $nonE3PairDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$nonE3PairDecision.run_provenance_gate_pass) "non-E3 pair level fixture incorrectly passed provenance gate"

$staleRoutingDir = New-FixtureRun "stale-routing-event" "PASS" $true 0
Add-Content -LiteralPath (Join-Path $staleRoutingDir "events.jsonl") -Encoding UTF8 -Value (([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:03.0000000Z" } | ConvertTo-Json -Compress -Depth 8))
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $staleRoutingDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "stale routing event fixture did not exit 1"
$staleRoutingDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $staleRoutingDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$staleRoutingDecision.run_provenance_gate_pass) "stale routing event fixture incorrectly passed provenance gate"

$uncorrelatedOutputRefDir = New-FixtureRun "uncorrelated-output-ref" "PASS" $true 0
Set-Content -LiteralPath (Join-Path $uncorrelatedOutputRefDir "pair-001\right\artifacts\output-ref-events.jsonl") -Encoding UTF8 -Value "{}"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $uncorrelatedOutputRefDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "uncorrelated output-ref fixture did not exit 1"
$uncorrelatedOutputRefDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $uncorrelatedOutputRefDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$uncorrelatedOutputRefDecision.output_ref_gate_pass) "uncorrelated output-ref fixture incorrectly passed output-ref gate"

$stringFalseDir = New-FixtureRun "string-false-quality" "PASS" $true 0
Write-Json ([pscustomobject]@{
        run_validity = "valid"
        score_valid = "false"
        both_success = 1
        both_failed = 0
        excluded_pairs = 0
        excluded_by_reason = [pscustomobject]@{}
    }) (Join-Path $stringFalseDir "aggregate.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $stringFalseDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "string false quality fixture did not exit 1"
$stringFalseDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stringFalseDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$stringFalseDecision.quality_gate_pass) "string false quality fixture incorrectly passed quality gate"

if ($failures.Count -gt 0) {
    Write-Error ("Release decision self-test failed: " + (@($failures.ToArray()) -join "; "))
    exit 1
}
Write-Host "Release decision self-test: PASS"
Write-Host "RunRoot: $RunRoot"
