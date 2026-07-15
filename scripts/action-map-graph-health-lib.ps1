function Get-TaskspaceGraphHealth($Obs) {
    $empty = [pscustomobject]@{
        SchemaVersion = "taskspace-graph-health-v2"
        GeneratedAtUtc = (Get-Date).ToUniversalTime().ToString("o")
        EdgeCount = 0
        OrderedEdgeCount = 0
        EdgeOrderViolationCount = 0
        ImplementationHasIncomingEdge = $false
        TestDependsOnImplementation = $false
        DirectTestDependsOnImplementation = $false
        ParserPricingIndependent = $false
        ImplementationDependsOnParserAndPricing = $false
        DirectImplementationDependsOnParallelInspectTracks = $false
        AnchoredImplementationCount = 0
        ParallelInspectTrackCount = 0
        ParallelInspectTracksIndependent = $false
        ImplementationDependsOnParallelInspectTracks = $false
        OpenLeafNodeCount = 0
        OpenFinalSynthesisCount = 0
        SpawnCount = 0
        SubagentPlanCount = 0
        SubagentResultCount = 0
        AcceptedSubagentResultCount = 0
        AdoptedSubagentResultCount = 0
        DecisionsSupportedBySubagentResultCount = 0
        SubagentDecisionYield = 0.0
        ThinModeRecommended = $false
        RecommendedMode = ""
        ThinModeReason = ""
        ThinModeViolation = $false
        Warnings = @()
    }
    if (-not $Obs) { return $empty }

    $nodes = @(Get-TaskspaceObsNodes $Obs)
    $edges = @(Get-TaskspaceObsEdges $Obs)
    $results = @(Get-TaskspaceObsResults $Obs $nodes)
    $subagentPlans = @(Get-TaskspaceObsSubagentPlans $Obs)
    $decisions = @(Get-TaskspaceObsDecisions $Obs)
    $warnings = New-Object System.Collections.Generic.List[object]
    $byId = @{}
    foreach ($node in $nodes) { $byId[[string]$node.id] = $node }

    $implNodes = @($nodes | Where-Object { [string]$_.kind -eq "implement_solution" })
    $testNodes = @($nodes | Where-Object { [string]$_.kind -match "smoke_test|regression_test" })
    $parserNodes = @($nodes | Where-Object { [string]$_.kind -eq "inspect_code_context" -and [string]$_.title -match "(?i)parser|parse|sku" })
    $pricingNodes = @($nodes | Where-Object { [string]$_.kind -eq "inspect_code_context" -and [string]$_.title -match "(?i)pricing|discount|invoice|shipping" })
    $parallelInspectNodes = @($nodes | Where-Object { [string]$_.kind -eq "inspect_code_context" -and @($_.agentThreads).Count -gt 0 })

    $incomingToImpl = @($edges | Where-Object {
            $to = [string]$_.to
            @($implNodes | Where-Object { [string]$_.id -eq $to }).Count -gt 0
        })
    $implIds = @($implNodes | ForEach-Object { [string]$_.id })
    $testIds = @($testNodes | ForEach-Object { [string]$_.id })
    $finalIds = @($nodes | Where-Object { [string]$_.kind -eq "final_synthesis" } | ForEach-Object { [string]$_.id })
    $anchoredImplIds = @($implIds | Where-Object {
            (Test-TaskspacePathExists $edges @($_) $testIds) -or
            (Test-TaskspacePathExists $edges @($_) $finalIds)
        })
    if ($anchoredImplIds.Count -eq 0) { $anchoredImplIds = $implIds }
    $testDependsOnImplementation = Test-TaskspacePathExists $edges $implIds $testIds
    $directTestDependsOnImplementation = Test-TaskspaceDirectEdgeExists $edges $implIds $testIds

    $parserIds = @($parserNodes | ForEach-Object { [string]$_.id })
    $pricingIds = @($pricingNodes | ForEach-Object { [string]$_.id })
    $parserPricingLinked =
        (Test-TaskspacePathExists $edges $parserIds $pricingIds) -or
        (Test-TaskspacePathExists $edges $pricingIds $parserIds)
    $parserPricingIndependent = $parserIds.Count -gt 0 -and $pricingIds.Count -gt 0 -and -not $parserPricingLinked
    $implementationDependsOnParser = Test-TaskspacePathExists $edges $parserIds $anchoredImplIds
    $implementationDependsOnPricing = Test-TaskspacePathExists $edges $pricingIds $anchoredImplIds
    $parallelInspectIds = @($parallelInspectNodes | ForEach-Object { [string]$_.id })
    $parallelTracksLinked = $false
    foreach ($left in $parallelInspectIds) {
        foreach ($right in $parallelInspectIds) {
            if ($left -eq $right) { continue }
            if (Test-TaskspacePathExists $edges @($left) @($right)) { $parallelTracksLinked = $true }
        }
    }
    $implementationDependsOnParallelInspectTracks = $parallelInspectIds.Count -gt 0 -and @($parallelInspectIds | Where-Object {
            Test-TaskspacePathExists $edges @($_) $anchoredImplIds
        }).Count -eq $parallelInspectIds.Count
    $directImplementationDependsOnParallelInspectTracks =
        $parallelInspectIds.Count -gt 0 -and
        (Test-TaskspaceDirectEdgesFromAll $edges $parallelInspectIds $anchoredImplIds)

    $nodesWithOutgoingEdges = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($edge in $edges) { [void]$nodesWithOutgoingEdges.Add([string]$edge.from) }
    $openStatuses = @("pending", "ready", "running")
    $openLeafNodeCount = @($nodes | Where-Object {
            -not $nodesWithOutgoingEdges.Contains([string]$_.id) -and $openStatuses -contains [string]$_.status
        }).Count
    $openFinalSynthesisCount = @($nodes | Where-Object {
            [string]$_.kind -eq "final_synthesis" -and $openStatuses -contains [string]$_.status
        }).Count

    $orderedEdgeCount = 0
    $edgeOrderViolationCount = 0
    foreach ($edge in $edges) {
        $from = [string]$edge.from
        $to = [string]$edge.to
        if (-not ($byId.ContainsKey($from) -and $byId.ContainsKey($to))) { continue }
        $fromCompleted = Get-NodeCompletedAt $byId[$from]
        $toFirstWork = Get-NodeFirstWorkAt $byId[$to]
        if ($fromCompleted -and $toFirstWork) {
            $orderedEdgeCount++
            if ($toFirstWork -lt $fromCompleted) { $edgeOrderViolationCount++ }
        }
    }

    $spawnCount = Get-TaskspaceSpawnCount $Obs $subagentPlans
    $subagentResults = @(Get-TaskspaceSubagentResults $results $nodes)
    $acceptedSubagentResults = @($subagentResults | Where-Object {
            [string](Get-ObjectValue $_ "validity" "Validity" "evidencePackage.validity") -eq "accepted"
        })
    $adoptedSubagentResults = @($acceptedSubagentResults | Where-Object {
            Test-TaskspaceResultHasAnyAdoption $_
        })
    $decisionSupportingResultIds = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($result in $acceptedSubagentResults) {
        if (Test-TaskspaceResultSupportsDecision $result $decisions) {
            [void]$decisionSupportingResultIds.Add((Get-TaskspaceResultId $result))
        }
    }
    foreach ($result in $acceptedSubagentResults) {
        if (-not (Test-TaskspaceResultHasAnyAdoption $result)) {
            $warnings.Add([pscustomobject]@{
                    Code = "subagent_no_adoption"
                    Severity = "warning"
                    Reason = "accepted subagent result has no recorded adoption"
                    EvidenceRefs = @((Get-TaskspaceResultId $result))
                })
        }
    }

    $thinModeRecommendation = Get-TaskspaceThinModeRecommendation $Obs $nodes $edges $implIds $testIds $parallelInspectIds
    $recommendedMode = [string]$thinModeRecommendation.Mode
    $thinModeReason = [string]$thinModeRecommendation.Reason
    $thinModeRecommended = $recommendedMode -eq "thin"
    $thinModeViolation = $thinModeRecommended -and $spawnCount -gt 0
    if ($thinModeViolation) {
        $warnings.Add([pscustomobject]@{
                Code = "thin_mode_violation"
                Severity = "warning"
                Reason = "thin mode was recommended, but subagent spawn activity was observed"
                EvidenceRefs = @("spawn_count=$spawnCount")
            })
    }
    if ($spawnCount -gt 0 -and $decisionSupportingResultIds.Count -eq 0) {
        $warnings.Add([pscustomobject]@{
                Code = "subagent_no_decision_yield"
                Severity = "warning"
                Reason = "subagent activity produced no accepted result supporting a current decision"
                EvidenceRefs = @("spawn_count=$spawnCount")
            })
    }

    [pscustomobject]@{
        SchemaVersion = "taskspace-graph-health-v2"
        GeneratedAtUtc = (Get-Date).ToUniversalTime().ToString("o")
        EdgeCount = $edges.Count
        OrderedEdgeCount = $orderedEdgeCount
        EdgeOrderViolationCount = $edgeOrderViolationCount
        ImplementationHasIncomingEdge = $incomingToImpl.Count -gt 0
        TestDependsOnImplementation = $testDependsOnImplementation
        DirectTestDependsOnImplementation = $directTestDependsOnImplementation
        ParserPricingIndependent = $parserPricingIndependent
        ImplementationDependsOnParserAndPricing = $implementationDependsOnParser -and $implementationDependsOnPricing
        AnchoredImplementationCount = $anchoredImplIds.Count
        ParallelInspectTrackCount = $parallelInspectIds.Count
        ParallelInspectTracksIndependent = $parallelInspectIds.Count -ge 2 -and -not $parallelTracksLinked
        ImplementationDependsOnParallelInspectTracks = $implementationDependsOnParallelInspectTracks
        DirectImplementationDependsOnParallelInspectTracks = $directImplementationDependsOnParallelInspectTracks
        OpenLeafNodeCount = $openLeafNodeCount
        OpenFinalSynthesisCount = $openFinalSynthesisCount
        SpawnCount = $spawnCount
        SubagentPlanCount = $subagentPlans.Count
        SubagentResultCount = $subagentResults.Count
        AcceptedSubagentResultCount = $acceptedSubagentResults.Count
        AdoptedSubagentResultCount = $adoptedSubagentResults.Count
        DecisionsSupportedBySubagentResultCount = $decisionSupportingResultIds.Count
        SubagentDecisionYield = if ($spawnCount -gt 0) { [double]$decisionSupportingResultIds.Count / [double]$spawnCount } else { 0.0 }
        ThinModeRecommended = $thinModeRecommended
        RecommendedMode = $recommendedMode
        ThinModeReason = $thinModeReason
        ThinModeViolation = $thinModeViolation
        Warnings = @($warnings.ToArray())
    }
}

