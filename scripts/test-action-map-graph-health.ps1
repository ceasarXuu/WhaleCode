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
    param(
        [object[]]$Nodes,
        [object[]]$Edges,
        [object[]]$Results = @(),
        [object[]]$SubagentPlans = @(),
        [object[]]$Decisions = @(),
        [string]$RecommendedMode = ""
    )
    [pscustomobject]@{
        nodes = $Nodes
        edges = $Edges
        results = $Results
        subagentPlans = $SubagentPlans
        decisions = $Decisions
        recommendedMode = $RecommendedMode
    }
}

function New-TestEdge([string]$From, [string]$To) {
    [pscustomobject]@{ from = $From; to = $To }
}

function New-TestSubagentPlan([string]$Id, [string]$NodeId, [string[]]$ResultIds = @()) {
    [pscustomobject]@{
        id = $Id
        parentNodeId = $NodeId
        status = if ($ResultIds.Count -gt 0) { "result_recorded" } else { "planned" }
        resultIds = $ResultIds
    }
}

function New-TestResult {
    param(
        [string]$Id,
        [string]$NodeId,
        [string]$ThreadId,
        [string]$PlanId,
        [string]$Validity = "accepted",
        [string[]]$AdoptedByDecisions = @(),
        [string[]]$AdoptedByFacts = @(),
        [string[]]$AdoptedByHypotheses = @(),
        [string[]]$AdoptedByNodes = @()
    )
    [pscustomobject]@{
        id = $Id
        nodeId = $NodeId
        sourceThreadId = $ThreadId
        subagentPlanId = $PlanId
        validity = $Validity
        adoption = [pscustomobject]@{
            adoptedByDecisions = $AdoptedByDecisions
            adoptedByFacts = $AdoptedByFacts
            adoptedByHypotheses = $AdoptedByHypotheses
            adoptedByCriteria = @()
            adoptedByNodes = $AdoptedByNodes
        }
    }
}

