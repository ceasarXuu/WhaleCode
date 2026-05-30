function Get-TaskspaceGraphHealth($Obs) {
    $empty = [pscustomobject]@{
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
    }
    if (-not $Obs) { return $empty }

    $nodes = @($Obs.nodes)
    $edges = @($Obs.edges)
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

    [pscustomobject]@{
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
    }
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

function Get-NodeCompletedAt($Node) {
    foreach ($event in @($Node.events)) {
        if ([string]$event.to -eq "completed" -and -not [string]::IsNullOrWhiteSpace([string]$event.at)) {
            return [datetime]::Parse([string]$event.at)
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
            return [datetime]::Parse([string]$event.at)
        }
    }
    foreach ($result in @($Node.results)) {
        if (-not [string]::IsNullOrWhiteSpace([string]$result.at)) {
            return [datetime]::Parse([string]$result.at)
        }
    }
    return $null
}
