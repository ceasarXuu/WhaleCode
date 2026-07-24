param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "test-action-map-store-fixture-lib.ps1")
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $PSScriptRoot "../target/r6-action-map-observability-test"
}
[void](New-Item -ItemType Directory -Force -Path $RunRoot)
$testWhale = New-TestActionMapStoreWhale (Join-Path $RunRoot "fake-map-store")

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) { throw "$Message expected=$Expected actual=$Actual" }
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$rolloutPath = Join-Path $RunRoot "rollout.jsonl"
$execPath = Join-Path $RunRoot "whale-exec.jsonl"
$exportDir = Join-Path $RunRoot "export"
$snapshot = [ordered]@{
    timestamp = "2026-07-15T00:00:00Z"
    payload = [ordered]@{
        type = "map_runtime"
        map_event_type = "snapshot_updated"
        snapshot = [ordered]@{
            schemaVersion = "TaskSpaceSnapshotR6V1"
            mode = "experiment"
            bootstrapRequired = $false
            map = [ordered]@{
                id = "map-1"
                taskId = "task-1"
                ownerSessionId = "thread-1"
                rootNodeId = "root"
                finishNodeId = "finish"
                revision = 6
                currentNodeId = $null
                terminalSummaryRef = "event:finish"
                complete = $true
                nodes = @(
                    [ordered]@{ id = "root"; role = "task_root"; goal = "Fix the defect"; status = "closed"; sourceRefs = @("task-event-1"); activeLease = $null; resultIds = @(); nodeEventIds = @() },
                    [ordered]@{ id = "inspect-code"; role = "work"; goal = "Inspect code"; status = "completed"; sourceRefs = @(); activeLease = $null; resultIds = @("result-1"); nodeEventIds = @("node-event-1") },
                    [ordered]@{ id = "inspect-tests"; role = "work"; goal = "Inspect tests"; status = "completed"; sourceRefs = @(); activeLease = $null; resultIds = @("result-2"); nodeEventIds = @() },
                    [ordered]@{ id = "patch"; role = "work"; goal = "Apply and verify patch"; status = "completed"; sourceRefs = @(); activeLease = $null; resultIds = @("result-3"); nodeEventIds = @() },
                    [ordered]@{ id = "finish"; role = "finish"; goal = "Validated result"; status = "closed"; sourceRefs = @(); activeLease = $null; resultIds = @(); nodeEventIds = @() }
                )
                edges = @(
                    [ordered]@{ from = "root"; to = "inspect-code" },
                    [ordered]@{ from = "root"; to = "inspect-tests" },
                    [ordered]@{ from = "inspect-code"; to = "patch" },
                    [ordered]@{ from = "inspect-tests"; to = "patch" },
                    [ordered]@{ from = "patch"; to = "finish" }
                )
                leases = @()
                results = @(
                    [ordered]@{ id = "result-1"; assignmentId = "lease-1"; mapId = "map-1"; nodeId = "inspect-code"; kind = "result"; sourceEventRef = "task-event-2"; artifactRefs = @(); sourceThreadId = "thread-1" },
                    [ordered]@{ id = "result-2"; assignmentId = "lease-2"; mapId = "map-1"; nodeId = "inspect-tests"; kind = "result"; sourceEventRef = "task-event-3"; artifactRefs = @(); sourceThreadId = "thread-1" },
                    [ordered]@{ id = "result-3"; assignmentId = "lease-3"; mapId = "map-1"; nodeId = "patch"; kind = "result"; sourceEventRef = "task-event-4"; artifactRefs = @("src/app.py"); sourceThreadId = "thread-1" }
                )
                nodeEvents = @()
            }
            maintenanceBarriers = @()
            sentinelWarnings = @()
        }
    }
}
($snapshot | ConvertTo-Json -Compress -Depth 30) | Set-Content -LiteralPath $rolloutPath -Encoding UTF8
"" | Set-Content -LiteralPath $execPath -Encoding UTF8
Set-TestActionMapStoreFixtureFromRollout -WhalePath $testWhale -RolloutPath $rolloutPath -ThreadId "thread-1"
& (Join-Path $PSScriptRoot "export-action-map-observability.ps1") -RolloutPath $rolloutPath -JsonlPath $execPath -OutputDir $exportDir -WhalePath $testWhale -ThreadId "thread-1" | Out-Null

