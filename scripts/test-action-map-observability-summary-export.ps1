param(
    [string]$RunRoot = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "test-action-map-store-fixture-lib.ps1")

if (-not $RunRoot) {
    $RunRoot = Join-Path $PSScriptRoot "..\target\action-map-observability-summary-export-test"
}
[void](New-Item -ItemType Directory -Force -Path $RunRoot)

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

$fixtureRoot = Join-Path $RunRoot "fixture"
$exportDir = Join-Path $RunRoot "export"
[void](New-Item -ItemType Directory -Force -Path $fixtureRoot)
[void](New-Item -ItemType Directory -Force -Path $exportDir)
$testWhale = New-TestActionMapStoreWhale (Join-Path $RunRoot "fake-map-store")
$rolloutPath = Join-Path $fixtureRoot "rollout.jsonl"
$jsonlPath = Join-Path $fixtureRoot "whale-exec.jsonl"

$largeMarker = "middle-secret-marker-" * 500
$lines = New-Object System.Collections.Generic.List[string]
for ($i = 1; $i -le 135; $i++) {
    $snapshot = [ordered]@{
        timestamp = "2026-06-29T00:00:$($i % 60).000Z"
        payload = [ordered]@{
            type = "map_runtime"
            map_event_type = "snapshot_updated"
            snapshot = [ordered]@{
                tasks = @([ordered]@{ id = "task-1"; title = "Large export"; objective = "Bound observability"; status = "active"; ownerSessionId = "thread-1"; activeMapId = "map-1"; mapIds = @("map-1") })
                maps = @([ordered]@{
                    id = "map-1"
                    taskId = "task-1"
                    title = "Large export"
                    ownerSessionId = "thread-1"
                    createdFrom = $null
                    edges = @([ordered]@{ from = "node-1"; to = "node-2" })
                    nodes = @(
                        [ordered]@{ id = "node-1"; title = "Summarize"; kind = "inspect_code_context"; status = "completed" },
                        [ordered]@{ id = "node-2"; title = "Patch"; kind = "implement_solution"; status = "ready" }
                    )
                    subagentPlans = @()
                    results = @([ordered]@{
                        id = "result-$i"
                        nodeId = "node-1"
                        assignmentId = "lease-1"
                        sourceThreadId = "thread-1"
                        kind = "result"
                        actionClass = "inspect"
                        body = $largeMarker
                        evidencePackage = [ordered]@{ validity = "accepted"; claims = @(); evidenceRefs = @(); validatorRefs = @(); changedArtifacts = @(); remainingUncertainty = @(); validityReason = "fixture" }
                    })
                })
                maintenanceBarriers = @()
                sentinelWarnings = @()
            }
        }
    }
    $lines.Add(($snapshot | ConvertTo-Json -Compress -Depth 30))
}
$lines.Add(([ordered]@{
    timestamp = "2026-06-29T00:01:00.000Z"
    payload = [ordered]@{ type = "map_runtime"; map_event_type = "taskspace_trace_event_recorded"; kind = "output_ref.created"; traceEventId = "trace-output-ref"; artifactRef = "output-ref://sha256/" + ("a" * 64) }
} | ConvertTo-Json -Compress -Depth 10))
$lines.Add(([ordered]@{
    timestamp = "2026-06-29T00:01:01.000Z"
    payload = [ordered]@{ type = "cognitive_state_updated"; updateKind = "state_commit.final"; recordId = "commit-1"; taskId = "task-1"; mapId = "map-1" }
} | ConvertTo-Json -Compress -Depth 10))
$lines.Add(([ordered]@{
    timestamp = "2026-06-29T00:01:02.000Z"
    payload = [ordered]@{ type = "map_runtime"; map_event_type = "snapshot_updated"; oversized = ("oversized-line-marker-" * 5000) }
} | ConvertTo-Json -Compress -Depth 10))
$lines | Set-Content -LiteralPath $rolloutPath -Encoding UTF8
"" | Set-Content -LiteralPath $jsonlPath -Encoding UTF8
Set-TestActionMapStoreFixture -WhalePath $testWhale -ThreadId "thread-1" -Snapshot $snapshot.payload.snapshot

try {
    $env:TASKSPACE_OBSERVABILITY_ROLLOUT_MAX_BYTES = "1048576"
    $env:TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES = "65536"
    $env:TASKSPACE_OBSERVABILITY_TIMELINE_SAMPLE_LIMIT = "60"
    & (Join-Path $PSScriptRoot "export-action-map-observability.ps1") -RolloutPath $rolloutPath -JsonlPath $jsonlPath -OutputDir $exportDir -WhalePath $testWhale -ThreadId "thread-1" -ArtifactRoot $fixtureRoot | Out-Null
} finally {
    Remove-Item Env:\TASKSPACE_OBSERVABILITY_ROLLOUT_MAX_BYTES -ErrorAction SilentlyContinue
    Remove-Item Env:\TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES -ErrorAction SilentlyContinue
    Remove-Item Env:\TASKSPACE_OBSERVABILITY_TIMELINE_SAMPLE_LIMIT -ErrorAction SilentlyContinue
}

$jsonPath = Join-Path $exportDir "action-map-observability.json"
$htmlPath = Join-Path $exportDir "action-map-observability.html"
$policyPath = Join-Path $exportDir "action-map-observability-policy.json"
Assert-True (Test-Path -LiteralPath $jsonPath) "summary export JSON was not written"
Assert-True (Test-Path -LiteralPath $htmlPath) "summary export HTML was not written"
Assert-True (Test-Path -LiteralPath $policyPath) "summary export policy was not written"

$obs = Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonPath | ConvertFrom-Json
$jsonText = Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonPath
$htmlText = Get-Content -Raw -Encoding UTF8 -LiteralPath $htmlPath
Assert-Equal ([string]$obs.source.exportPolicy.rollout_export_mode) "summary_only_large_rollout" "large rollout should use summary-only mode"
Assert-True ([int64]$obs.source.exportPolicy.rollout_bytes -gt 1048576) "fixture rollout should exceed summary threshold"
Assert-True ([int]$obs.summary.timelineEventsDropped -gt 0) "summary export should bound timeline size"
Assert-Equal ([int]$obs.summary.runtimeEventCounts.'output_ref.created') 1 "summary event counts should preserve output ref creation"
Assert-Equal ([int]$obs.summary.runtimeEventCounts.'state_commit.final') 1 "summary event counts should preserve state commit update kind"
Assert-Equal ([int]$obs.summary.edges) 1 "summary export should deduplicate repeated snapshot edges"
Assert-Equal (@($obs.edges).Count) 1 "summary edge table should contain one logical edge"
Assert-True ([int]$obs.source.rolloutReadStats.largeLineSkippedCount -ge 1) "summary export should skip oversized event payload materialization"
Assert-True ((Get-Item -LiteralPath $jsonPath).Length -lt 1048576) "summary JSON should stay under 1MiB"
Assert-True ((Get-Item -LiteralPath $htmlPath).Length -lt 1048576) "summary HTML should stay under 1MiB"
Assert-True ($jsonText -notmatch "middle-secret-marker") "summary JSON leaked raw large result body"
Assert-True ($htmlText -notmatch "middle-secret-marker") "summary HTML leaked raw large result body"

Write-Host "Action-map observability summary export self-test: PASS"
Write-Host "RunRoot: $RunRoot"