function Get-TaskspaceThinModeRecommendation($Obs, [object[]]$Nodes, [object[]]$Edges, [string[]]$ImplementationIds, [string[]]$TestIds, [string[]]$ParallelInspectIds) {
    $explicitMode = [string](Get-ObjectValue $Obs "recommendedMode" "RecommendedMode" "thinMode.recommendedMode" "graphHealth.recommendedMode")
    if (-not [string]::IsNullOrWhiteSpace($explicitMode)) {
        return [pscustomobject]@{
            Mode = $explicitMode
            Reason = "explicit report input"
        }
    }

    $hasImplementationOrTest = @($ImplementationIds).Count -gt 0 -or @($TestIds).Count -gt 0
    $isSmallLinearGraph = @($Nodes).Count -le 2 -and @($Edges).Count -le 1
    $hasParallelInspectNeed = @($ParallelInspectIds).Count -gt 1
    if ($isSmallLinearGraph -and -not $hasImplementationOrTest -and -not $hasParallelInspectNeed) {
        return [pscustomobject]@{
            Mode = "thin"
            Reason = "small read-only graph without implementation, validation, or parallel inspect need"
        }
    }

    return [pscustomobject]@{
        Mode = "standard"
        Reason = "graph needs normal TaskSpace orchestration"
    }
}

