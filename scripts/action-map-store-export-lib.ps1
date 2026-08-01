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

function Get-TaskSpaceRecordValues {
    param([object]$Records)
    if ($null -eq $Records) { return @() }
    if ($Records -is [System.Collections.IDictionary]) { return @($Records.Values) }
    @($Records.PSObject.Properties | ForEach-Object { $_.Value })
}

function Test-TaskSpaceRecord {
    param([object]$Records, [string]$Id)
    if ($null -eq $Records) { return $false }
    if ($Records -is [System.Collections.IDictionary]) { return $Records.Contains($Id) }
    $Records.PSObject.Properties.Name -contains $Id
}

function Move-PreviousActionMapStoreExport {
    param([Parameter(Mandatory = $true)][string]$ExportPath)
    if (-not (Test-Path -LiteralPath $ExportPath -PathType Leaf)) { return }
    $backup = "$ExportPath.previous-$([guid]::NewGuid().ToString('N'))"
    Move-Item -LiteralPath $ExportPath -Destination $backup
}

function Invoke-WithActionMapStoreHome {
    param(
        [string]$WhaleHome = "",
        [Parameter(Mandatory = $true)][scriptblock]$Operation
    )
    $resolvedWhaleHome = ""
    if (-not [string]::IsNullOrWhiteSpace($WhaleHome)) {
        if (-not (Test-Path -LiteralPath $WhaleHome -PathType Container)) {
            throw "Whale Home does not exist: $WhaleHome"
        }
        $resolvedWhaleHome = (Resolve-Path -LiteralPath $WhaleHome).Path
    }
    $previousWhaleHome = [Environment]::GetEnvironmentVariable("WHALE_HOME", "Process")
    $previousSqliteHome = [Environment]::GetEnvironmentVariable("CODEX_SQLITE_HOME", "Process")
    try {
        if ($resolvedWhaleHome) {
            $env:WHALE_HOME = $resolvedWhaleHome
            $env:CODEX_SQLITE_HOME = $resolvedWhaleHome
        }
        & $Operation $resolvedWhaleHome
    }
    finally {
        if ($null -eq $previousWhaleHome) { Remove-Item Env:\WHALE_HOME -ErrorAction SilentlyContinue }
        else { $env:WHALE_HOME = $previousWhaleHome }
        if ($null -eq $previousSqliteHome) { Remove-Item Env:\CODEX_SQLITE_HOME -ErrorAction SilentlyContinue }
        else { $env:CODEX_SQLITE_HOME = $previousSqliteHome }
    }
}

function Invoke-ActionMapStoreExport {
    param(
        [Parameter(Mandatory = $true)][string]$WhalePath,
        [Parameter(Mandatory = $true)][string]$ThreadId,
        [Parameter(Mandatory = $true)][string]$OutputDir,
        [string]$WhaleHome = ""
    )
    if (-not (Test-Path -LiteralPath $WhalePath -PathType Leaf)) {
        throw "Whale binary does not exist: $WhalePath"
    }
    $resolvedWhale = (Resolve-Path -LiteralPath $WhalePath).Path
    $exportPath = Join-Path $OutputDir "taskspace-map-store.json"
    $logPath = Join-Path $OutputDir "taskspace-map-store.stdout.log"
    Move-PreviousActionMapStoreExport $exportPath

    $commandResult = Invoke-WithActionMapStoreHome -WhaleHome $WhaleHome -Operation {
        param($activeWhaleHome)
        $capturedOutput = @(& $resolvedWhale debug taskspace-map --thread-id $ThreadId --output $exportPath 2>&1)
        [pscustomobject]@{
            command_output = $capturedOutput
            exit_code = [int]$LASTEXITCODE
            whale_home = $activeWhaleHome
        }
    }
    $commandOutput = @($commandResult.command_output)
    $exitCode = [int]$commandResult.exit_code
    $resolvedWhaleHome = [string]$commandResult.whale_home
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
    elseif ($schema -ne "TaskSpaceMapExportR7V2") {
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
    $canonicalMap = Get-ActionMapStoreProperty $map "canonical_map"
    if ($status -eq "ok" -and $canonicalMap) {
        $canonicalSchema = [string](Get-ActionMapStoreProperty $canonicalMap "schema_version" "")
        if ($canonicalSchema -ne "taskspace-canonical-map-v2") {
            $status = "error"
            $errorCode = "invalid_canonical_map"
            $errorMessage = "Unsupported canonical Map schema '$canonicalSchema'."
        }
    }
    $availability = if ($status -eq "ok") { "measured" } else { "map_store_failed" }
    [pscustomobject]@{
        schema_version = "taskspace-observer-map-store-source-r7-v2"
        availability = $availability
        error_code = $errorCode
        error_message = $errorMessage
        command_exit_code = $exitCode
        whale_path = $resolvedWhale
        whale_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedWhale).Hash.ToLowerInvariant()
        whale_home = $resolvedWhaleHome
        sqlite_home = $resolvedWhaleHome
        export_path = $exportPath
        command_log_path = $logPath
        map_id = [string](Get-ActionMapStoreProperty $map "map_id" "")
        owner_thread_id = [string](Get-ActionMapStoreProperty $map "owner_thread_id" "")
        canonical_sha256 = [string](Get-ActionMapStoreProperty $map "canonical_sha256" "")
        store_revision = [uint64](Get-ActionMapStoreProperty $map "store_revision" 0)
        map_revision = [uint64](Get-ActionMapStoreProperty $map "map_revision" 0)
        terminal = [bool](Get-ActionMapStoreProperty $map "terminal" $false)
        binding_thread_id = [string](Get-ActionMapStoreProperty $binding "thread_id" "")
        binding_relation = [string](Get-ActionMapStoreProperty $binding "relation" "")
        parent_thread_id = [string](Get-ActionMapStoreProperty $binding "parent_thread_id" "")
        canonical_map = $canonicalMap
    }
}