$obsPath = Join-Path $exportDir "action-map-observability.json"
$obs = Get-Content -Raw -Encoding UTF8 -LiteralPath $obsPath | ConvertFrom-Json
Assert-Equal ([int]$obs.summary.tasks) 1 "R6 task count"
Assert-Equal ([int]$obs.summary.maps) 1 "R6 map count"
Assert-Equal ([int]$obs.summary.nodes) 5 "R6 node count"
Assert-Equal ([int]$obs.summary.edges) 5 "R6 edge count"
Assert-Equal ([string]$obs.maps[0].rootNodeId) "root" "R6 root id"
Assert-Equal ([string]$obs.maps[0].finishNodeId) "finish" "R6 finish id"
Assert-Equal ([int]$obs.maps[0].revision) 6 "R6 revision"
Assert-True (@($obs.nodes | Where-Object { $_.id -eq "patch" -and $_.kind -eq "work" -and $_.status -eq "completed" }).Count -eq 1) "R6 node identity was not preserved"
Assert-True (@($obs.edges | Where-Object { $_.from -eq "inspect-code" -and $_.to -eq "patch" }).Count -eq 1) "R6 edge direction was not preserved"

. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-graph-health-lib.ps1")
. (Join-Path $PSScriptRoot "taskspace-benchmark/lib/graph-health.ps1")
$health = New-TaskspaceGraphHealthReport $obs "taskspace" "taskspace"
Assert-Equal ([int]$health.source_node_count) 1 "R6 source count"
Assert-Equal ([int]$health.sink_node_count) 1 "R6 sink count"
Assert-Equal ([int]$health.max_depth) 3 "R6 graph depth"
Assert-Equal ([int]$health.max_in_degree) 2 "R6 join indegree"
Assert-Equal ([int]$health.max_out_degree) 2 "R6 fork outdegree"
Assert-Equal ([int]$health.active_frontier_count) 0 "R6 closed frontier"
Assert-True (-not [bool]$health.finish_ready) "R6 closed finish is not ready"
Assert-True ([bool]$health.all_nodes_on_root_finish_path) "R6 rooted reachability"
Assert-True (-not [bool]$health.cycle_detected) "R6 graph should be acyclic"

. (Join-Path $PSScriptRoot "taskspace-benchmark/lib/map-management.ps1")
$managed = @(Get-TaskspaceMapManagedItems $obs)
Assert-True (@($managed | Where-Object { $_.item_type -eq "node" }).Count -eq 5) "R6 managed nodes"
Assert-True (@($managed | Where-Object { $_.id -eq "root" -and $_.map_id -eq "map-1" -and $_.protected_reason -eq "rooted_graph_skeleton" }).Count -eq 1) "R6 root skeleton retention"

. (Join-Path $PSScriptRoot "taskspace-benchmark/lib/cost-instrumentation.ps1")
$projectionPath = Join-Path $RunRoot "projection.txt"
@"
TaskSpaceMapEpochSnapshotR6V1:
- map_id: map-1
- revision: 6
- root_node_id: root
- finish_node_id: finish
- complete: true
  root_source_event_ids:
    - task-event-1
- current_node: none
  active_frontier:
    - none
  map_nodes:
    - root role=task_root status=closed
    - inspect-code role=work status=completed
    - inspect-tests role=work status=completed
    - patch role=work status=completed
    - finish role=finish status=closed
  map_edges:
    - root->inspect-code
    - root->inspect-tests
    - inspect-code->patch
    - inspect-tests->patch
    - patch->finish
  node_details:
    - none
TaskSpaceMapEpochSnapshotR6V1 end.
"@ | Set-Content -LiteralPath $projectionPath -Encoding UTF8
$projection = New-TaskspaceContextProjectionSummary $projectionPath "" ""
Assert-Equal ([string]$projection.availability) "measured" "R6 projection availability"
Assert-Equal ([int]$projection.active_projection_count) 1 "R6 active projection count"
Assert-Equal ([int]$projection.protected_miss_count) 0 "R6 projection required sections"

Write-Host "R6 action-map observability tests passed."
