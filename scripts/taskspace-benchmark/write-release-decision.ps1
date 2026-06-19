param(
    [Parameter(Mandatory = $true)][string]$RunDir,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

function Read-ReleaseJson {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    try { Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json } catch { $null }
}

function Get-ReleaseBool {
    param($Object, [string]$Name, [bool]$DefaultValue = $false)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        if ($Object.$Name -is [bool]) { return [bool]$Object.$Name }
        if ($Object.$Name -is [string]) {
            $value = [string]$Object.$Name
            if ($value -eq "true") { return $true }
            if ($value -eq "false") { return $false }
            return $DefaultValue
        }
        return [bool]$Object.$Name
    }
    $DefaultValue
}

function Get-ReleaseInt {
    param($Object, [string]$Name, [int]$DefaultValue = 0)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        return [int]$Object.$Name
    }
    $DefaultValue
}

function Get-ReleaseString {
    param($Object, [string]$Name, [string]$DefaultValue = "")
    if ($Object -and $Object.PSObject.Properties.Name -contains $Name -and $null -ne $Object.$Name) {
        return [string]$Object.$Name
    }
    $DefaultValue
}

function Read-ReleaseJsonl {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return @() }
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { [void]$rows.Add(($line | ConvertFrom-Json)) } catch {}
    }
    @($rows.ToArray())
}

function Add-ReleaseLine {
    param($Lines, [string]$Line)
    [void]$Lines.Add($Line)
}

function Test-ReleasePathUnderRoot {
    param([string]$Root, [string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $false }
    try {
        $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
        $pathFull = [System.IO.Path]::GetFullPath($Path)
        return $pathFull.StartsWith($rootFull + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
    } catch {
        return $false
    }
}

function Read-ReleaseMetric {
    param([Parameter(Mandatory = $true)][string]$Path)
    $metric = Read-ReleaseJson $Path
    if (-not $metric) { return $null }
    [pscustomobject]@{
        path = $Path
        metric = $metric
        logical_mode = Get-ReleaseString $metric "logical_mode"
    }
}

function Get-ReleaseValidOutputRefCreatedEvents {
    param([Parameter(Mandatory = $true)][string]$Path)
    @(Read-ReleaseJsonl $Path | Where-Object {
            [string]$_.schema_version -eq "taskspace-output-ref-event-v1" -and
            [string]$_.kind -eq "output_ref.created" -and
            [string]$_.source -in @("observability_timeline", "observability_text_fallback", "taskspace.graph.final.json", "taskspace.graph.timeout.json", "graph-health.json") -and
            [string]$_.artifact_ref -match '^output-ref://sha256/[a-fA-F0-9]{64}$'
        })
}

$runRoot = (Resolve-Path -LiteralPath $RunDir).Path
if (-not $OutputPath) { $OutputPath = Join-Path $runRoot "release-decision.md" }
$jsonPath = [System.IO.Path]::ChangeExtension($OutputPath, ".json")

$costPath = Join-Path $runRoot "suite-cost-gate.json"
$aggregatePath = Join-Path $runRoot "aggregate.json"
$projectionPath = Join-Path $runRoot "context-projection-summary.json"
$mapPath = Join-Path $runRoot "suite-map-management-summary.json"
$routingPath = Join-Path $runRoot "suite-routing-summary.json"
$costDiagnosticsPath = Join-Path $runRoot "cost-diagnostics.json"
$runStatusPath = Join-Path $runRoot "run-status.json"
$eventsPath = Join-Path $runRoot "events.jsonl"
$outputRefEventsPath = Join-Path $runRoot "output-ref-events.jsonl"
$requiredArtifacts = @(
    "token-summary.json",
    "request-summary.json",
    "taskspace-control-usage.json",
    "projection-events.jsonl",
    "output-ref-events.jsonl",
    "compaction-events.jsonl",
    "routing-decision.json",
    "suite-cost-gate.json",
    "suite-map-management-summary.json"
)
$cost = Read-ReleaseJson $costPath
$aggregate = Read-ReleaseJson $aggregatePath
$projection = Read-ReleaseJson $projectionPath
$map = Read-ReleaseJson $mapPath
$routing = Read-ReleaseJson $routingPath
$costDiagnostics = Read-ReleaseJson $costDiagnosticsPath
$runStatus = Read-ReleaseJson $runStatusPath
$runEvents = @(Read-ReleaseJsonl $eventsPath)
$outputRefEvents = @(Read-ReleaseJsonl $outputRefEventsPath)
$pairMetricPattern = '(?i)[\\/](pair-\d+)[\\/](left|right)[\\/]artifacts[\\/]metrics\.json$'
$metricRecords = @(Get-ChildItem -LiteralPath $runRoot -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq "metrics.json" -and $_.FullName -match $pairMetricPattern } |
    ForEach-Object { Read-ReleaseMetric $_.FullName } |
    Where-Object { $_ })
$standardMetrics = @($metricRecords | Where-Object { [string]$_.logical_mode -eq "standard" })
$taskspaceMetrics = @($metricRecords | Where-Object { [string]$_.logical_mode -eq "taskspace" })
$maxLargeReplay = 0
$runtimeOutputRefs = 0
foreach ($metric in $taskspaceMetrics) {
    $maxLargeReplay = [Math]::Max($maxLargeReplay, (Get-ReleaseInt $metric.metric "large_output_replay_count" 0))
    $runtimeOutputRefs += Get-ReleaseInt $metric.metric "runtime_output_ref_created_count" 0
}
$validOutputRefCreatedEvents = @(Get-ReleaseValidOutputRefCreatedEvents $outputRefEventsPath)
$runInitializedEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "run_initialized" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.scenario_id) -and
        [int]$_.repeats -gt 0 -and
        [string]$_.evidence_target -eq "E3"
    })
$routingDecisionEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "routing_decision_completed" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.mode) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.status) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.path) -and
        (Test-ReleasePathUnderRoot $runRoot ([string]$_.path)) -and
        (Test-Path -LiteralPath ([string]$_.path) -PathType Leaf)
    })
$latestRoutingDecisionEvent = @($runEvents | Where-Object { [string]$_.event -eq "routing_decision_completed" } | Select-Object -Last 1)
$routingDecisionPathExpected = Join-Path $runRoot "routing-decision.json"
$routingDecisionPass = ($latestRoutingDecisionEvent.Count -eq 1 `
    -and [int]$latestRoutingDecisionEvent[0].schema_version -eq 1 `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].timestamp) `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].mode) `
    -and -not [string]::IsNullOrWhiteSpace([string]$latestRoutingDecisionEvent[0].status) `
    -and ([System.IO.Path]::GetFullPath([string]$latestRoutingDecisionEvent[0].path).Equals([System.IO.Path]::GetFullPath($routingDecisionPathExpected), [System.StringComparison]::OrdinalIgnoreCase)) `
    -and (Test-Path -LiteralPath $routingDecisionPathExpected -PathType Leaf))
$pairCompletedEvents = @($runEvents | Where-Object {
        [string]$_.event -eq "pair_completed" -and
        [int]$_.schema_version -eq 1 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.timestamp) -and
        [int]$_.repeat -gt 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$_.pair_report) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.reported_evidence_level)
    })
