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
    elseif ($schema -ne "TaskSpaceMapExportR8V1") {
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
        if ($canonicalSchema -ne "taskspace-canonical-map-v4") {
            $status = "error"
            $errorCode = "invalid_canonical_map"
            $errorMessage = "Unsupported canonical Map schema '$canonicalSchema'."
        }
    }
    $availability = if ($status -eq "ok") { "measured" } else { "map_store_failed" }
    [pscustomobject]@{
        schema_version = "taskspace-observer-map-store-source-r8-v1"
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
        canonical_revision = [uint64](Get-ActionMapStoreProperty $canonicalMap "revision" 0)
        binding_thread_id = [string](Get-ActionMapStoreProperty $binding "thread_id" "")
        binding_relation = [string](Get-ActionMapStoreProperty $binding "relation" "")
        parent_thread_id = [string](Get-ActionMapStoreProperty $binding "parent_thread_id" "")
        canonical_map = $canonicalMap
    }
}

function Add-TaskSpaceCanonicalNode {
    param(
        [hashtable]$Nodes,
        [object]$CanonicalMap,
        [object]$CanonicalNode,
        [string]$Role
    )
    $nodeId = [string]$CanonicalNode.node_id
    $Nodes[$nodeId] = [ordered]@{
        id = $nodeId
        title = [string]$CanonicalNode.goal
        goal = [string]$CanonicalNode.goal
        role = $Role
        status = [string]$CanonicalNode.state
        content = [string]$CanonicalNode.content
        mapId = [string]$CanonicalMap.map_id
        taskId = [string]$CanonicalMap.map_id
        parents = @($CanonicalNode.parents | ForEach-Object { [string]$_ })
        children = @()
        actions = @($CanonicalNode.actions | ForEach-Object {
            [ordered]@{
                actionId = [string]$_.action_id
                toolName = [string]$_.tool_name
                outcome = [string]$_.outcome
            }
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
    $complete = [string]$CanonicalMap.root.state -eq "completed" -and
        [string]$CanonicalMap.finish.state -eq "completed"
    $edges = New-Object System.Collections.Generic.List[object]
    foreach ($node in @($nodes.Values)) {
        foreach ($parent in @($node.parents)) {
            $edges.Add([ordered]@{ mapId = $mapId; from = [string]$parent; to = [string]$node.id })
            if ($nodes.ContainsKey([string]$parent)) {
                $nodes[[string]$parent].children += [string]$node.id
            }
        }
    }
    $sortedNodes = [System.Collections.ArrayList]::new()
    foreach ($nodeId in @($nodes.Keys | Sort-Object)) {
        [void]$sortedNodes.Add([pscustomobject]$nodes[[string]$nodeId])
    }
    $map = [ordered]@{
        id = $mapId
        taskId = $mapId
        title = [string]$CanonicalMap.root.goal
        ownerSessionId = $OwnerThreadId
        rootNodeId = [string]$CanonicalMap.root.node_id
        finishNodeId = [string]$CanonicalMap.finish.node_id
        revision = [uint64]$CanonicalMap.revision
        complete = $complete
    }
    $task = [ordered]@{
        id = $mapId
        title = [string]$CanonicalMap.root.goal
        objective = [string]$CanonicalMap.root.goal
        status = if ($complete) { "completed" } else { "active" }
        ownerSessionId = $OwnerThreadId
        activeMapId = $mapId
        mapIds = @($mapId)
        cognitiveState = $null
        events = @()
    }
    [pscustomobject]@{
        tasks = @($task)
        maps = @($map)
        nodes = $sortedNodes.ToArray()
        edges = $edges.ToArray()
        sentinelWarnings = @()
        agents = @()
    }
}
