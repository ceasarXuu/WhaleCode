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
    $legacyHealth = Get-TaskspaceGraphHealth $Observability
    $resultCount = @($results).Count
    $accepted = @($results | Where-Object { [string]$_.validity -eq "accepted" })
    $unreviewed = @($results | Where-Object { [string]$_.validity -eq "unreviewed" })
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
    $blockedNodes = @($nodes | Where-Object { [string]$_.status -eq "blocked" })
    $spawnCalls = @($toolCalls | Where-Object { [string]$_.tool -eq "spawn_agent" -and [string]$_.status -eq "completed" })
    $subagentResultCount = 0
    $subagentAdoptedCount = 0
    $subagentThreadIds = @($nodes | ForEach-Object {
            @($_.leases | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.agentThreadId) } | ForEach-Object { [string]$_.agentThreadId })
        } | Sort-Object -Unique)
    foreach ($result in $results) {
        if ($subagentThreadIds -contains [string]$result.sourceThreadId) {
            $subagentResultCount++
            if ((Get-TaskspaceResultAdoptionState $result) -in @("accepted_adopted", "adopted")) {
                $subagentAdoptedCount++
            }
        }
    }
    $nodeCount = @($nodes).Count
    $decisionCount = @($decisionEvents).Count
    $warnings = New-Object System.Collections.Generic.List[string]
    $unreviewedRatio = Get-TaskspaceSafeRatio @($unreviewed).Count $resultCount
    $adoptionRate = if ($adoptionMetricState -eq "unsupported_legacy") { $null } else { Get-TaskspaceSafeRatio @($acceptedAdopted).Count @($accepted).Count }
    $decisionDensity = Get-TaskspaceSafeRatio $decisionCount $nodeCount
    $blockedRatio = Get-TaskspaceSafeRatio @($blockedNodes).Count $nodeCount
    $nodeInflationRatio = if ($decisionCount -gt 0) { Get-TaskspaceSafeRatio $nodeCount $decisionCount } else { [double]$nodeCount }
    if ($resultCount -gt 0 -and $unreviewedRatio -gt 0.3) { $warnings.Add("high_unreviewed_result_ratio") }
    if ($nodeCount -ge 4 -and $decisionDensity -lt 0.25) { $warnings.Add("low_decision_density") }
    if ($nodeCount -gt 0 -and $blockedRatio -gt 0.25) { $warnings.Add("high_blocked_node_ratio") }
    if ($nodeInflationRatio -gt 12) { $warnings.Add("node_inflation_high") }
    if (@($spawnCalls).Count -gt 0 -and $subagentAdoptedCount -eq 0 -and $adoptionMetricState -eq "measured") { $warnings.Add("subagent_no_adoption") }
    if ([string]$LogicalMode -eq "taskspace" -and $nodeCount -le 1 -and @($spawnCalls).Count -eq 0) { $warnings.Add("thin_mode_violation") }
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
        subagent_decision_yield = Get-TaskspaceSafeRatio $subagentAdoptedCount @($spawnCalls).Count
        subagent_spawn_count = @($spawnCalls).Count
        subagent_result_count = $subagentResultCount
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
