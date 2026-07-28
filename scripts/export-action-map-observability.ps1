param(
    [Parameter(Mandatory = $true)][string]$RolloutPath,
    [Parameter(Mandatory = $true)][string]$JsonlPath,
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [Parameter(Mandatory = $true)][string]$WhalePath,
    [Parameter(Mandatory = $true)][string]$ThreadId,
    [string]$ArtifactRoot = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-observability-report-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-observability-summary-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-jsonl-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-store-export-lib.ps1")

$output = New-Item -ItemType Directory -Force -Path $OutputDir
$rolloutReadStats = New-JsonLineReadStats $RolloutPath
$jsonlReadStats = New-JsonLineReadStats $JsonlPath
$exportPolicy = Get-ActionMapObservabilityExportPolicy $RolloutPath
$exportPolicyPath = Join-Path $output.FullName "action-map-observability-policy.json"
($exportPolicy | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $exportPolicyPath -Encoding UTF8

$mapStore = Invoke-ActionMapStoreExport -WhalePath $WhalePath -ThreadId $ThreadId -OutputDir $output.FullName
$mapStoreSource = $mapStore | Select-Object * -ExcludeProperty canonical_map
$scan = New-ActionMapObservabilityEventScan -RolloutPath $RolloutPath -JsonlPath $JsonlPath -Policy $exportPolicy -RolloutReadStats $rolloutReadStats -JsonlReadStats $jsonlReadStats
$finalState = if ([string]$mapStore.availability -eq "measured" -and $mapStore.canonical_map) {
    ConvertFrom-TaskSpaceCanonicalMap $mapStore.canonical_map $mapStore.owner_thread_id
}
else {
    [pscustomobject]@{ tasks = @(); maps = @(); nodes = @(); edges = @(); sentinelWarnings = @(); agents = @() }
}

$isLarge = [string]$exportPolicy.rollout_export_mode -eq "summary_only_large_rollout"
$cognitiveAudit = if ($isLarge -or [string]$mapStore.availability -ne "measured") {
    New-ActionMapSummaryCognitiveAudit
}
else {
    Get-CognitiveAuditSummary $finalState.tasks $finalState.nodes $finalState.sentinelWarnings $scan.timeline $ArtifactRoot
}
$blockedToolActions = [int](Get-ActionMapStoreProperty $scan.runtimeEventCounts "tool_action_blocked" 0)
$activeMaintenanceBarriers = 0
foreach ($node in @($finalState.nodes)) {
    $activeMaintenanceBarriers += @(Get-ObjectArray $node.maintenanceBarriers | Where-Object { $_.state -eq "active" }).Count
}
$subagentPlanCount = 0
foreach ($map in @($finalState.maps)) {
    $subagentPlanCount += @(Get-ObjectArray $map.subagentPlans).Count
}
$summary = [ordered]@{
    tasks = @($finalState.tasks).Count
    maps = @($finalState.maps).Count
    subagentPlans = $subagentPlanCount
    nodes = @($finalState.nodes).Count
    edges = @($finalState.edges).Count
    agents = @($finalState.agents).Count
    toolCalls = @($scan.toolCalls).Count
    blockedToolActions = $blockedToolActions
    activeMaintenanceBarriers = $activeMaintenanceBarriers
    mapRuntimeEvents = [int]$scan.mapRuntimeEventCount
    runtimeEventCounts = $scan.runtimeEventCounts
    topLevelEventCounts = $scan.topLevelEventCounts
    largeLineEventCounts = $scan.largeLineEventCounts
    timelineEventsDropped = [int]$scan.timelineEventsDropped
    outputContracts = [int]$cognitiveAudit.metrics.outputContractCount
    factSources = [int]$cognitiveAudit.metrics.factSourceCount
    acceptedResults = [int]$cognitiveAudit.metrics.acceptedResultCount
    questionedOrInvalidResults = [int]$cognitiveAudit.metrics.questionedOrInvalidResultCount
    cognitiveStructuralGatePassed = [bool]$cognitiveAudit.structuralGatePassed
    cognitiveAuditHardGatePassed = [bool]$cognitiveAudit.hardGatePassed
    inputParseErrors = ([int]$rolloutReadStats.parseErrorCount + [int]$jsonlReadStats.parseErrorCount)
    finalArtifacts = [int]$cognitiveAudit.metrics.finalArtifactCount
}
$reduced = [ordered]@{
    generatedAt = (Get-Date).ToString("o")
    source = [ordered]@{
        rolloutPath = (Resolve-Path -LiteralPath $RolloutPath).Path
        jsonlPath = if (Test-Path -LiteralPath $JsonlPath) { (Resolve-Path -LiteralPath $JsonlPath).Path } else { $JsonlPath }
        rolloutReadStats = $rolloutReadStats
        jsonlReadStats = $jsonlReadStats
        artifactRoot = $ArtifactRoot
        exportPolicy = $exportPolicy
        exportPolicyPath = $exportPolicyPath
        mapStore = $mapStoreSource
    }
    summary = $summary
    tasks = @($finalState.tasks)
    maps = @($finalState.maps)
    nodes = @($finalState.nodes)
    edges = @($finalState.edges)
    sentinelWarnings = @($finalState.sentinelWarnings)
    cognitiveAudit = $cognitiveAudit
    finalArtifacts = @($cognitiveAudit.finalArtifacts)
    agents = @($finalState.agents)
    toolCalls = @($scan.toolCalls)
    timeline = @($scan.timeline)
}

$reportPaths = Write-ActionMapObservabilityReport -Reduced $reduced -OutputDir $output.FullName
Write-Host "MapStoreAvailability: $($mapStore.availability)"
Write-Host "MapStoreErrorCode: $($mapStore.error_code)"
Write-Host "ObservabilityJson: $($reportPaths.Json)"
Write-Host "ObservabilityMarkdown: $($reportPaths.Markdown)"
Write-Host "ObservabilityHtml: $($reportPaths.Html)"

if ([string]$mapStore.availability -eq "map_store_failed") {
    throw "Canonical TaskSpace Map Store export failed: $($mapStore.error_code)"
}