function Get-TaskspaceObsNodes($Obs) {
    if ((Get-ObjectPropertyNames $Obs) -contains "nodes") { return @($Obs.nodes) }
    if ((Get-ObjectPropertyNames $Obs) -contains "maps") {
        return @($Obs.maps | ForEach-Object { @($_.nodes) })
    }
    return @()
}

function Get-TaskspaceObsEdges($Obs) {
    if ((Get-ObjectPropertyNames $Obs) -contains "edges") { return @($Obs.edges) }
    if ((Get-ObjectPropertyNames $Obs) -contains "maps") {
        return @($Obs.maps | ForEach-Object { @($_.edges) })
    }
    return @()
}

function Get-TaskspaceObsSubagentPlans($Obs) {
    if ((Get-ObjectPropertyNames $Obs) -contains "subagentPlans") { return @($Obs.subagentPlans | Where-Object { $null -ne $_ }) }
    if ((Get-ObjectPropertyNames $Obs) -contains "subagent_plans") { return @($Obs.subagent_plans | Where-Object { $null -ne $_ }) }
    if ((Get-ObjectPropertyNames $Obs) -contains "maps") {
        return @($Obs.maps | ForEach-Object { @($_.subagentPlans) + @($_.subagent_plans) } | Where-Object { $null -ne $_ })
    }
    return @()
}

