. (Join-Path $PSScriptRoot "action-map-snapshot-normalizer.ps1")

function Get-ActionMapStoreProperty {
    param([object]$Value, [string]$Name, [object]$Default = $null)
    if ($null -eq $Value) { return $Default }
    if ($Value -is [System.Collections.IDictionary]) {
        if ($Value.Contains($Name)) { return $Value[$Name] }
        return $Default
    }
    if ($Value.PSObject.Properties.Name -contains $Name) { return $Value.$Name }
    return $Default
}

function Move-PreviousActionMapStoreExport {
    param([Parameter(Mandatory = $true)][string]$ExportPath)
    if (-not (Test-Path -LiteralPath $ExportPath -PathType Leaf)) { return }
    $backup = "$ExportPath.previous-$([guid]::NewGuid().ToString('N'))"
    Move-Item -LiteralPath $ExportPath -Destination $backup
}

function Invoke-ActionMapStoreExport {
    param(
        [Parameter(Mandatory = $true)][string]$WhalePath,
        [Parameter(Mandatory = $true)][string]$ThreadId,
        [Parameter(Mandatory = $true)][string]$OutputDir
    )
    if (-not (Test-Path -LiteralPath $WhalePath -PathType Leaf)) {
        throw "Whale binary does not exist: $WhalePath"
    }
    $resolvedWhale = (Resolve-Path -LiteralPath $WhalePath).Path
    $exportPath = Join-Path $OutputDir "taskspace-map-store.json"
    $logPath = Join-Path $OutputDir "taskspace-map-store.stdout.log"
    Move-PreviousActionMapStoreExport $exportPath

    $commandOutput = @(& $resolvedWhale debug taskspace-map --thread-id $ThreadId --output $exportPath 2>&1)
    $exitCode = [int]$LASTEXITCODE
    @($commandOutput | ForEach-Object { [string]$_ }) | Set-Content -LiteralPath $logPath -Encoding UTF8

    $envelope = $null
    $parseError = ""
    if (Test-Path -LiteralPath $exportPath -PathType Leaf) {
        try { $envelope = Get-Content -Raw -Encoding UTF8 -LiteralPath $exportPath | ConvertFrom-Json }
        catch { $parseError = $_.Exception.Message }
    }
    $schema = [string](Get-ActionMapStoreProperty $envelope "schema_version" "")
    $status = [string](Get-ActionMapStoreProperty $envelope "status" "")
    $errorCode = ""
    $errorMessage = ""
    if ($parseError) {
        $status = "error"
        $errorCode = "invalid_map_store_envelope"
        $errorMessage = $parseError
    }
    elseif ($exitCode -ne 0) {
        $status = "error"
        $errorCode = "map_store_export_failed"
        $errorMessage = (@($commandOutput | ForEach-Object { [string]$_ }) -join [Environment]::NewLine)
    }
    elseif ($schema -ne "TaskSpaceMapExportR7V1") {
        $status = "error"
        $errorCode = "invalid_map_store_envelope"
        $errorMessage = "Unsupported Map Store export schema '$schema'."
    }
    elseif ($status -ne "ok") {
        $status = "error"
        $errorCode = "invalid_map_store_envelope"
        $errorMessage = "Map Store command succeeded without an ok export."
    }

    $map = Get-ActionMapStoreProperty $envelope "map"
    $binding = Get-ActionMapStoreProperty $envelope "binding"
    $snapshot = Get-ActionMapStoreProperty $map "snapshot"
    if ($status -eq "ok" -and $null -eq $snapshot) {
        $status = "error"
        $errorCode = "invalid_map_store_envelope"
        $errorMessage = "Map Store export omitted the canonical snapshot."
    }
    $availability = if ($status -eq "ok") { "measured" } else { "map_store_failed" }
    [pscustomobject]@{
        schema_version = "taskspace-observer-map-store-source-r7-v1"
        availability = $availability
        error_code = $errorCode
        error_message = $errorMessage
        command_exit_code = $exitCode
        whale_path = $resolvedWhale
        whale_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedWhale).Hash.ToLowerInvariant()
        export_path = $exportPath
        command_log_path = $logPath
        map_id = [string](Get-ActionMapStoreProperty $map "map_id" "")
        owner_thread_id = [string](Get-ActionMapStoreProperty $map "owner_thread_id" "")
        snapshot_sha256 = [string](Get-ActionMapStoreProperty $map "snapshot_sha256" "")
        store_revision = [uint64](Get-ActionMapStoreProperty $map "store_revision" 0)
        graph_revision = [uint64](Get-ActionMapStoreProperty $map "graph_revision" 0)
        complete = [bool](Get-ActionMapStoreProperty $map "complete" $false)
        binding_thread_id = [string](Get-ActionMapStoreProperty $binding "thread_id" "")
        binding_relation = [string](Get-ActionMapStoreProperty $binding "relation" "")
        parent_thread_id = [string](Get-ActionMapStoreProperty $binding "parent_thread_id" "")
        node_id = [string](Get-ActionMapStoreProperty $binding "node_id" "")
        lease_id = [string](Get-ActionMapStoreProperty $binding "lease_id" "")
        snapshot = $snapshot
    }
}

