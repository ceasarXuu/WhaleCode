param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-graph-health-lib.ps1")

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "..\target\test-reports\action-map-graph-health"
}
[void](New-Item -ItemType Directory -Force -Path $OutputDir)

function New-TestNode {
    param(
        [string]$Id,
        [string]$Kind,
        [string]$Status,
        [string]$CompletedAt = "",
        [string]$FirstWorkAt = "",
        [string[]]$AgentThreads = @()
    )
    $events = New-Object System.Collections.Generic.List[object]
    if ($FirstWorkAt) {
        $events.Add([pscustomobject]@{ to = "running"; at = $FirstWorkAt })
    }
    if ($CompletedAt) {
        $events.Add([pscustomobject]@{ to = "completed"; at = $CompletedAt })
    }
    [pscustomobject]@{
        id = $Id
        title = $Id
        kind = $Kind
        status = $Status
        agentThreads = $AgentThreads
        results = @()
        events = @($events.ToArray())
    }
}

function New-TestObs {
    param([object[]]$Nodes, [object[]]$Edges)
    [pscustomobject]@{ nodes = $Nodes; edges = $Edges }
}

function New-TestEdge([string]$From, [string]$To) {
    [pscustomobject]@{ from = $From; to = $To }
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$results = New-Object System.Collections.Generic.List[string]

try {
    $healthy = New-TestObs @(
        (New-TestNode "parser" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:10Z" @("agent-a")),
        (New-TestNode "pricing" "inspect_code_context" "completed" "2026-05-30T00:01:10Z" "2026-05-30T00:00:15Z" @("agent-b")),
        (New-TestNode "impl" "implement_solution" "completed" "2026-05-30T00:03:00Z" "2026-05-30T00:02:00Z"),
        (New-TestNode "test" "smoke_test" "completed" "2026-05-30T00:05:00Z" "2026-05-30T00:04:00Z")
    ) @(
        (New-TestEdge "parser" "impl"),
        (New-TestEdge "pricing" "impl"),
        (New-TestEdge "impl" "test")
    )
    $health = Get-TaskspaceGraphHealth $healthy
    Assert-True ($health.EdgeCount -eq 3) "healthy graph edge count mismatch"
    Assert-True ($health.OrderedEdgeCount -eq 3) "healthy graph ordered edge count mismatch"
    Assert-True ($health.EdgeOrderViolationCount -eq 0) "healthy graph had order violation"
    Assert-True $health.ParallelInspectTracksIndependent "healthy inspect tracks were not independent"
    Assert-True $health.DirectImplementationDependsOnParallelInspectTracks "healthy implementation lacked direct inspect dependencies"
    Assert-True $health.DirectTestDependsOnImplementation "healthy test lacked direct implementation dependency"
    Assert-True ($health.OpenLeafNodeCount -eq 0) "healthy graph had open leaf nodes"
    $results.Add("healthy-direct: PASS")

    $transitiveOnly = New-TestObs @(
        (New-TestNode "parser" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:10Z" @("agent-a")),
        (New-TestNode "pricing" "inspect_code_context" "completed" "2026-05-30T00:01:10Z" "2026-05-30T00:00:15Z" @("agent-b")),
        (New-TestNode "baseline" "smoke_test" "completed" "2026-05-30T00:02:00Z" "2026-05-30T00:01:30Z"),
        (New-TestNode "impl" "implement_solution" "completed" "2026-05-30T00:03:00Z" "2026-05-30T00:02:30Z"),
        (New-TestNode "test" "regression_test" "completed" "2026-05-30T00:04:00Z" "2026-05-30T00:03:30Z")
    ) @(
        (New-TestEdge "parser" "baseline"),
        (New-TestEdge "pricing" "baseline"),
        (New-TestEdge "baseline" "impl"),
        (New-TestEdge "impl" "test")
    )
    $health = Get-TaskspaceGraphHealth $transitiveOnly
    Assert-True $health.ImplementationDependsOnParallelInspectTracks "transitive graph did not detect reachable inspect dependencies"
    Assert-True (-not $health.DirectImplementationDependsOnParallelInspectTracks) "transitive graph was misreported as direct inspect dependency"
    Assert-True $health.DirectTestDependsOnImplementation "transitive graph direct test dependency should still hold"
    $results.Add("direct-vs-transitive: PASS")

    $deadEndImpl = New-TestObs @(
        (New-TestNode "parser" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:10Z" @("agent-a")),
        (New-TestNode "pricing" "inspect_code_context" "completed" "2026-05-30T00:01:10Z" "2026-05-30T00:00:15Z" @("agent-b")),
        (New-TestNode "dead_impl" "implement_solution" "completed" "2026-05-30T00:02:00Z" "2026-05-30T00:01:30Z"),
        (New-TestNode "real_impl" "implement_solution" "completed" "2026-05-30T00:03:00Z" "2026-05-30T00:02:30Z"),
        (New-TestNode "test" "smoke_test" "completed" "2026-05-30T00:04:00Z" "2026-05-30T00:03:30Z")
    ) @(
        (New-TestEdge "parser" "dead_impl"),
        (New-TestEdge "pricing" "dead_impl"),
        (New-TestEdge "real_impl" "test")
    )
    $health = Get-TaskspaceGraphHealth $deadEndImpl
    Assert-True ($health.AnchoredImplementationCount -eq 1) "dead-end graph did not anchor to validation-reaching implementation"
    Assert-True (-not $health.DirectImplementationDependsOnParallelInspectTracks) "dead-end implementation incorrectly satisfied direct inspect dependency"
    $results.Add("anchored-implementation: PASS")

    $unordered = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:05:00Z" "2026-05-30T00:00:00Z" @("agent-a")),
        (New-TestNode "impl" "implement_solution" "completed" "2026-05-30T00:04:00Z" "2026-05-30T00:02:00Z")
    ) @((New-TestEdge "inspect" "impl"))
    $health = Get-TaskspaceGraphHealth $unordered
    Assert-True ($health.EdgeOrderViolationCount -eq 1) "unordered graph did not detect order violation"
    $results.Add("order-violation: PASS")

    $openTerminal = New-TestObs @(
        (New-TestNode "impl" "implement_solution" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z"),
        (New-TestNode "final" "final_synthesis" "running" "" "2026-05-30T00:02:00Z")
    ) @((New-TestEdge "impl" "final"))
    $health = Get-TaskspaceGraphHealth $openTerminal
    Assert-True ($health.OpenFinalSynthesisCount -eq 1) "open final synthesis node was not detected"
    Assert-True ($health.OpenLeafNodeCount -eq 1) "open leaf node was not detected"
    $results.Add("open-terminal: PASS")

    $blockedTerminal = New-TestObs @(
        (New-TestNode "impl" "implement_solution" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z"),
        (New-TestNode "blocked" "inspect_code_context" "blocked" "" "2026-05-30T00:02:00Z")
    ) @((New-TestEdge "impl" "blocked"))
    $health = Get-TaskspaceGraphHealth $blockedTerminal
    Assert-True ($health.OpenLeafNodeCount -eq 0) "blocked leaf node was misclassified as open"
    $results.Add("blocked-terminal: PASS")

    $report = @("# Action Map Graph Health Self-Test", "", "- overall: PASS") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: PASS"
} catch {
    $report = @("# Action Map Graph Health Self-Test", "", "- overall: FAIL", "- error: $($_.Exception.Message)") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: FAIL"
    throw
}