function Get-TaskspaceObsResults($Obs, $Nodes) {
    $direct = New-Object System.Collections.Generic.List[object]
    if ((Get-ObjectPropertyNames $Obs) -contains "results") {
        foreach ($result in @($Obs.results | Where-Object { $null -ne $_ })) { $direct.Add($result) }
    }
    if ((Get-ObjectPropertyNames $Obs) -contains "maps") {
        foreach ($map in @($Obs.maps)) {
            foreach ($result in @($map.results | Where-Object { $null -ne $_ })) { $direct.Add($result) }
        }
    }
    foreach ($node in @($Nodes)) {
        foreach ($result in @($node.results | Where-Object { $null -ne $_ })) { $direct.Add($result) }
    }
    return @($direct.ToArray())
}

function Get-TaskspaceObsDecisions($Obs) {
    $decisions = New-Object System.Collections.Generic.List[object]
    if ((Get-ObjectPropertyNames $Obs) -contains "decisions") {
        foreach ($decision in @($Obs.decisions | Where-Object { $null -ne $_ })) { $decisions.Add($decision) }
    }
    if ((Get-ObjectPropertyNames $Obs) -contains "tasks") {
        foreach ($task in @($Obs.tasks)) {
            foreach ($decision in @($task.problemLedger.decisions | Where-Object { $null -ne $_ })) { $decisions.Add($decision) }
            foreach ($decision in @($task.problem_ledger.decisions | Where-Object { $null -ne $_ })) { $decisions.Add($decision) }
        }
    }
    return @($decisions.ToArray())
}

function Get-TaskspaceSpawnCount($Obs, $SubagentPlans) {
    if (@($SubagentPlans).Count -gt 0) { return @($SubagentPlans).Count }
    if ((Get-ObjectPropertyNames $Obs) -contains "toolCalls") {
        return @($Obs.toolCalls | Where-Object { $_.tool -eq "spawn_agent" -and $_.status -eq "completed" }).Count
    }
    if ((Get-ObjectPropertyNames $Obs) -contains "agents") { return @($Obs.agents).Count }
    return 0
}

function Get-TaskspaceSubagentResults($Results, $Nodes) {
    $agentThreads = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($node in @($Nodes)) {
        foreach ($thread in @($node.agentThreads)) { [void]$agentThreads.Add([string]$thread) }
    }
    return @($Results | Where-Object {
            -not [string]::IsNullOrWhiteSpace([string](Get-ObjectValue $_ "subagentPlanId" "subagent_plan_id")) -or
            $agentThreads.Contains([string](Get-ObjectValue $_ "sourceThreadId" "source_thread_id"))
        })
}

