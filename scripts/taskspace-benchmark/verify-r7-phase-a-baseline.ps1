param(
    [string]$ResultPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($ResultPath)) {
    $ResultPath = Join-Path $repoRoot "benchmarks/taskspace/r7/phase-a-baseline-result.json"
}

. (Join-Path $PSScriptRoot "lib/cost-instrumentation.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message. expected=$Expected actual=$Actual"
    }
}

function Read-Json {
    param([string]$Path)
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json -Depth 50
}

function Get-Sha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Assert-JsonEquivalent {
    param($Actual, $Expected, [string]$Message)
    $actualJson = $Actual | ConvertTo-Json -Compress -Depth 50
    $expectedJson = $Expected | ConvertTo-Json -Compress -Depth 50
    Assert-Equal $actualJson $expectedJson $Message
}

$result = Read-Json $ResultPath
Assert-Equal $result.schema_version 1 "Unexpected result schema"
Assert-Equal $result.status "phase_a_complete" "Phase A result is not complete"
Assert-Equal $result.production_behavior_changed $false "Phase A must not change production behavior"
Assert-Equal $result.phase_b_ready $true "Phase B readiness gate failed"

$contractPath = Join-Path $repoRoot ([string]$result.baseline_contract.path)
Assert-True (Test-Path -LiteralPath $contractPath -PathType Leaf) "Baseline contract missing"
Assert-Equal (Get-Sha256 $contractPath) $result.baseline_contract.sha256 "Baseline contract hash drifted"
$baselineContract = Read-Json $contractPath
Assert-Equal $baselineContract.status "frozen" "Baseline contract is not frozen"
Assert-Equal $baselineContract.repeats_per_arm 1 "Phase A baseline repeat contract drifted"

$binaryPath = Join-Path $repoRoot ([string]$result.frozen_runtime.binary_path)
$attestationPath = Join-Path $repoRoot ([string]$result.frozen_runtime.attestation_path)
Assert-Equal (Get-Sha256 $binaryPath) $result.frozen_runtime.binary_sha256 "Frozen binary hash drifted"
Assert-Equal (Get-Sha256 $attestationPath) $result.frozen_runtime.attestation_sha256 "Binary attestation hash drifted"
$attestation = Read-Json $attestationPath
Assert-Equal $attestation.status "pass" "Binary attestation did not pass"
Assert-Equal $attestation.codex_source_latest_commit $result.frozen_runtime.runtime_commit "Runtime commit drifted"

$verifiedArmCount = 0
foreach ($scenario in @($result.scenarios)) {
    $runRoot = Join-Path $repoRoot ([string]$scenario.run_root)
    $pairRoot = Join-Path $runRoot ([string]$scenario.pair_id)
    $audit = Read-Json (Join-Path $pairRoot "audit.json")
    Assert-Equal $audit.run_score_valid $true "$($scenario.scenario_id) audit invalid"
    Assert-Equal $audit.engineering_unclean $false "$($scenario.scenario_id) engineering unclean"
    Assert-Equal $audit.outcome_standard "solved" "$($scenario.scenario_id) Standard did not solve"
    Assert-Equal $audit.outcome_taskspace "solved" "$($scenario.scenario_id) R6 did not solve"

    foreach ($armName in @("standard", "r6")) {
        $expected = $scenario.arms.$armName
        $artifactDir = Join-Path $pairRoot "$($expected.side)/artifacts"
        $metrics = Read-Json (Join-Path $artifactDir "metrics.json")
        $graph = Read-Json (Join-Path $artifactDir "graph-health.json")
        $projection = Read-Json (Join-Path $artifactDir "context-projection-summary.json")
        $wirePath = Join-Path $artifactDir "provider-wire-trace.jsonl"
        $storedSummary = Read-Json (Join-Path $artifactDir "provider-cache-trace-summary.json")
        $recomputed = New-TaskspaceProviderCacheTraceArtifacts @() $wirePath

        Assert-JsonEquivalent $recomputed.provider_cache_trace_summary $storedSummary "$($scenario.scenario_id)/$armName wire summary recompute drifted"
        Assert-Equal (Get-Sha256 $wirePath) $expected.wire_trace_sha256 "$($scenario.scenario_id)/$armName wire hash drifted"
        Assert-Equal $metrics.logical_mode $expected.logical_mode "$($scenario.scenario_id)/$armName logical mode drifted"
        Assert-Equal $metrics.business_success $true "$($scenario.scenario_id)/$armName business result failed"
        Assert-Equal $metrics.model_request_count $expected.provider_requests "$($scenario.scenario_id)/$armName request count drifted"
        Assert-Equal $metrics.tool_call_count $expected.total_tool_calls "$($scenario.scenario_id)/$armName tool count drifted"
        Assert-Equal $metrics.taskspace_control_count $expected.control_calls "$($scenario.scenario_id)/$armName control count drifted"
        Assert-Equal $metrics.input_tokens $expected.input_tokens "$($scenario.scenario_id)/$armName input tokens drifted"
        Assert-Equal $metrics.cached_input_tokens $expected.cached_input_tokens "$($scenario.scenario_id)/$armName cached tokens drifted"
        Assert-Equal $metrics.uncached_input_tokens $expected.uncached_input_tokens "$($scenario.scenario_id)/$armName uncached tokens drifted"
        Assert-Equal $metrics.output_tokens $expected.output_tokens "$($scenario.scenario_id)/$armName output tokens drifted"
        Assert-Equal $storedSummary.section_cost_summary.availability "measured" "$($scenario.scenario_id)/$armName section cost unavailable"
        Assert-Equal $storedSummary.request_2_plus_hit_rate $expected.request_2_plus_cache_hit_rate "$($scenario.scenario_id)/$armName cache rate drifted"
        Assert-Equal $storedSummary.section_cost_summary.active_projection_identity_summary.bootstrap_count $expected.active_projection_identity.bootstrap_count "$($scenario.scenario_id)/$armName bootstrap identity drifted"
        Assert-Equal $storedSummary.section_cost_summary.active_projection_identity_summary.active_count $expected.active_projection_identity.active_count "$($scenario.scenario_id)/$armName active identity drifted"
        Assert-Equal $storedSummary.section_cost_summary.active_projection_identity_summary.unavailable_count $expected.active_projection_identity.unavailable_count "$($scenario.scenario_id)/$armName unavailable identity drifted"
        Assert-Equal $projection.projection_count $expected.projection_count "$($scenario.scenario_id)/$armName projection count drifted"
        Assert-Equal $graph.node_count $expected.map_nodes "$($scenario.scenario_id)/$armName map node count drifted"
        Assert-Equal $graph.edge_count $expected.map_edges "$($scenario.scenario_id)/$armName map edge count drifted"
        if ($armName -eq "r6") {
            Assert-Equal $graph.all_nodes_on_root_finish_path $true "$($scenario.scenario_id)/R6 graph is not rooted"
            Assert-Equal $graph.cycle_detected $false "$($scenario.scenario_id)/R6 graph has a cycle"
            Assert-Equal $graph.legacy.OpenLeafNodeCount $expected.open_leaf_nodes "$($scenario.scenario_id)/R6 open leaves drifted"
        }
        $verifiedArmCount++
    }
}

Assert-Equal $verifiedArmCount 4 "Expected four frozen baseline arms"
Assert-Equal $result.offline_recompute_contract.result "passed_all_four_arms" "Offline recompute result is not frozen as passed"
Write-Host "R7 Phase A frozen baseline passed."
Write-Host "Scenarios: $(@($result.scenarios).Count)"
Write-Host "Offline wire summaries recomputed: $verifiedArmCount"
