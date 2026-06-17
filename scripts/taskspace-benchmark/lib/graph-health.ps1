function Get-TaskspaceSafeRatio {
    param([double]$Numerator, [double]$Denominator)
    if ($Denominator -le 0) { return 0.0 }
    [math]::Round($Numerator / $Denominator, 4)
}

function Get-TaskspaceResultAdoptionState {
    param($Result)
    $ep = if ($Result -and $Result.PSObject.Properties.Name -contains "evidencePackage") { $Result.evidencePackage } else { $null }
    if ($null -ne $ep -and $ep.PSObject.Properties.Name -contains "adoptionState") {
        return [string]$ep.adoptionState
    }
    if ($null -ne $Result -and $Result.PSObject.Properties.Name -contains "adoptionState") {
        return [string]$Result.adoptionState
    }
    "legacy_unset"
}

function Get-TaskspaceBenchmarkResultId {
    param($Result)
    if ($Result -and $Result.PSObject.Properties.Name -contains "id") { return [string]$Result.id }
    if ($Result -and $Result.PSObject.Properties.Name -contains "resultId") { return [string]$Result.resultId }
    if ($Result -and $Result.PSObject.Properties.Name -contains "result_id") { return [string]$Result.result_id }
    ""
}

function Get-TaskspaceBenchmarkResultAdoption {
    param($Result)
    if (-not $Result) { return $null }
    if ($Result.PSObject.Properties.Name -contains "adoption") { return $Result.adoption }
    if ($Result.PSObject.Properties.Name -contains "evidencePackage" -and
        $null -ne $Result.evidencePackage -and
        $Result.evidencePackage.PSObject.Properties.Name -contains "adoption") {
        return $Result.evidencePackage.adoption
    }
    return $null
}

function Test-TaskspaceBenchmarkResultSupportsDecision {
    param($Result, [object[]]$Decisions)
    $resultId = Get-TaskspaceBenchmarkResultId $Result
    if ([string]::IsNullOrWhiteSpace($resultId)) { return $false }
    $adoption = Get-TaskspaceBenchmarkResultAdoption $Result
    $adoptedDecisionIds = @()
    if ($adoption) {
        foreach ($name in @("adoptedByDecisions", "adopted_by_decisions")) {
            if ($adoption.PSObject.Properties.Name -contains $name -and @($adoption.$name).Count -gt 0) {
                $adoptedDecisionIds = @($adoption.$name)
                break
            }
        }
    }
    foreach ($decision in @($Decisions)) {
        $decisionId = if ($decision.PSObject.Properties.Name -contains "id") { [string]$decision.id } elseif ($decision.PSObject.Properties.Name -contains "Id") { [string]$decision.Id } else { "" }
        foreach ($name in @("dependsOnResults", "depends_on_results")) {
            $adoptedCurrentDecision = $adoptedDecisionIds.Count -eq 0 -or $adoptedDecisionIds -contains $decisionId
            if ($decision.PSObject.Properties.Name -contains $name -and @($decision.$name) -contains $resultId -and $adoptedCurrentDecision) {
                return $true
            }
        }
    }
    if ($adoptedDecisionIds.Count -gt 0) { return $false }
    return $false
}