function Get-ActionMapSnapshotSentinelClearAction {
    param([object]$Warning, [object]$Snapshot)
    $direct = [string](Get-ActionMapStoreProperty $Warning "clearAction" "")
    if ($direct) { return $direct }
    if ([string](Get-ActionMapStoreProperty $Warning "status" "") -ne "cleared") { return "" }
    if ([string](Get-ActionMapStoreProperty $Warning "sentinelType" "") -ne "validator_failure") { return "" }
    $clearedAtMs = [string](Get-ActionMapStoreProperty $Warning "clearedAtMs" "")
    if (-not $clearedAtMs) { return "" }
    $traceIds = @(Get-ObjectArray (Get-ActionMapStoreProperty $Warning "traceEventIds") | ForEach-Object { [string]$_ })
    foreach ($trace in @(Get-ObjectArray (Get-ActionMapStoreProperty $Snapshot "traceEvents"))) {
        if ($traceIds -notcontains [string](Get-ActionMapStoreProperty $trace "id" "")) { continue }
        if ([string](Get-ActionMapStoreProperty $trace "createdAtMs" "") -ne $clearedAtMs) { continue }
        if ([string](Get-ActionMapStoreProperty $trace "taskId" "") -ne [string](Get-ActionMapStoreProperty $Warning "taskId" "")) { continue }
        if ([string](Get-ActionMapStoreProperty $trace "mapId" "") -ne [string](Get-ActionMapStoreProperty $Warning "mapId" "")) { continue }
        if (@(Get-ObjectArray (Get-ActionMapStoreProperty $trace "tags")) -contains "validator_success") {
            return "FixApplied"
        }
    }
    ""
}