function New-TestDecision([string]$Id, [string[]]$DependsOnResults) {
    [pscustomobject]@{ id = $Id; dependsOnResults = $DependsOnResults }
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

    $unixMilliseconds = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "1784158601384" "1784158599383"),
        (New-TestNode "impl" "implement_solution" "completed" "1784158613278" "1784158610592")
    ) @((New-TestEdge "inspect" "impl"))
    $health = Get-TaskspaceGraphHealth $unixMilliseconds
    Assert-True ($health.EdgeOrderViolationCount -eq 0) "Unix millisecond event times were not ordered"
    Assert-True ((ConvertFrom-TaskspaceEventTime "1784158601").Kind -eq [DateTimeKind]::Utc) "Unix second event time was not normalized to UTC"
    $results.Add("unix-event-time: PASS")

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

    $unusedSubagent = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a"))
    ) @() @(
        (New-TestResult "result-1" "inspect" "agent-a" "subagent-plan-1")
    ) @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @("result-1"))
    )
    $health = Get-TaskspaceGraphHealth $unusedSubagent
    Assert-True ($health.SpawnCount -eq 1) "unused subagent spawn count mismatch"
    Assert-True ($health.AcceptedSubagentResultCount -eq 1) "unused subagent accepted result count mismatch"
    Assert-True ($health.DecisionsSupportedBySubagentResultCount -eq 0) "unused subagent result incorrectly supported a decision"
    Assert-True (@($health.Warnings | Where-Object { $_.Code -eq "subagent_no_adoption" }).Count -eq 1) "unused subagent warning missing"
    $results.Add("subagent-unused-warning: PASS")

    $nodeOnlyAdoption = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a"))
    ) @() @(
        (New-TestResult "result-1" "inspect" "agent-a" "subagent-plan-1" "accepted" @() @() @() @("inspect"))
    ) @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @("result-1"))
    ) @(
        (New-TestDecision "decision-1" @())
    )
    $health = Get-TaskspaceGraphHealth $nodeOnlyAdoption
    Assert-True ($health.AdoptedSubagentResultCount -eq 1) "node-only adopted result should count as adopted"
    Assert-True ($health.DecisionsSupportedBySubagentResultCount -eq 0) "node-only adoption must not count as decision yield"
    Assert-True ($health.SubagentDecisionYield -eq 0) "node-only adoption yielded nonzero decision yield"
    $results.Add("subagent-node-only-no-yield: PASS")

    $decisionYield = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a")),
        (New-TestNode "inspect2" "inspect_code_context" "completed" "2026-05-30T00:01:10Z" "2026-05-30T00:00:05Z" @("agent-b"))
    ) @() @(
        (New-TestResult "result-1" "inspect" "agent-a" "subagent-plan-1" "accepted" @("decision-1")),
        (New-TestResult "result-2" "inspect2" "agent-b" "subagent-plan-2" "questioned" @("decision-1"))
    ) @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @("result-1")),
        (New-TestSubagentPlan "subagent-plan-2" "inspect2" @("result-2"))
    ) @(
        (New-TestDecision "decision-1" @("result-1", "result-2"))
    )
    $health = Get-TaskspaceGraphHealth $decisionYield
    Assert-True ($health.DecisionsSupportedBySubagentResultCount -eq 1) "accepted decision-supporting subagent result was not counted exactly once"
    Assert-True ($health.SubagentDecisionYield -eq 0.5) "decision yield should be accepted decision results divided by spawn count"
    $results.Add("subagent-decision-yield: PASS")

    $staleDecisionAdoption = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a"))
    ) @() @(
        (New-TestResult "result-1" "inspect" "agent-a" "subagent-plan-1" "accepted" @("deleted-decision"))
    ) @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @("result-1"))
    ) @(
        (New-TestDecision "decision-1" @("result-1"))
    )
    $health = Get-TaskspaceGraphHealth $staleDecisionAdoption
    Assert-True ($health.DecisionsSupportedBySubagentResultCount -eq 0) "stale decision adoption incorrectly counted as current decision yield"
    Assert-True (@($health.Warnings | Where-Object { $_.Code -eq "subagent_no_decision_yield" }).Count -eq 1) "stale decision adoption did not emit missing-yield warning"
    $results.Add("subagent-stale-decision-no-yield: PASS")

    $exportedShape = [pscustomobject]@{
        tasks = @([pscustomobject]@{
                problemLedger = [pscustomobject]@{
                    decisions = @(
                        (New-TestDecision "decision-1" @("result-1")),
                        (New-TestDecision "decision-2" @("result-2"))
                    )
                }
            })
        maps = @([pscustomobject]@{
                subagentPlans = @(
                    (New-TestSubagentPlan "subagent-plan-1" "inspect" @("result-1")),
                    (New-TestSubagentPlan "subagent-plan-2" "inspect2" @("result-2"))
                )
            })
        nodes = @(
            [pscustomobject]@{
                id = "inspect"; kind = "inspect_code_context"; status = "completed"; title = "inspect"; agentThreads = @("agent-a"); events = @()
                results = @([pscustomobject]@{
                        resultId = "result-1"; nodeId = "inspect"; sourceThreadId = "agent-a"; subagentPlanId = "subagent-plan-1"
                        evidencePackage = [pscustomobject]@{ validity = "accepted"; adoption = [pscustomobject]@{ adoptedByDecisions = @("decision-1") } }
                    })
            },
            [pscustomobject]@{
                id = "inspect2"; kind = "inspect_code_context"; status = "completed"; title = "inspect2"; agentThreads = @("agent-b"); events = @()
                results = @([pscustomobject]@{
                        resultId = "result-2"; nodeId = "inspect2"; sourceThreadId = "agent-b"; subagentPlanId = "subagent-plan-2"
                        evidencePackage = [pscustomobject]@{ validity = "accepted"; adoption = [pscustomobject]@{ adoptedByDecisions = @("decision-2") } }
                    })
            }
        )
        edges = @()
    }
    $health = Get-TaskspaceGraphHealth $exportedShape
    Assert-True ($health.DecisionsSupportedBySubagentResultCount -eq 2) "exported observability resultId shape collapsed or missed decision yield"
    Assert-True ($health.SubagentDecisionYield -eq 1) "exported observability resultId shape yielded wrong ratio"
    $results.Add("subagent-exported-resultid-decision-yield: PASS")

    $thinMode = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a"))
    ) @() @() @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @())
    ) @() "thin"
    $health = Get-TaskspaceGraphHealth $thinMode
    Assert-True $health.ThinModeRecommended "thin mode recommendation was not reported"
    Assert-True ($health.ThinModeReason -eq "explicit report input") "explicit thin mode reason missing"
    Assert-True $health.ThinModeViolation "thin mode violation warning flag missing"
    Assert-True (@($health.Warnings | Where-Object { $_.Code -eq "thin_mode_violation" }).Count -eq 1) "thin mode warning missing"
    $results.Add("thin-mode-report-only-warning: PASS")

    $derivedThinMode = New-TestObs @(
        (New-TestNode "inspect" "inspect_code_context" "completed" "2026-05-30T00:01:00Z" "2026-05-30T00:00:00Z" @("agent-a"))
    ) @() @() @(
        (New-TestSubagentPlan "subagent-plan-1" "inspect" @())
    )
    $health = Get-TaskspaceGraphHealth $derivedThinMode
    Assert-True $health.ThinModeRecommended "simple graph did not derive thin mode recommendation"
    Assert-True ($health.ThinModeReason -match "small read-only graph") "derived thin mode reason missing"
    Assert-True $health.ThinModeViolation "derived thin mode violation flag missing"
    $results.Add("thin-mode-derived-report-only-warning: PASS")

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
