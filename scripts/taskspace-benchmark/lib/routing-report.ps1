function Read-TaskspaceRoutingJson {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    try {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
    } catch {
        $null
    }
}

function Get-TaskspaceRoutingMetricInt {
    param(
        [object]$Metrics,
        [Parameter(Mandatory = $true)][string]$Name,
        [int]$DefaultValue = 0
    )
    if ($null -ne $Metrics -and $Metrics.PSObject.Properties.Name -contains $Name -and $null -ne $Metrics.$Name) {
        return [int]$Metrics.$Name
    }
    $DefaultValue
}

function Get-TaskspaceRoutingMetricBool {
    param(
        [object]$Metrics,
        [Parameter(Mandatory = $true)][string]$Name,
        [bool]$DefaultValue = $false
    )
    if ($null -ne $Metrics -and $Metrics.PSObject.Properties.Name -contains $Name -and $null -ne $Metrics.$Name) {
        return [bool]$Metrics.$Name
    }
    $DefaultValue
}

function Write-TaskspacePairRoutingReport {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)]$Assessment
    )
    $path = Join-Path $PairDir "pair-routing-report.md"
    $lines = New-Object System.Collections.Generic.List[string]
    [void]$lines.Add("# TaskSpace Pair Routing Report")
    [void]$lines.Add("")
    [void]$lines.Add("- recommended_mode: $($Assessment.recommended_mode)")
    [void]$lines.Add("- confidence: $($Assessment.confidence)")
    [void]$lines.Add("- trigger_reasons: $(@($Assessment.trigger_reasons) -join ', ')")
    [void]$lines.Add("- taskspace_nodes: $($Assessment.taskspace_nodes)")
    [void]$lines.Add("- taskspace_spawn_agent_calls: $($Assessment.taskspace_spawn_agent_calls)")
    [void]$lines.Add("- taskspace_business_success: $($Assessment.taskspace_business_success)")
    [void]$lines.Add("- clean: $($Assessment.clean)")
    [void]$lines.Add("- routing_mistakes: $(@($Assessment.routing_mistakes) -join ', ')")
    $lines | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}

function Write-TaskspaceSuiteRoutingSummary {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(ValueFromRemainingArguments = $true)]$Ignored
    )
    $summaryPath = Join-Path $RunDir "suite-routing-summary.json"
    $routingDecision = Read-TaskspaceRoutingJson (Join-Path $RunDir "routing-decision.json")
    $routingDecision = @($routingDecision | Where-Object { $_ -and $_.PSObject.Properties.Name -contains "recommended_mode" }) | Select-Object -Last 1
    if ($null -eq $routingDecision) {
        $summary = [pscustomobject]@{
            schema_version = "taskspace-suite-routing-summary-v1"
            availability = "routing_decision_missing"
            assessed_pair_count = 0
            clean_pair_count = 0
            routing_mistake_count = 1
            routing_mistakes = @("routing_decision_missing")
            pairs = @()
        }
        Write-TaskspaceJson $summary $summaryPath
        return $summary
    }
    $pairs = @(Get-ChildItem -LiteralPath $RunDir -Directory -Filter "pair-*" -ErrorAction SilentlyContinue | Sort-Object Name)
    $assessments = New-Object System.Collections.Generic.List[object]
    foreach ($pair in $pairs) {
        $pairDir = [string]$pair.FullName
        $decisionObject = @($routingDecision | Where-Object { $_ -and $_.PSObject.Properties.Name -contains "recommended_mode" }) | Select-Object -Last 1
        $left = Read-TaskspaceRoutingJson (Join-Path $pairDir "left\artifacts\metrics.json")
        $right = Read-TaskspaceRoutingJson (Join-Path $pairDir "right\artifacts\metrics.json")
        $taskspace = @($left, $right) | Where-Object { $_ -and [string]$_.logical_mode -eq "taskspace" } | Select-Object -First 1
        $mistakes = New-Object System.Collections.Generic.List[string]
        if ($null -eq $taskspace) {
            [void]$mistakes.Add("taskspace_metrics_missing")
        } else {
            $mode = [string]$decisionObject.recommended_mode
            $nodeBudget = [int]$decisionObject.initial_constraints.node_budget
            $spawnBudget = if ($decisionObject.initial_constraints.subagent_allowed) { [int]::MaxValue } else { 0 }
            $nodes = Get-TaskspaceRoutingMetricInt -Metrics $taskspace -Name "nodes" -DefaultValue 0
            $spawnCalls = Get-TaskspaceRoutingMetricInt -Metrics $taskspace -Name "spawn_agent_calls" -DefaultValue 0
            if ($mode -eq "thin" -and $spawnCalls -gt $spawnBudget) { [void]$mistakes.Add("thin_spawned_subagent") }
            if ($mode -eq "thin" -and $nodes -gt $nodeBudget) { [void]$mistakes.Add("thin_node_budget_exceeded") }
            if ($mode -eq "verification_first" -and (Get-TaskspaceRoutingMetricBool -Metrics $taskspace -Name "public_validation_skipped" -DefaultValue $false)) {
                [void]$mistakes.Add("verification_first_validation_skipped")
            }
            if (-not (Get-TaskspaceRoutingMetricBool -Metrics $taskspace -Name "business_success" -DefaultValue $false)) {
                [void]$mistakes.Add("taskspace_not_business_success")
            }
        }
        $assessment = [pscustomobject]@{
            schema_version = "taskspace-pair-routing-assessment-v1"
            pair_dir = $pairDir
            recommended_mode = [string]$decisionObject.recommended_mode
            confidence = [string]$decisionObject.confidence
            trigger_reasons = @($decisionObject.trigger_reasons)
            taskspace_nodes = if ($taskspace) { Get-TaskspaceRoutingMetricInt -Metrics $taskspace -Name "nodes" -DefaultValue 0 } else { $null }
            taskspace_spawn_agent_calls = if ($taskspace) { Get-TaskspaceRoutingMetricInt -Metrics $taskspace -Name "spawn_agent_calls" -DefaultValue 0 } else { $null }
            taskspace_business_success = if ($taskspace) { Get-TaskspaceRoutingMetricBool -Metrics $taskspace -Name "business_success" -DefaultValue $false } else { $false }
            routing_mistakes = @($mistakes.ToArray())
            clean = ($mistakes.Count -eq 0)
        }
        Write-TaskspacePairRoutingReport -PairDir $pairDir -Assessment $assessment | Out-Null
        [void]$assessments.Add($assessment)
    }
    $allMistakes = @($assessments.ToArray() | ForEach-Object { @($_.routing_mistakes) } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    $summary = [pscustomobject]@{
        schema_version = "taskspace-suite-routing-summary-v1"
        availability = "measured"
        router_status = [string]$routingDecision.status
        recommended_mode = [string]$routingDecision.recommended_mode
        confidence = [string]$routingDecision.confidence
        trigger_reasons = @($routingDecision.trigger_reasons)
        escalation_rules = @($routingDecision.escalation_rules)
        assessed_pair_count = [int]$assessments.Count
        clean_pair_count = [int]@($assessments.ToArray() | Where-Object { [bool]$_.clean }).Count
        routing_mistake_count = [int]$allMistakes.Count
        routing_mistakes = @($allMistakes | Sort-Object -Unique)
        pairs = @($assessments.ToArray())
    }
    Write-TaskspaceJson $summary $summaryPath
    $summary
}
