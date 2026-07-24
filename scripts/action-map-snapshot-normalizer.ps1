function Test-ActionMapSnapshotSingleMapShape {
    param([object]$Snapshot)
    $null -ne $Snapshot -and $Snapshot.PSObject.Properties.Name -contains "map"
}

function Get-ActionMapSnapshotTasks {
    param([object]$Snapshot)
    if (-not $Snapshot) { return @() }
    if (-not (Test-ActionMapSnapshotSingleMapShape $Snapshot)) {
        if ($Snapshot.PSObject.Properties.Name -contains "tasks") {
            return @($Snapshot.tasks)
        }
        return @()
    }
    $map = $Snapshot.map
    if (-not $map) { return @() }
    $root = @($map.nodes | Where-Object { [string]$_.role -eq "task_root" } | Select-Object -First 1)
    $objective = if ($root.Count -gt 0) { [string]$root[0].goal } else { "" }
    @([pscustomobject]@{
            id = [string]$map.taskId
            title = $objective
            objective = $objective
            status = if ([bool]$map.complete) { "completed" } else { "active" }
            ownerSessionId = [string]$map.ownerSessionId
            activeMapId = [string]$map.id
            mapIds = @([string]$map.id)
            cognitiveState = $null
        })
}

function Convert-ActionMapSnapshotNode {
    param([object]$Node, [string]$MapId, [string]$TaskId)
    [pscustomobject]@{
        id = [string]$Node.id
        title = [string]$Node.goal
        goal = [string]$Node.goal
        kind = [string]$Node.role
        role = [string]$Node.role
        status = [string]$Node.status
        mapId = $MapId
        taskId = $TaskId
        sourceRefs = @($Node.sourceRefs)
        activeLease = $Node.activeLease
        resultIds = @($Node.resultIds)
        nodeEventIds = @($Node.nodeEventIds)
    }
}

function Convert-ActionMapSnapshotMap {
    param([object]$Map)
    if (-not $Map) { return $null }
    $mapId = [string]$Map.id
    $taskId = [string]$Map.taskId
    $root = @($Map.nodes | Where-Object { [string]$_.role -eq "task_root" } | Select-Object -First 1)
    [pscustomobject]@{
        id = $mapId
        taskId = $taskId
        title = if ($root.Count -gt 0) { [string]$root[0].goal } else { "" }
        ownerSessionId = [string]$Map.ownerSessionId
        createdFrom = $null
        rootNodeId = [string]$Map.rootNodeId
        finishNodeId = [string]$Map.finishNodeId
        revision = [int64]$Map.revision
        currentNodeId = [string]$Map.currentNodeId
        complete = [bool]$Map.complete
        terminalSummaryRef = [string]$Map.terminalSummaryRef
        nodes = @($Map.nodes | ForEach-Object { Convert-ActionMapSnapshotNode $_ $mapId $taskId })
        edges = @($Map.edges)
        leases = @($Map.leases)
        results = @($Map.results)
        nodeEvents = @($Map.nodeEvents)
        subagentPlans = @()
    }
}

function Get-ActionMapSnapshotMaps {
    param([object]$Snapshot)
    if (-not $Snapshot) { return @() }
    if (Test-ActionMapSnapshotSingleMapShape $Snapshot) {
        $map = Convert-ActionMapSnapshotMap $Snapshot.map
        if ($map) { return @($map) }
        return @()
    }
    if ($Snapshot.PSObject.Properties.Name -contains "maps") {
        return @($Snapshot.maps)
    }
    @()
}