$completedPairs = Get-ReleaseInt $runStatus "completed_pairs" 0
$validPairEvidence = New-Object System.Collections.Generic.List[object]
$pairEvidenceKeys = New-Object 'System.Collections.Generic.HashSet[string]'
$pairRepeatKeys = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($event in $pairCompletedEvents) {
    $pairReport = [string]$event.pair_report
    if (-not (Test-ReleasePathUnderRoot $runRoot $pairReport)) { continue }
    if ([string]$event.reported_evidence_level -ne "E3") { continue }
    $pairDir = Split-Path -Parent $pairReport
    if (-not (Test-Path -LiteralPath $pairReport -PathType Leaf)) { continue }
    $pairLeaf = Split-Path -Leaf $pairDir
    if ($pairLeaf -notmatch '^pair-\d{3}$') { continue }
    $expectedPairLeaf = "pair-{0:D3}" -f [int]$event.repeat
    if ($pairLeaf -ne $expectedPairLeaf) { continue }
    $pairParent = Split-Path -Parent $pairDir
    if (-not ([System.IO.Path]::GetFullPath($pairParent).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar).Equals($runRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar), [System.StringComparison]::OrdinalIgnoreCase))) { continue }
    $pairKey = [System.IO.Path]::GetFullPath($pairDir).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar).ToLowerInvariant()
    $repeatKey = [string][int]$event.repeat
    if (-not $pairEvidenceKeys.Add($pairKey)) { continue }
    if (-not $pairRepeatKeys.Add($repeatKey)) { continue }
    $leftMetricPath = Join-Path $pairDir "left\artifacts\metrics.json"
    $rightMetricPath = Join-Path $pairDir "right\artifacts\metrics.json"
    if (-not (Test-Path -LiteralPath $leftMetricPath) -or -not (Test-Path -LiteralPath $rightMetricPath)) { continue }
    $leftRecord = Read-ReleaseMetric $leftMetricPath
    $rightRecord = Read-ReleaseMetric $rightMetricPath
    if (-not $leftRecord -or -not $rightRecord) { continue }
    $modes = @([string]$leftRecord.logical_mode, [string]$rightRecord.logical_mode)
    if (-not ($modes -contains "standard") -or -not ($modes -contains "taskspace")) { continue }
    $taskspaceRecord = if ([string]$leftRecord.logical_mode -eq "taskspace") { $leftRecord } else { $rightRecord }
    $taskspaceOutputRefEventsPath = Join-Path (Split-Path -Parent $taskspaceRecord.path) "output-ref-events.jsonl"
    $taskspaceOutputRefEvents = @(Get-ReleaseValidOutputRefCreatedEvents $taskspaceOutputRefEventsPath)
    $taskspaceRuntimeOutputRefs = Get-ReleaseInt $taskspaceRecord.metric "runtime_output_ref_created_count" 0
    [void]$validPairEvidence.Add([pscustomobject]@{
            pair_dir = $pairDir
            pair_report = $pairReport
            repeat = [int]$event.repeat
            taskspace_output_ref_events_path = $taskspaceOutputRefEventsPath
            taskspace_runtime_output_ref_created_count = [int]$taskspaceRuntimeOutputRefs
            taskspace_output_ref_created_event_count = [int]$taskspaceOutputRefEvents.Count
            output_ref_correlated = ($taskspaceRuntimeOutputRefs -gt 0 -and $taskspaceOutputRefEvents.Count -gt 0)
        })
}
$validOutputRefPairEvidence = @($validPairEvidence | Where-Object { [bool]$_.output_ref_correlated })
$runProvenancePass = ($runStatus `
    -and (Get-ReleaseInt $runStatus "schema_version" 0) -eq 1 `
    -and (Get-ReleaseString $runStatus "run_validity") -eq "valid" `
    -and (Get-ReleaseString $runStatus "evidence_target") -eq "E3" `
    -and (Get-ReleaseBool $runStatus "diagnostic_comparison_enabled") `
    -and (Get-ReleaseBool $runStatus "final_aggregate_ready") `
    -and $completedPairs -gt 0 `
    -and $runInitializedEvents.Count -gt 0 `
    -and $routingDecisionEvents.Count -gt 0 `
    -and $routingDecisionPass `
    -and $pairCompletedEvents.Count -eq $completedPairs `
    -and $validPairEvidence.Count -eq $completedPairs `
    -and $pairEvidenceKeys.Count -eq $completedPairs `
    -and $pairRepeatKeys.Count -eq $completedPairs)

$evidence = [ordered]@{
    cost_gate_path = $costPath
    aggregate_path = $aggregatePath
    projection_summary_path = $projectionPath
    map_summary_path = $mapPath
    routing_summary_path = $routingPath
    cost_diagnostics_path = $costDiagnosticsPath
    run_status_path = $runStatusPath
    events_path = $eventsPath
    output_ref_events_path = $outputRefEventsPath
    required_artifacts = @($requiredArtifacts)
}
$qualityPass = ($aggregate -and (Get-ReleaseBool $aggregate "score_valid") -and (Get-ReleaseString $aggregate "run_validity") -eq "valid")
$costStatus = Get-ReleaseString $cost "status" "MISSING"
$projectionPass = ($projection `
    -and (Get-ReleaseInt $projection "missing_taskspace_projection_count" 0) -eq 0 `
    -and (Get-ReleaseInt $projection "taskspace_projection_protected_miss_count" 0) -eq 0 `
    -and (Get-ReleaseInt $projection "active_projection_count" 0) -gt 0 `
    -and (Get-ReleaseInt $projection "shadow_projection_count" 0) -eq 0)
$mapPass = ($map -and (Get-ReleaseString $map "availability") -eq "measured" -and (Get-ReleaseInt $map "protected_miss_count" 0) -eq 0)
$routingPass = ($routing -and (Get-ReleaseString $routing "availability") -eq "measured" -and (Get-ReleaseInt $routing "routing_mistake_count" 0) -eq 0)
$outputRefPass = ($maxLargeReplay -eq 0 -and $runtimeOutputRefs -gt 0 -and $validOutputRefCreatedEvents.Count -gt 0 -and $validOutputRefPairEvidence.Count -eq $completedPairs)

$blockers = New-Object System.Collections.Generic.List[string]
foreach ($artifactName in $requiredArtifacts) {
    if (-not (Test-Path -LiteralPath (Join-Path $runRoot $artifactName))) {
        Add-ReleaseLine $blockers "required_artifact_missing:$artifactName"
    }
}
if (-not $qualityPass) { Add-ReleaseLine $blockers "quality_gate_failed" }
if ($costStatus -eq "MISSING") { Add-ReleaseLine $blockers "cost_gate_missing" }
elseif ($costStatus -eq "FAIL") { Add-ReleaseLine $blockers "cost_gate_failed" }
if (-not $projectionPass) { Add-ReleaseLine $blockers "projection_gate_failed" }
if (-not $mapPass) { Add-ReleaseLine $blockers "map_gate_failed" }
if (-not $routingPass) { Add-ReleaseLine $blockers "routing_gate_failed" }
if (-not $outputRefPass) { Add-ReleaseLine $blockers "output_ref_replay_failed" }
if (-not $runProvenancePass) { Add-ReleaseLine $blockers "run_provenance_gate_failed" }
if ($aggregate -and (Get-ReleaseInt $aggregate "excluded_pairs" 0) -gt 0) { Add-ReleaseLine $blockers "excluded_pairs_present" }

$decision = "fail"
$closeable = $false
if ($qualityPass -and $projectionPass -and $mapPass -and $routingPass -and $outputRefPass -and $runProvenancePass) {
    if ($costStatus -eq "PASS" -and $blockers.Count -eq 0) {
        $decision = "release_pass"
        $closeable = $true
    } elseif ($costStatus -eq "PARTIAL" -and $blockers.Count -eq 0) {
        $decision = "blocked_partial"
    }
}

$summary = [pscustomobject]@{
    schema_version = "taskspace-v005-release-decision-v1"
    decision = $decision
    closeable = [bool]$closeable
    run_dir = $runRoot
    cost_status = $costStatus
    quality_gate_pass = [bool]$qualityPass
    projection_gate_pass = [bool]$projectionPass
    map_gate_pass = [bool]$mapPass
    routing_gate_pass = [bool]$routingPass
    output_ref_gate_pass = [bool]$outputRefPass
    run_provenance_gate_pass = [bool]$runProvenancePass
    max_large_output_replay_count = [int]$maxLargeReplay
    runtime_output_ref_created_count = [int]$runtimeOutputRefs
    valid_output_ref_created_event_count = [int]$validOutputRefCreatedEvents.Count
    standard_metric_count = [int]$standardMetrics.Count
    taskspace_metric_count = [int]$taskspaceMetrics.Count
    completed_pair_count = [int]$completedPairs
    completed_pair_event_count = [int]$pairCompletedEvents.Count
    valid_pair_evidence_count = [int]$validPairEvidence.Count
    output_ref_correlated_pair_count = [int]$validOutputRefPairEvidence.Count
    accepted_pair_dirs = @($validPairEvidence | ForEach-Object { [string]$_.pair_dir })
    accepted_pair_repeats = @($validPairEvidence | ForEach-Object { [int]$_.repeat })
    routing_decision_event_count = [int]$routingDecisionEvents.Count
    latest_routing_decision_path = if ($latestRoutingDecisionEvent.Count -eq 1) { [string]$latestRoutingDecisionEvent[0].path } else { "" }
    latest_routing_decision_pass = [bool]$routingDecisionPass
    cost_root_cause = Get-ReleaseString $costDiagnostics "root_cause" ""
    cost_drivers = if ($costDiagnostics -and $costDiagnostics.PSObject.Properties.Name -contains "drivers") { @($costDiagnostics.drivers) } else { @() }
    blockers = @($blockers.ToArray())
    evidence = [pscustomobject]$evidence
}
$summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$lines = New-Object System.Collections.Generic.List[string]
Add-ReleaseLine $lines "# TaskSpace v0.0.5 Release Decision"
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "- decision: $decision"
Add-ReleaseLine $lines "- closeable: $closeable"
Add-ReleaseLine $lines "- run_dir: $runRoot"
Add-ReleaseLine $lines "- cost_status: $costStatus"
Add-ReleaseLine $lines "- quality_gate_pass: $qualityPass"
Add-ReleaseLine $lines "- projection_gate_pass: $projectionPass"
Add-ReleaseLine $lines "- map_gate_pass: $mapPass"
Add-ReleaseLine $lines "- routing_gate_pass: $routingPass"
Add-ReleaseLine $lines "- output_ref_gate_pass: $outputRefPass"
Add-ReleaseLine $lines "- run_provenance_gate_pass: $runProvenancePass"
Add-ReleaseLine $lines "- max_large_output_replay_count: $maxLargeReplay"
Add-ReleaseLine $lines "- runtime_output_ref_created_count: $runtimeOutputRefs"
Add-ReleaseLine $lines "- valid_output_ref_created_event_count: $($validOutputRefCreatedEvents.Count)"
Add-ReleaseLine $lines "- standard_metric_count: $($standardMetrics.Count)"
Add-ReleaseLine $lines "- taskspace_metric_count: $($taskspaceMetrics.Count)"
Add-ReleaseLine $lines "- completed_pair_count: $completedPairs"
Add-ReleaseLine $lines "- completed_pair_event_count: $($pairCompletedEvents.Count)"
Add-ReleaseLine $lines "- valid_pair_evidence_count: $($validPairEvidence.Count)"
Add-ReleaseLine $lines "- output_ref_correlated_pair_count: $($validOutputRefPairEvidence.Count)"
Add-ReleaseLine $lines "- routing_decision_event_count: $($routingDecisionEvents.Count)"
Add-ReleaseLine $lines "- latest_routing_decision_pass: $routingDecisionPass"
Add-ReleaseLine $lines "- latest_routing_decision_path: $(if ($latestRoutingDecisionEvent.Count -eq 1) { [string]$latestRoutingDecisionEvent[0].path } else { '' })"
Add-ReleaseLine $lines "- blockers: $(if ($blockers.Count -eq 0) { 'none' } else { @($blockers.ToArray()) -join ', ' })"
if ($decision -eq "blocked_partial") {
    Add-ReleaseLine $lines "- release_note: blocked_partial is engineering progress only and cannot close v0.0.5."
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Evidence Paths"
foreach ($entry in $evidence.GetEnumerator()) {
    Add-ReleaseLine $lines "- $($entry.Key): $($entry.Value)"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Cost"
if ($cost) {
    Add-ReleaseLine $lines "- direct_input_output_ratio: $($cost.ratios.direct_input_output_ratio)"
    Add-ReleaseLine $lines "- walltime_ratio: $($cost.ratios.walltime_ratio)"
    Add-ReleaseLine $lines "- model_request_count_ratio: $($cost.ratios.model_request_count_ratio)"
    if ($costDiagnostics) {
        Add-ReleaseLine $lines "- root_cause: $($costDiagnostics.root_cause)"
        Add-ReleaseLine $lines "- drivers: $(@($costDiagnostics.drivers) -join ', ')"
        Add-ReleaseLine $lines "- rollout_trace_model_request_count_ratio: $($costDiagnostics.ratios.rollout_trace_model_request_count_ratio)"
        Add-ReleaseLine $lines "- uncached_input_ratio: $($costDiagnostics.ratios.uncached_input_ratio)"
        Add-ReleaseLine $lines "- projection_token_share_of_taskspace_input: $($costDiagnostics.ratios.projection_token_share_of_taskspace_input)"
    }
} else {
    Add-ReleaseLine $lines "- unavailable"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Quality"
if ($aggregate) {
    Add-ReleaseLine $lines "- score_valid: $($aggregate.score_valid)"
    Add-ReleaseLine $lines "- run_validity: $($aggregate.run_validity)"
    Add-ReleaseLine $lines "- both_success: $($aggregate.both_success)"
    Add-ReleaseLine $lines "- both_failed: $($aggregate.both_failed)"
    Add-ReleaseLine $lines "- excluded_pairs: $($aggregate.excluded_pairs)"
    Add-ReleaseLine $lines "- excluded_by_reason: $($aggregate.excluded_by_reason | ConvertTo-Json -Compress)"
} else {
    Add-ReleaseLine $lines "- unavailable"
}
Add-ReleaseLine $lines ""
Add-ReleaseLine $lines "## Routing"
if ($routing) {
    Add-ReleaseLine $lines "- recommended_mode: $($routing.recommended_mode)"
    Add-ReleaseLine $lines "- router_status: $($routing.router_status)"
    Add-ReleaseLine $lines "- routing_mistake_count: $($routing.routing_mistake_count)"
    Add-ReleaseLine $lines "- verification_first_expected_format_count: $($routing.verification_first_expected_format_count)"
} else {
    Add-ReleaseLine $lines "- unavailable"
}
$lines | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Host "ReleaseDecision: $OutputPath"
Write-Host "ReleaseDecisionJson: $jsonPath"
if ($decision -eq "release_pass") { exit 0 }
if ($decision -eq "blocked_partial") { exit 2 }
exit 1
