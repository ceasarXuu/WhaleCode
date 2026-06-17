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

function Add-ReleaseLine {
    param($Lines, [string]$Line)
    [void]$Lines.Add($Line)
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
$metrics = @(Get-ChildItem -LiteralPath $runRoot -Filter "metrics.json" -Recurse -ErrorAction SilentlyContinue | ForEach-Object { Read-ReleaseJson $_.FullName } | Where-Object { $_ })

$taskspaceMetrics = @($metrics | Where-Object { Get-ReleaseString $_ "logical_mode" -eq "taskspace" })
$maxLargeReplay = 0
$runtimeOutputRefs = 0
foreach ($metric in $taskspaceMetrics) {
    $maxLargeReplay = [Math]::Max($maxLargeReplay, (Get-ReleaseInt $metric "large_output_replay_count" 0))
    $runtimeOutputRefs += Get-ReleaseInt $metric "runtime_output_ref_created_count" 0
}

$evidence = [ordered]@{
    cost_gate_path = $costPath
    aggregate_path = $aggregatePath
    projection_summary_path = $projectionPath
    map_summary_path = $mapPath
    routing_summary_path = $routingPath
    cost_diagnostics_path = $costDiagnosticsPath
    required_artifacts = @($requiredArtifacts)
}
$qualityPass = ($aggregate -and (Get-ReleaseBool $aggregate "score_valid") -and (Get-ReleaseString $aggregate "run_validity") -eq "valid")
$costStatus = Get-ReleaseString $cost "status" "MISSING"
$projectionPass = ($projection -and (Get-ReleaseInt $projection "missing_taskspace_projection_count" 0) -eq 0 -and (Get-ReleaseInt $projection "taskspace_projection_protected_miss_count" 0) -eq 0)
$mapPass = ($map -and (Get-ReleaseString $map "availability") -eq "measured" -and (Get-ReleaseInt $map "protected_miss_count" 0) -eq 0)
$routingPass = ($routing -and (Get-ReleaseString $routing "availability") -eq "measured" -and (Get-ReleaseInt $routing "routing_mistake_count" 0) -eq 0)
$outputRefPass = ($maxLargeReplay -eq 0)

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
if ($aggregate -and (Get-ReleaseInt $aggregate "excluded_pairs" 0) -gt 0) { Add-ReleaseLine $blockers "excluded_pairs_present" }

$decision = "FAIL"
if ($qualityPass -and $projectionPass -and $mapPass -and $routingPass -and $outputRefPass) {
    if ($costStatus -eq "PASS" -and $blockers.Count -eq 0) {
        $decision = "PASS"
    } elseif ($costStatus -eq "PARTIAL" -and -not $blockers.Contains("excluded_pairs_present")) {
        $decision = "PARTIAL"
    }
}

$summary = [pscustomobject]@{
    schema_version = "taskspace-v005-release-decision-v1"
    decision = $decision
    run_dir = $runRoot
    cost_status = $costStatus
    quality_gate_pass = [bool]$qualityPass
    projection_gate_pass = [bool]$projectionPass
    map_gate_pass = [bool]$mapPass
    routing_gate_pass = [bool]$routingPass
    output_ref_gate_pass = [bool]$outputRefPass
    max_large_output_replay_count = [int]$maxLargeReplay
    runtime_output_ref_created_count = [int]$runtimeOutputRefs
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
Add-ReleaseLine $lines "- run_dir: $runRoot"
Add-ReleaseLine $lines "- cost_status: $costStatus"
Add-ReleaseLine $lines "- quality_gate_pass: $qualityPass"
Add-ReleaseLine $lines "- projection_gate_pass: $projectionPass"
Add-ReleaseLine $lines "- map_gate_pass: $mapPass"
Add-ReleaseLine $lines "- routing_gate_pass: $routingPass"
Add-ReleaseLine $lines "- output_ref_gate_pass: $outputRefPass"
Add-ReleaseLine $lines "- max_large_output_replay_count: $maxLargeReplay"
Add-ReleaseLine $lines "- runtime_output_ref_created_count: $runtimeOutputRefs"
Add-ReleaseLine $lines "- blockers: $(if ($blockers.Count -eq 0) { 'none' } else { @($blockers.ToArray()) -join ', ' })"
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
if ($decision -eq "PASS") { exit 0 }
if ($decision -eq "PARTIAL") { exit 2 }
exit 1