function ConvertFrom-ActionMapStoreSnapshot {
    param([Parameter(Mandatory = $true)][object]$Snapshot)
    $tasks = New-Object System.Collections.Generic.List[object]
    $taskById = @{}
    $maps = New-Object System.Collections.Generic.List[object]
    $mapById = @{}
    $nodes = @{}
    $edges = New-Object System.Collections.Generic.List[object]
    $warnings = New-Object System.Collections.Generic.List[object]
    $warningById = @{}
    $agents = @{}

    foreach ($snapshotTask in @(Get-ActionMapSnapshotTasks $Snapshot)) {
        [void](Ensure-Task $tasks $taskById ([string]$snapshotTask.id) ([string]$snapshotTask.title) ([string]$snapshotTask.objective) ([string]$snapshotTask.status) ([string]$snapshotTask.ownerSessionId) ([string]$snapshotTask.activeMapId) $snapshotTask.mapIds $snapshotTask.cognitiveState)
    }
    foreach ($snapshotMap in @(Get-ActionMapSnapshotMaps $Snapshot)) {
        $map = Ensure-Map $maps $mapById ([string]$snapshotMap.id) ([string]$snapshotMap.title) ([string]$snapshotMap.ownerSessionId) $snapshotMap.createdFrom ([string]$snapshotMap.taskId)
        foreach ($field in @("rootNodeId", "finishNodeId", "revision", "currentNodeId", "complete", "terminalSummaryRef")) {
            if ($snapshotMap.PSObject.Properties.Name -contains $field) { $map[$field] = $snapshotMap.$field }
        }
        foreach ($snapshotPlan in @($snapshotMap.subagentPlans)) { Add-Or-Update-SubagentPlan $map $snapshotPlan }
        foreach ($snapshotNode in @($snapshotMap.nodes)) {
            $goal = if ([string]$snapshotNode.goal) { [string]$snapshotNode.goal } else { [string]$snapshotNode.title }
            $role = if ([string]$snapshotNode.role) { [string]$snapshotNode.role } else { [string]$snapshotNode.kind }
            $node = Ensure-Node $nodes ([string]$snapshotNode.id) $goal $role
            $node.status = [string]$snapshotNode.status
            $node.mapId = [string]$snapshotMap.id
            $node.taskId = [string]$snapshotMap.taskId
        }
        foreach ($snapshotEdge in @($snapshotMap.edges)) {
            $edges.Add([ordered]@{ mapId = [string]$snapshotMap.id; from = [string]$snapshotEdge.from; to = [string]$snapshotEdge.to })
        }
        foreach ($snapshotLease in @($snapshotMap.leases)) {
            $node = Ensure-Node $nodes ([string]$snapshotLease.nodeId)
            $agentThreadId = [string]$snapshotLease.agentThreadId
            Add-Or-Update-Lease $node "" ([string]$snapshotLease.id) "active" "" $agentThreadId
            if ($agentThreadId) {
                if (-not $node.agentThreads.Contains($agentThreadId)) { $node.agentThreads.Add($agentThreadId) }
                $agents[$agentThreadId] = [ordered]@{ threadId = $agentThreadId; path = [string]$snapshotLease.agentPath; nodeId = [string]$snapshotLease.nodeId; leaseId = [string]$snapshotLease.id }
            }
        }
        foreach ($snapshotResult in @($snapshotMap.results)) {
            $node = Ensure-Node $nodes ([string]$snapshotResult.nodeId)
            Add-Or-Update-NodeResult $node ([string]$snapshotResult.createdAtMs) ([string]$snapshotResult.id) ([string]$snapshotResult.assignmentId) ([string]$snapshotResult.sourceThreadId) ([string]$snapshotResult.kind) ([string]$snapshotResult.actionClass) ([string]$snapshotResult.sourceEventRef) $null ([string]$snapshotMap.id) ([string]$snapshotMap.taskId) "" $snapshotResult.artifactRefs $snapshotResult.toolSuccess
        }
        foreach ($nodeEvent in @($snapshotMap.nodeEvents)) {
            if ($null -eq $nodeEvent) { continue }
            $node = Ensure-Node $nodes ([string]$nodeEvent.nodeId)
            if ($null -eq $node) { continue }
            $node.events.Add([ordered]@{
                    at = [string]$nodeEvent.createdAtMs
                    kind = [string]$nodeEvent.eventKind
                    source = [string]$nodeEvent.source
                    sourceEventId = [string]$nodeEvent.sourceEventId
                    callId = [string]$nodeEvent.callId
                    actionClass = [string]$nodeEvent.actionClass
                    toolSuccess = $nodeEvent.toolSuccess
                    artifactRefs = @($nodeEvent.artifactRefs)
                })
        }
    }
    foreach ($barrier in @($Snapshot.maintenanceBarriers)) {
        $node = Ensure-Node $nodes ([string]$barrier.nodeId)
        Add-Or-Update-MaintenanceBarrier $node "" ([string]$barrier.mapId) ([string]$barrier.reason) ([int]$barrier.resultCount) ([int]$barrier.budget) "active"
    }
    foreach ($warning in @($Snapshot.sentinelWarnings)) {
        [void](Add-Or-Update-SentinelWarning $warnings $warningById "" ([string]$warning.id) ([string]$warning.sentinelType) ([string]$warning.status) ([string]$warning.severity) ([string]$warning.taskId) ([string]$warning.mapId) ([string]$warning.nodeId) ([string]$warning.resultId) $warning.traceEventIds ([string]$warning.reason) ([string]$warning.clearanceAction) ([string]$warning.createdAtMs) ([string]$warning.clearedAtMs) "" @() (Get-ActionMapSnapshotSentinelClearAction $warning $Snapshot))
    }
    [pscustomobject]@{
        tasks = @($tasks.ToArray() | Sort-Object { [string](Get-ObjectField $_ "id") })
        maps = @($maps.ToArray() | Sort-Object { [string](Get-ObjectField $_ "id") })
        nodes = @($nodes.Values | Sort-Object { [string](Get-ObjectField $_ "id") })
        edges = @($edges.ToArray() | Sort-Object { [string](Get-ObjectField $_ "mapId") }, { [string](Get-ObjectField $_ "from") }, { [string](Get-ObjectField $_ "to") })
        sentinelWarnings = @($warnings.ToArray() | Sort-Object { [string](Get-ObjectField $_ "id") })
        agents = @($agents.Values | Sort-Object { [string](Get-ObjectField $_ "threadId") })
    }
}
