. (Join-Path $PSScriptRoot "action-map-snapshot-normalizer.ps1")

function Get-ActionMapReplayProperty {
    param([object]$Value, [string]$Name, [object]$Default = $null)
    if ($null -eq $Value) { return $Default }
    if ($Value -is [System.Collections.IDictionary]) {
        if ($Value.Contains($Name)) { return $Value[$Name] }
        return $Default
    }
    if ($Value.PSObject.Properties.Name -contains $Name) { return $Value.$Name }
    return $Default
}

function Move-PreviousActionMapReplayProof {
    param([Parameter(Mandatory = $true)][string]$ProofPath)
    if (-not (Test-Path -LiteralPath $ProofPath -PathType Leaf)) { return }
    $backup = "$ProofPath.previous-$([guid]::NewGuid().ToString('N'))"
    Move-Item -LiteralPath $ProofPath -Destination $backup
}

function Invoke-ActionMapCanonicalReplay {
    param(
        [Parameter(Mandatory = $true)][string]$WhalePath,
        [Parameter(Mandatory = $true)][string]$RolloutPath,
        [Parameter(Mandatory = $true)][string]$OutputDir
    )
    if (-not (Test-Path -LiteralPath $WhalePath -PathType Leaf)) {
        throw "Whale replay binary does not exist: $WhalePath"
    }
    $resolvedWhale = (Resolve-Path -LiteralPath $WhalePath).Path
    $resolvedRollout = (Resolve-Path -LiteralPath $RolloutPath).Path
    $proofPath = Join-Path $OutputDir "taskspace-replay-proof.json"
    $logPath = Join-Path $OutputDir "taskspace-replay.stdout.log"
    Move-PreviousActionMapReplayProof $proofPath

    $commandOutput = @(& $resolvedWhale debug taskspace-replay --rollout $resolvedRollout --output $proofPath 2>&1)
    $exitCode = [int]$LASTEXITCODE
    @($commandOutput | ForEach-Object { [string]$_ }) | Set-Content -LiteralPath $logPath -Encoding UTF8

    $envelope = $null
    $parseError = ""
    if (Test-Path -LiteralPath $proofPath -PathType Leaf) {
        try { $envelope = Get-Content -Raw -Encoding UTF8 -LiteralPath $proofPath | ConvertFrom-Json }
        catch { $parseError = $_.Exception.Message }
    }
    $schema = [string](Get-ActionMapReplayProperty $envelope "schema_version" "")
    $status = [string](Get-ActionMapReplayProperty $envelope "status" "")
    $error = Get-ActionMapReplayProperty $envelope "error"
    $errorCode = [string](Get-ActionMapReplayProperty $error "code" "")
    $errorMessage = [string](Get-ActionMapReplayProperty $error "message" "")
    if ($parseError) {
        $status = "error"
        $errorCode = "invalid_proof_envelope"
        $errorMessage = $parseError
    }
    elseif ($schema -ne "TaskSpaceReplayProofR6V1") {
        $status = "error"
        $errorCode = "invalid_proof_envelope"
        $errorMessage = "Unsupported replay proof schema '$schema'."
    }
    elseif ($exitCode -eq 0 -and $status -ne "ok") {
        $status = "error"
        $errorCode = "invalid_proof_envelope"
        $errorMessage = "Replay command succeeded without an ok proof."
    }
    elseif ($exitCode -ne 0 -and $status -ne "error") {
        $status = "error"
        $errorCode = "invalid_proof_envelope"
        $errorMessage = "Replay command failed without an error proof."
    }

    $proof = Get-ActionMapReplayProperty $envelope "proof"
    $snapshot = Get-ActionMapReplayProperty $envelope "snapshot"
    if ($status -eq "ok" -and $null -eq $snapshot) {
        $status = "error"
        $errorCode = "invalid_proof_envelope"
        $errorMessage = "Replay proof omitted the canonical snapshot."
    }
    $availability = if ($status -eq "ok") {
        "measured"
    }
    elseif ($errorCode -eq "not_applicable") {
        "not_applicable"
    }
    else {
        "replay_failed"
    }
    [pscustomobject]@{
        schema_version = "taskspace-observer-replay-source-r6-v1"
        availability = $availability
        error_code = $errorCode
        error_message = $errorMessage
        command_exit_code = $exitCode
        whale_path = $resolvedWhale
        whale_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedWhale).Hash.ToLowerInvariant()
        proof_path = $proofPath
        command_log_path = $logPath
        rollout_sha256 = [string](Get-ActionMapReplayProperty $proof "rollout_sha256" "")
        final_snapshot_sha256 = [string](Get-ActionMapReplayProperty $proof "final_snapshot_sha256" "")
        checkpoint_id = [string](Get-ActionMapReplayProperty $proof "checkpoint_id" "")
        base_snapshot_sha256 = [string](Get-ActionMapReplayProperty $proof "base_snapshot_sha256" "")
        parsed_checkpoint_count = [int](Get-ActionMapReplayProperty $proof "parsed_checkpoint_count" 0)
        parsed_delta_count = [int](Get-ActionMapReplayProperty $proof "parsed_delta_count" 0)
        surviving_checkpoint_count = [int](Get-ActionMapReplayProperty $proof "surviving_checkpoint_count" 0)
        surviving_delta_count = [int](Get-ActionMapReplayProperty $proof "surviving_delta_count" 0)
        active_checkpoint_id = [string](Get-ActionMapReplayProperty $proof "active_checkpoint_id" "")
        active_chain_applied_delta_count = [int](Get-ActionMapReplayProperty $proof "active_chain_applied_delta_count" 0)
        active_chain_last_delta_sequence = Get-ActionMapReplayProperty $proof "active_chain_last_delta_sequence"
        snapshot = $snapshot
    }
}

function Get-ActionMapSnapshotSentinelClearAction {
    param([object]$Warning, [object]$Snapshot)
    $direct = [string](Get-ActionMapReplayProperty $Warning "clearAction" "")
    if ($direct) { return $direct }
    if ([string](Get-ActionMapReplayProperty $Warning "status" "") -ne "cleared") { return "" }
    if ([string](Get-ActionMapReplayProperty $Warning "sentinelType" "") -ne "validator_failure") { return "" }
    $clearedAtMs = [string](Get-ActionMapReplayProperty $Warning "clearedAtMs" "")
    if (-not $clearedAtMs) { return "" }
    $traceIds = @(Get-ObjectArray (Get-ActionMapReplayProperty $Warning "traceEventIds") | ForEach-Object { [string]$_ })
    foreach ($trace in @(Get-ObjectArray (Get-ActionMapReplayProperty $Snapshot "traceEvents"))) {
        if ($traceIds -notcontains [string](Get-ActionMapReplayProperty $trace "id" "")) { continue }
        if ([string](Get-ActionMapReplayProperty $trace "createdAtMs" "") -ne $clearedAtMs) { continue }
        if ([string](Get-ActionMapReplayProperty $trace "taskId" "") -ne [string](Get-ActionMapReplayProperty $Warning "taskId" "")) { continue }
        if ([string](Get-ActionMapReplayProperty $trace "mapId" "") -ne [string](Get-ActionMapReplayProperty $Warning "mapId" "")) { continue }
        if (@(Get-ObjectArray (Get-ActionMapReplayProperty $trace "tags")) -contains "validator_success") {
            return "FixApplied"
        }
    }
    ""
}

function ConvertFrom-ActionMapReplaySnapshot {
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