function Test-TaskspaceResultHasAnyAdoption($Result) {
    $adoption = Get-ObjectValue $Result "adoption" "evidencePackage.adoption" "evidence_package.adoption"
    if (-not $adoption) { return $false }
    foreach ($name in @("adoptedByFacts", "adoptedByHypotheses", "adoptedByDecisions", "adoptedByCriteria", "adoptedByNodes", "adopted_by_facts", "adopted_by_hypotheses", "adopted_by_decisions", "adopted_by_criteria", "adopted_by_nodes")) {
        if ((Get-ObjectValueCount $adoption $name) -gt 0) { return $true }
    }
    return $false
}

function Test-TaskspaceResultSupportsDecision($Result, $Decisions) {
    $resultId = Get-TaskspaceResultId $Result
    if ([string]::IsNullOrWhiteSpace($resultId)) { return $false }
    $adoption = Get-ObjectValue $Result "adoption" "evidencePackage.adoption" "evidence_package.adoption"
    $adoptedDecisionIds = @()
    if ($adoption) {
        $adoptedDecisionIds = @(Get-ObjectValueArray $adoption "adoptedByDecisions" "adopted_by_decisions")
        $factOrHypothesisAdoptionCount =
            (Get-ObjectValueCount $adoption "adoptedByFacts" "adopted_by_facts") +
            (Get-ObjectValueCount $adoption "adoptedByHypotheses" "adopted_by_hypotheses")
        if ($adoptedDecisionIds.Count -gt 0 -or $factOrHypothesisAdoptionCount -gt 0) {
            foreach ($decision in @($Decisions)) {
                $decisionId = [string](Get-ObjectValue $decision "id" "Id")
                $dependsOnResult = (Get-ObjectValueArray $decision "dependsOnResults" "depends_on_results") -contains $resultId
                $adoptedCurrentDecision = $adoptedDecisionIds.Count -eq 0 -or $adoptedDecisionIds -contains $decisionId
                if ($dependsOnResult -and $adoptedCurrentDecision) { return $true }
            }
        }
        if ($adoptedDecisionIds.Count -gt 0) { return $false }
    }
    foreach ($decision in @($Decisions)) {
        if ((Get-ObjectValueArray $decision "dependsOnResults" "depends_on_results") -contains $resultId) { return $true }
    }
    return $false
}

function Get-TaskspaceResultId($Result) {
    return [string](Get-ObjectValue $Result "id" "Id" "resultId" "result_id")
}

function Get-ObjectPropertyNames($Object) {
    if (-not $Object) { return @() }
    return @($Object.PSObject.Properties | ForEach-Object { $_.Name })
}

function Get-ObjectValue($Object, [Parameter(ValueFromRemainingArguments = $true)][string[]]$Paths) {
    foreach ($path in $Paths) {
        $value = Get-ObjectPathValue $Object $path
        if ($null -ne $value) { return $value }
    }
    return $null
}

function Get-ObjectValueArray($Object, [Parameter(ValueFromRemainingArguments = $true)][string[]]$Paths) {
    $value = $null
    foreach ($path in $Paths) {
        $candidate = Get-ObjectPathValue $Object $path
        if ($null -ne $candidate) {
            $value = $candidate
            break
        }
    }
    if ($null -eq $value) { return @() }
    if ($value -is [array]) { return @($value | Where-Object { $null -ne $_ -and -not [string]::IsNullOrWhiteSpace([string]$_) }) }
    if ([string]::IsNullOrWhiteSpace([string]$value)) { return @() }
    return @($value)
}

function Get-ObjectValueCount($Object, [Parameter(ValueFromRemainingArguments = $true)][string[]]$Paths) {
    $values = Get-ObjectValueArray $Object $Paths
    return @($values).Count
}