function New-TaskspaceGraphHealthReport {
    param(
        $Observability,
        [string]$Mode = "",
        [string]$LogicalMode = ""
    )
    $nodes = if ($Observability) { @($Observability.nodes) } else { @() }
    $edges = if ($Observability) { @($Observability.edges) } else { @() }
    $toolCalls = if ($Observability) { @($Observability.toolCalls) } else { @() }
    $results = @($nodes | ForEach-Object { @($_.results) })
    $reviewableResults = @($nodes | ForEach-Object {
            $nodeKind = [string]$_.kind
            @($_.results | Where-Object {
                    [string]$_.kind -eq "result" -and $nodeKind -ne "final_synthesis"
                })
        })
    $legacyHealth = Get-TaskspaceGraphHealth $Observability
    $resultCount = @($results).Count
    $accepted = @($results | Where-Object { [string]$_.validity -eq "accepted" })
    $unreviewed = @($results | Where-Object { [string]$_.validity -eq "unreviewed" })
    $reviewableUnreviewed = @($reviewableResults | Where-Object { [string]$_.validity -eq "unreviewed" })
    $questionedOrInvalid = @($results | Where-Object { [string]$_.validity -in @("questioned", "invalid") })
    $acceptedAdopted = @($accepted | Where-Object { (Get-TaskspaceResultAdoptionState $_) -in @("accepted_adopted", "adopted") })
    $acceptedWithAdoptionState = @($accepted | Where-Object { (Get-TaskspaceResultAdoptionState $_) -ne "legacy_unset" })
    $adoptionMetricState = if (@($accepted).Count -eq 0) {
        "no_accepted_results"
    } elseif (@($acceptedWithAdoptionState).Count -eq 0) {
        "unsupported_legacy"
    } else {
        "measured"
    }
    $decisionEvents = if ($Observability) {
        @($Observability.timeline | Where-Object {
            [string]$_.kind -eq "cognitive_state_updated" -and
            [string]$_.details.updateKind -match "(?i)decision"
        })
    } else { @() }
    $decisions = @()
    if ($Observability -and $Observability.PSObject.Properties.Name -contains "decisions") {
        $decisions = @($Observability.decisions | Where-Object { $null -ne $_ })
    }
    elseif ($Observability -and $Observability.PSObject.Properties.Name -contains "tasks") {
        $decisions = @($Observability.tasks | ForEach-Object { @($_.problemLedger.decisions) + @($_.problem_ledger.decisions) } | Where-Object { $null -ne $_ })
    }
    $blockedNodes = @($nodes | Where-Object { [string]$_.status -eq "blocked" })
    $spawnCalls = @($toolCalls | Where-Object { [string]$_.tool -eq "spawn_agent" -and [string]$_.status -eq "completed" })
    $subagentPlans = if ($Observability -and $Observability.PSObject.Properties.Name -contains "maps") {
        @($Observability.maps | ForEach-Object { @($_.subagentPlans) + @($_.subagent_plans) } | Where-Object { $null -ne $_ })
    } else { @() }
    $subagentSpawnCount = if (@($subagentPlans).Count -gt 0) { @($subagentPlans).Count } else { @($spawnCalls).Count }
    $subagentResultCount = 0
    $subagentAdoptedCount = 0
    $subagentDecisionResultIds = [System.Collections.Generic.HashSet[string]]::new()
    $subagentThreadIds = @($nodes | ForEach-Object {
            @($_.leases | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.agentThreadId) } | ForEach-Object { [string]$_.agentThreadId })
        } | Sort-Object -Unique)
    foreach ($result in $results) {
        $isSubagentResult = (
            ($result.PSObject.Properties.Name -contains "subagentPlanId" -and -not [string]::IsNullOrWhiteSpace([string]$result.subagentPlanId)) -or
            ($result.PSObject.Properties.Name -contains "subagent_plan_id" -and -not [string]::IsNullOrWhiteSpace([string]$result.subagent_plan_id)) -or
            ($subagentThreadIds -contains [string]$result.sourceThreadId)
        )
        if ($isSubagentResult) {
            $subagentResultCount++
            if ((Get-TaskspaceResultAdoptionState $result) -in @("accepted_adopted", "adopted")) {
                $subagentAdoptedCount++
            }
            if ([string]$result.validity -eq "accepted" -and (Test-TaskspaceBenchmarkResultSupportsDecision $result $decisions)) {
                [void]$subagentDecisionResultIds.Add((Get-TaskspaceBenchmarkResultId $result))
            }
        }
    }
    $nodeCount = @($nodes).Count
    $decisionCount = @($decisionEvents).Count
    $warnings = New-Object System.Collections.Generic.List[string]
    $unreviewedRatio = Get-TaskspaceSafeRatio @($unreviewed).Count $resultCount
    $reviewableUnreviewedRatio = Get-TaskspaceSafeRatio @($reviewableUnreviewed).Count @($reviewableResults).Count
    $adoptionRate = if ($adoptionMetricState -eq "unsupported_legacy") { $null } else { Get-TaskspaceSafeRatio @($acceptedAdopted).Count @($accepted).Count }
    $decisionDensity = Get-TaskspaceSafeRatio $decisionCount $nodeCount
    $blockedRatio = Get-TaskspaceSafeRatio @($blockedNodes).Count $nodeCount
    $nodeInflationRatio = if ($decisionCount -gt 0) { Get-TaskspaceSafeRatio $nodeCount $decisionCount } else { [double]$nodeCount }
    if (@($reviewableResults).Count -gt 0 -and $reviewableUnreviewedRatio -gt 0.3) { $warnings.Add("high_unreviewed_result_ratio") }
    if ($nodeCount -ge 4 -and $decisionDensity -lt 0.25) { $warnings.Add("low_decision_density") }
    if ($nodeCount -gt 0 -and $blockedRatio -gt 0.25) { $warnings.Add("high_blocked_node_ratio") }
    if ($nodeInflationRatio -gt 12) { $warnings.Add("node_inflation_high") }
    if ($subagentSpawnCount -gt 0 -and $subagentAdoptedCount -eq 0 -and $adoptionMetricState -eq "measured") { $warnings.Add("subagent_no_adoption") }
    if ($subagentSpawnCount -gt 0 -and $subagentDecisionResultIds.Count -eq 0) { $warnings.Add("subagent_no_decision_yield") }
    if ([int]$legacyHealth.OpenFinalSynthesisCount -gt 0) { $warnings.Add("synthesis_not_ready") }
    [pscustomobject]@{
        schema_version = "taskspace-graph-health-v1"
        mode = $Mode
        logical_mode = $LogicalMode
        node_count = $nodeCount
        edge_count = @($edges).Count
        result_count = $resultCount
        decision_count = $decisionCount
        accepted_result_count = @($accepted).Count
        accepted_adopted_result_count = @($acceptedAdopted).Count
        unreviewed_result_count = @($unreviewed).Count
        questioned_or_invalid_result_count = @($questionedOrInvalid).Count
        unreviewed_result_ratio = $unreviewedRatio
        reviewable_result_count = @($reviewableResults).Count
        reviewable_unreviewed_result_count = @($reviewableUnreviewed).Count
        reviewable_unreviewed_result_ratio = $reviewableUnreviewedRatio
        result_adoption_rate = $adoptionRate
        decision_density = $decisionDensity
        blocked_node_ratio = $blockedRatio
        node_inflation_ratio = $nodeInflationRatio
        metric_availability = [ordered]@{
            result_adoption = $adoptionMetricState
            decision_density = "measured"
            open_question_closure = "unsupported"
        }
        open_question_closure_rate = $null
        subagent_decision_yield = Get-TaskspaceSafeRatio $subagentDecisionResultIds.Count $subagentSpawnCount
        subagent_spawn_count = $subagentSpawnCount
        subagent_result_count = $subagentResultCount
        subagent_decision_result_count = $subagentDecisionResultIds.Count
        thin_mode_violation = (@($warnings) -contains "thin_mode_violation")
        legacy = $legacyHealth
        warnings = @($warnings.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
}

function Write-TaskspaceGraphHealthReport {
    param(
        [Parameter(Mandatory = $true)]$Report,
        [Parameter(Mandatory = $true)][string]$Path
    )
    ($Report | ConvertTo-Json -Depth 30) | Set-Content -LiteralPath $Path -Encoding UTF8
}