function Get-TaskSpaceCanonicalNodeState {
    param(
        [Parameter(Mandatory = $true)][object]$Map,
        [Parameter(Mandatory = $true)][string]$NodeId,
        [Parameter(Mandatory = $true)][string]$Role
    )
    if ($Map.terminal_record -and $Role -in @("task_root", "finish")) { return "completed" }
    if (Test-TaskSpaceRecord $Map.completion_records $NodeId) { return "completed" }
    if (Test-TaskSpaceRecord $Map.block_records $NodeId) { return "blocked" }
    if (@(Get-TaskSpaceRecordValues $Map.action_reservations | Where-Object {
        [string]$_.node_id -eq $NodeId
    }).Count -gt 0) {
        return "in_flight"
    }
    if ($Role -eq "task_root") { return "ready" }
    $predecessors = @($Map.edges | Where-Object { [string]$_.to -eq $NodeId } | ForEach-Object {
        [string]$_.from
    })
    if ($predecessors.Count -eq 0) { return "waiting" }
    foreach ($predecessor in $predecessors) {
        if ($predecessor -ne [string]$Map.root.node_id -and
            -not (Test-TaskSpaceRecord $Map.completion_records $predecessor)) {
            return "waiting"
        }
    }
    "ready"
}

function Add-TaskSpaceCanonicalNode {
    param(
        [hashtable]$Nodes,
        [object]$CanonicalMap,
        [object]$CanonicalNode,
        [string]$Role
    )
    $nodeId = [string]$CanonicalNode.node_id
    $node = Ensure-Node $Nodes $nodeId ([string]$CanonicalNode.goal) $Role
    $node.status = Get-TaskSpaceCanonicalNodeState $CanonicalMap $nodeId $Role
    $node.mapId = [string]$CanonicalMap.map_id
    $node.taskId = [string]$CanonicalMap.map_id
    foreach ($reservation in @(Get-TaskSpaceRecordValues $CanonicalMap.action_reservations | Where-Object {
        [string]$_.node_id -eq $nodeId
    })) {
        $node.reservations.Add([ordered]@{
            actionId = [string]$reservation.action_id
            toolName = [string]$reservation.tool_name
            responseCallIndex = [int]$reservation.response_call_index
        })
    }
    foreach ($property in @($CanonicalMap.result_refs.PSObject.Properties | Where-Object {
        [string]$_.Value.node_id -eq $nodeId
    })) {
        $result = $property.Value
        $node.results.Add([ordered]@{
            resultId = [string]$property.Name
            mapId = [string]$CanonicalMap.map_id
            taskId = [string]$CanonicalMap.map_id
            actionId = [string]$result.action_id
            reservationId = [string]$result.reservation_id
            success = -not [bool]$result.is_error
            validity = "unreviewed"
            artifactRefs = @()
        })
    }
    $block = Get-ActionMapStoreProperty $CanonicalMap.block_records $nodeId
    if ($block) {
        $node.blockedActions.Add([ordered]@{
            actionId = [string]$block.action_id
            reasonRef = [string]$block.reason_ref
        })
    }
}

function ConvertFrom-TaskSpaceCanonicalMap {
    param(
        [Parameter(Mandatory = $true)][object]$CanonicalMap,
        [Parameter(Mandatory = $true)][string]$OwnerThreadId
    )
    $nodes = @{}
    Add-TaskSpaceCanonicalNode $nodes $CanonicalMap $CanonicalMap.root "task_root"
    foreach ($workNode in @($CanonicalMap.work_nodes)) {
        Add-TaskSpaceCanonicalNode $nodes $CanonicalMap $workNode "work"
    }
    Add-TaskSpaceCanonicalNode $nodes $CanonicalMap $CanonicalMap.finish "finish"

    $mapId = [string]$CanonicalMap.map_id
    $terminal = $null -ne $CanonicalMap.terminal_record
    $terminalSummary = if ($terminal) { [string]$CanonicalMap.terminal_record.summary_ref } else { "" }
    $history = @(Get-ObjectArray $CanonicalMap.terminal_history | ForEach-Object { [string]$_.summary_ref })
    $map = [ordered]@{
        id = $mapId
        taskId = $mapId
        title = [string]$CanonicalMap.root.goal
        ownerSessionId = $OwnerThreadId
        rootNodeId = [string]$CanonicalMap.root.node_id
        finishNodeId = [string]$CanonicalMap.finish.node_id
        revision = [uint64]$CanonicalMap.revision
        complete = $terminal
        terminalSummaryRef = $terminalSummary
        terminalHistorySummaryRefs = $history
        subagentPlans = @()
    }
    $task = [ordered]@{
        id = $mapId
        title = [string]$CanonicalMap.root.goal
        objective = [string]$CanonicalMap.root.goal
        status = if ($terminal) { "completed" } else { "active" }
        ownerSessionId = $OwnerThreadId
        activeMapId = $mapId
        mapIds = @($mapId)
        cognitiveState = $null
        events = @()
    }
    [pscustomobject]@{
        tasks = @($task)
        maps = @($map)
        nodes = @($nodes.Values | Sort-Object { [string](Get-ObjectField $_ "id") })
        edges = @($CanonicalMap.edges | ForEach-Object {
            [ordered]@{ mapId = $mapId; from = [string]$_.from; to = [string]$_.to }
        })
        sentinelWarnings = @()
        agents = @()
    }
}