function Get-ObjectPathValue($Object, [string]$Path) {
    if (-not $Object -or [string]::IsNullOrWhiteSpace($Path)) { return $null }
    $current = $Object
    foreach ($part in ($Path -split "\.")) {
        if (-not $current) { return $null }
        $property = $current.PSObject.Properties[$part]
        if (-not $property) { return $null }
        $current = $property.Value
    }
    return $current
}

function Test-TaskspaceDirectEdgeExists([object[]]$Edges, [string[]]$FromIds, [string[]]$ToIds) {
    if ($FromIds.Count -eq 0 -or $ToIds.Count -eq 0) { return $false }
    foreach ($edge in @($Edges)) {
        if ($FromIds -contains [string]$edge.from -and $ToIds -contains [string]$edge.to) {
            return $true
        }
    }
    return $false
}

function Test-TaskspaceDirectEdgesFromAll([object[]]$Edges, [string[]]$FromIds, [string[]]$ToIds) {
    if ($FromIds.Count -eq 0 -or $ToIds.Count -eq 0) { return $false }
    foreach ($fromId in $FromIds) {
        if (-not (Test-TaskspaceDirectEdgeExists $Edges @($fromId) $ToIds)) {
            return $false
        }
    }
    return $true
}

function ConvertFrom-TaskspaceEventTime($Value) {
    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) { return $null }
    [long]$epoch = 0
    if ([long]::TryParse($text, [Globalization.NumberStyles]::Integer, [Globalization.CultureInfo]::InvariantCulture, [ref]$epoch)) {
        if ([Math]::Abs($epoch) -ge 100000000000) {
            return [datetimeoffset]::FromUnixTimeMilliseconds($epoch).UtcDateTime
        }
        return [datetimeoffset]::FromUnixTimeSeconds($epoch).UtcDateTime
    }
    return [datetime]::Parse($text, [Globalization.CultureInfo]::InvariantCulture)
}

function Get-NodeCompletedAt($Node) {
    foreach ($event in @($Node.events)) {
        if ([string]$event.to -eq "completed" -and -not [string]::IsNullOrWhiteSpace([string]$event.at)) {
            return ConvertFrom-TaskspaceEventTime $event.at
        }
    }
    return $null
}

function Test-TaskspacePathExists([object[]]$Edges, [string[]]$FromIds, [string[]]$ToIds) {
    if ($FromIds.Count -eq 0 -or $ToIds.Count -eq 0) { return $false }
    $targetSet = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($id in $ToIds) { [void]$targetSet.Add($id) }
    $outgoing = @{}
    foreach ($edge in @($Edges)) {
        $from = [string]$edge.from
        $to = [string]$edge.to
        if ([string]::IsNullOrWhiteSpace($from) -or [string]::IsNullOrWhiteSpace($to)) { continue }
        if (-not $outgoing.ContainsKey($from)) { $outgoing[$from] = [System.Collections.Generic.List[string]]::new() }
        $outgoing[$from].Add($to)
    }
    foreach ($start in $FromIds) {
        $seen = [System.Collections.Generic.HashSet[string]]::new()
        $queue = [System.Collections.Generic.Queue[string]]::new()
        $queue.Enqueue($start)
        while ($queue.Count -gt 0) {
            $current = $queue.Dequeue()
            if (-not $seen.Add($current)) { continue }
            if ($targetSet.Contains($current) -and $current -ne $start) { return $true }
            if (-not $outgoing.ContainsKey($current)) { continue }
            foreach ($next in $outgoing[$current]) { $queue.Enqueue($next) }
        }
    }
    return $false
}

function Get-NodeFirstWorkAt($Node) {
    foreach ($event in @($Node.events)) {
        if ([string]$event.to -in @("running", "completed") -and -not [string]::IsNullOrWhiteSpace([string]$event.at)) {
            return ConvertFrom-TaskspaceEventTime $event.at
        }
    }
    foreach ($result in @($Node.results)) {
        if (-not [string]::IsNullOrWhiteSpace([string]$result.at)) {
            return ConvertFrom-TaskspaceEventTime $result.at
        }
    }
    return $null
}
