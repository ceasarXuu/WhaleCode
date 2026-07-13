param(
    [Parameter(Mandatory = $true)]
    [string]$RolloutPath,
    [Parameter(Mandatory = $true)]
    [string]$JsonlPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputDir,
    [string]$ArtifactRoot = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-observability-report-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-observability-summary-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-jsonl-lib.ps1")

$output = New-Item -ItemType Directory -Force -Path $OutputDir
$rolloutReadStats = New-JsonLineReadStats $RolloutPath
$jsonlReadStats = New-JsonLineReadStats $JsonlPath
$exportPolicy = Get-ActionMapObservabilityExportPolicy $RolloutPath
$exportPolicyPath = Join-Path $output.FullName "action-map-observability-policy.json"
($exportPolicy | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $exportPolicyPath -Encoding UTF8
if ([string]$exportPolicy.rollout_export_mode -eq "summary_only_large_rollout") {
    $reduced = New-ActionMapLargeRolloutSummary -RolloutPath $RolloutPath -JsonlPath $JsonlPath -Policy $exportPolicy -RolloutReadStats $rolloutReadStats -JsonlReadStats $jsonlReadStats -ArtifactRoot $ArtifactRoot
    $reduced.source["exportPolicyPath"] = $exportPolicyPath
    $reportPaths = Write-ActionMapObservabilityReport -Reduced $reduced -OutputDir $output.FullName
    Write-Host "ObservabilityJson: $($reportPaths.Json)"
    Write-Host "ObservabilityMarkdown: $($reportPaths.Markdown)"
    Write-Host "ObservabilityHtml: $($reportPaths.Html)"
    return
}
$rolloutItems = Read-JsonLines $RolloutPath $rolloutReadStats
$jsonlItems = Read-JsonLines $JsonlPath $jsonlReadStats

$timeline = New-Object System.Collections.Generic.List[object]
$tasks = New-Object System.Collections.Generic.List[object]
$taskById = @{}
$maps = New-Object System.Collections.Generic.List[object]
$mapById = @{}
$nodes = @{}
$agents = @{}
$edges = New-Object System.Collections.Generic.List[object]
$edgeKeys = @{}
$sentinelWarnings = New-Object System.Collections.Generic.List[object]
$sentinelById = @{}
$toolCalls = New-Object System.Collections.Generic.List[object]
$toolCallById = @{}
$collabToolNames = @("spawn_agent", "wait_agent", "close_agent", "resume_agent")

function Get-SnapshotSentinelClearAction {
    param([object]$Warning, [object]$Snapshot)
    $direct = Get-SentinelClearAction $Warning
    if (-not [string]::IsNullOrWhiteSpace($direct)) { return $direct }
    if ([string](Get-ObjectField $Warning "status") -ne "cleared") { return "" }
    if ([string](Get-ObjectField $Warning "sentinelType") -ne "validator_failure") { return "" }
    $clearedAtMs = [string](Get-ObjectField $Warning "clearedAtMs")
    if ([string]::IsNullOrWhiteSpace($clearedAtMs)) { return "" }
    $traceIds = @(Get-ObjectArray (Get-ObjectField $Warning "traceEventIds") | ForEach-Object { [string]$_ })
    foreach ($trace in @(Get-ObjectArray (Get-ObjectField $Snapshot "traceEvents"))) {
        if ($traceIds -notcontains [string](Get-ObjectField $trace "id")) { continue }
        if ([string](Get-ObjectField $trace "createdAtMs") -ne $clearedAtMs) { continue }
        if ([string](Get-ObjectField $trace "taskId") -ne [string](Get-ObjectField $Warning "taskId")) { continue }
        if ([string](Get-ObjectField $trace "mapId") -ne [string](Get-ObjectField $Warning "mapId")) { continue }
        if (@(Get-ObjectArray (Get-ObjectField $trace "tags")) -contains "validator_success") {
            return "FixApplied"
        }
    }
    return ""
}

foreach ($item in $rolloutItems) {
    $payload = $item.payload
    $kind = Get-ActionMapRolloutPayloadType $payload
    if (-not $payload -or $kind -notin @(
            "mode_changed",
            "task_created",
            "task_status_changed",
            "task_routed",
            "map_created",
            "node_status_changed",
            "lease_created",
            "lease_attached",
            "node_result_recorded",
            "taskspace_trace_event_recorded",
            "sentinel_warning_raised",
            "sentinel_warning_cleared",
            "cognitive_state_updated",
            "result_validity_changed",
            "tool_action_blocked",
            "lease_released",
            "timeout_summary_requested",
            "maintenance_barrier_raised",
            "maintenance_barrier_cleared",
            "snapshot_updated"
        )) {
        continue
    }

    $at = [string]$item.timestamp
    switch ($kind) {
        "mode_changed" {
            Add-TimelineEvent $timeline $at $kind "mode changed: $($payload.previousMode) -> $($payload.currentMode)" $payload
        }
        "task_created" {
            [void](Ensure-Task $tasks $taskById ([string]$payload.taskId) ([string]$payload.title) ([string]$payload.objective) "active" ([string]$payload.ownerSessionId) ([string]$payload.activeMapId))
            Add-TimelineEvent $timeline $at $kind "task created: $($payload.taskId) $($payload.title)" $payload
        }
        "task_status_changed" {
            $task = Ensure-Task $tasks $taskById ([string]$payload.taskId)
            if ($task) {
                $task.status = [string]$payload.currentStatus
                $task.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    from = [string]$payload.previousStatus
                    to = [string]$payload.currentStatus
                })
            }
            Add-TimelineEvent $timeline $at $kind "task status: $($payload.taskId) $($payload.previousStatus) -> $($payload.currentStatus)" $payload
        }
        "task_routed" {
            Add-TimelineEvent $timeline $at $kind "task routed: $($payload.previousTaskId) -> $($payload.currentTaskId)" $payload
        }
        "map_created" {
            [void](Ensure-Map $maps $mapById ([string]$payload.mapId) ([string]$payload.title) ([string]$payload.ownerSessionId) $payload.createdFrom ([string]$payload.taskId))
            Add-TimelineEvent $timeline $at $kind "map created: $($payload.mapId) $($payload.title)" $payload
        }
        "node_status_changed" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId) ([string]$payload.nodeTitle)
            if ($node) {
                $node.status = [string]$payload.currentStatus
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    from = [string]$payload.previousStatus
                    to = [string]$payload.currentStatus
                })
            }
            Add-TimelineEvent $timeline $at $kind "node status: $($payload.nodeId) $($payload.previousStatus) -> $($payload.currentStatus)" $payload
        }
        "lease_created" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "created"
            }
            Add-TimelineEvent $timeline $at $kind "lease created: $($payload.leaseId) on $($payload.nodeId)" $payload
        }
        "lease_attached" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node -and $payload.agentThreadId) {
                $agentId = [string]$payload.agentThreadId
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "attached" "" $agentId
                if (-not $node.agentThreads.Contains($agentId)) {
                    $node.agentThreads.Add($agentId)
                }
                $agents[$agentId] = [ordered]@{
                    threadId = $agentId
                    path = [string]$payload.agentPath
                    nodeId = [string]$payload.nodeId
                    leaseId = [string]$payload.leaseId
                }
            }
            Add-TimelineEvent $timeline $at $kind "agent attached: $($payload.agentPath) -> $($payload.nodeId)" $payload
        }
        "node_result_recorded" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                $mapTaskId = ""
                $mapIdForResult = [string]$payload.mapId
                if ($mapIdForResult -and $mapById.ContainsKey($mapIdForResult)) {
                    $mapTaskId = [string]$mapById[$mapIdForResult].taskId
                }
                Add-Or-Update-NodeResult $node $at ([string]$payload.resultId) ([string]$payload.leaseId) ([string]$payload.sourceThreadId) ([string]$payload.kind) ([string]$payload.actionClass) "" $null $mapIdForResult $mapTaskId
            }
            $actionClassSuffix = if ($payload.actionClass) { " action=$($payload.actionClass)" } else { "" }
            Add-TimelineEvent $timeline $at $kind "node result recorded: $($payload.nodeId) / $($payload.resultId)$actionClassSuffix" $payload
        }
        "taskspace_trace_event_recorded" {
            Add-TimelineEvent $timeline $at $kind "trace event: $($payload.traceEventId) result=$($payload.resultId) tags=$(@($payload.tags) -join ',')" $payload
        }
        "sentinel_warning_raised" {
            [void](Add-Or-Update-SentinelWarning $sentinelWarnings $sentinelById $at ([string]$payload.sentinelId) ([string]$payload.sentinelType) ([string]$payload.status) ([string]$payload.severity) ([string]$payload.taskId) ([string]$payload.mapId) ([string]$payload.nodeId) ([string]$payload.resultId) $payload.traceEventIds ([string]$payload.reason) ([string]$payload.clearanceAction) ([string]$payload.createdAtMs) ([string]$payload.clearedAtMs))
            Add-TimelineEvent $timeline $at $kind "sentinel warning: $($payload.sentinelType) on $($payload.nodeId)" $payload
        }
        "sentinel_warning_cleared" {
            $clearAction = Get-SentinelClearAction $payload
            [void](Add-Or-Update-SentinelWarning $sentinelWarnings $sentinelById $at ([string]$payload.sentinelId) "" "cleared" "" "" "" "" "" @() "" "" "" ([string]$payload.clearedAtMs) ([string]$payload.clearedBy) $payload.clearEventIds $clearAction)
            Add-TimelineEvent $timeline $at $kind "sentinel warning cleared: $($payload.sentinelId) action=$clearAction" $payload
        }
        "cognitive_state_updated" {
            $task = Ensure-Task $tasks $taskById ([string]$payload.taskId)
            if ($task) {
                $task.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    mapId = [string]$payload.mapId
                    updateKind = [string]$payload.updateKind
                    recordId = [string]$payload.recordId
                })
            }
            Add-TimelineEvent $timeline $at $kind "cognitive state updated: $($payload.updateKind) $($payload.recordId)" $payload
        }
        "result_validity_changed" {
            Add-TimelineEvent $timeline $at $kind "result validity: $($payload.resultId) -> $($payload.validity)" $payload
        }
        "tool_action_blocked" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId) "" ([string]$payload.nodeKind)
            if ($node) {
                $node.blockedActions.Add([ordered]@{
                    at = $at
                    toolName = [string]$payload.toolName
                    actionClass = [string]$payload.actionClass
                    reason = [string]$payload.reason
                })
            }
            Add-TimelineEvent $timeline $at $kind "tool action blocked: $($payload.nodeId) $($payload.actionClass) via $($payload.toolName)" $payload
        }
        "lease_released" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "released" ([string]$payload.reason)
            }
            Add-TimelineEvent $timeline $at $kind "lease released: $($payload.leaseId), reason=$($payload.reason)" $payload
        }
        "timeout_summary_requested" {
            Add-TimelineEvent $timeline $at $kind "timeout summary requested: $($payload.agentPath)" $payload
        }
        "maintenance_barrier_raised" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$payload.mapId) ([string]$payload.reason) ([int]$payload.resultCount) ([int]$payload.budget) "active"
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    reason = [string]$payload.reason
                    resultCount = [int]$payload.resultCount
                    budget = [int]$payload.budget
                })
            }
            Add-TimelineEvent $timeline $at $kind "maintenance barrier raised: $($payload.nodeId) $($payload.resultCount)/$($payload.budget)" $payload
        }
        "maintenance_barrier_cleared" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$payload.mapId) ([string]$payload.reason) -1 -1 "cleared"
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    reason = [string]$payload.reason
                })
            }
            Add-TimelineEvent $timeline $at $kind "maintenance barrier cleared: $($payload.nodeId), reason=$($payload.reason)" $payload
        }
        "snapshot_updated" {
            $snapshotMapCount = 0
            $snapshotNodeCount = 0
            foreach ($snapshotTask in @($payload.snapshot.tasks)) {
                $task = Ensure-Task $tasks $taskById ([string]$snapshotTask.id) ([string]$snapshotTask.title) ([string]$snapshotTask.objective) ([string]$snapshotTask.status) ([string]$snapshotTask.ownerSessionId) ([string]$snapshotTask.activeMapId) $snapshotTask.mapIds $snapshotTask.cognitiveState
                if ($task) { $task.activeMapId = [string]$snapshotTask.activeMapId }
            }
            foreach ($snapshotMap in @($payload.snapshot.maps)) {
                $snapshotMapCount++
                $map = Ensure-Map $maps $mapById ([string]$snapshotMap.id) ([string]$snapshotMap.title) ([string]$snapshotMap.ownerSessionId) $snapshotMap.createdFrom ([string]$snapshotMap.taskId)
                foreach ($snapshotPlan in @($snapshotMap.subagentPlans)) {
                    Add-Or-Update-SubagentPlan $map $snapshotPlan
                }
                foreach ($snapshotEdge in @($snapshotMap.edges)) {
                    $from = [string]$snapshotEdge.from
                    $to = [string]$snapshotEdge.to
                    $mapId = [string]$snapshotMap.id
                    $edgeKey = "$mapId|$from|$to"
                    if ($from -and $to -and -not $edgeKeys.ContainsKey($edgeKey)) {
                        $edgeKeys[$edgeKey] = $true
                        $edges.Add([ordered]@{
                            mapId = $mapId
                            from = $from
                            to = $to
                        })
                    }
                }
                foreach ($snapshotNode in @($snapshotMap.nodes)) {
                    $snapshotNodeCount++
                    $node = Ensure-Node $nodes ([string]$snapshotNode.id) ([string]$snapshotNode.title) ([string]$snapshotNode.kind)
                    if ($node) {
                        if ($snapshotNode.status) { $node.status = [string]$snapshotNode.status }
                    }
                }
                foreach ($snapshotResult in @($snapshotMap.results)) {
                    $node = Ensure-Node $nodes ([string]$snapshotResult.nodeId)
                    Add-Or-Update-NodeResult $node $at ([string]$snapshotResult.id) ([string]$snapshotResult.assignmentId) ([string]$snapshotResult.sourceThreadId) ([string]$snapshotResult.kind) ([string]$snapshotResult.actionClass) ([string]$snapshotResult.sourceEventRef) $null ([string]$snapshotMap.id) ([string]$snapshotMap.taskId) "" $snapshotResult.artifactRefs $snapshotResult.toolSuccess
                }
            }
            foreach ($snapshotBarrier in @($payload.snapshot.maintenanceBarriers)) {
                $node = Ensure-Node $nodes ([string]$snapshotBarrier.nodeId)
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$snapshotBarrier.mapId) ([string]$snapshotBarrier.reason) ([int]$snapshotBarrier.resultCount) ([int]$snapshotBarrier.budget) "active"
            }
            foreach ($snapshotWarning in @($payload.snapshot.sentinelWarnings)) {
                [void](Add-Or-Update-SentinelWarning $sentinelWarnings $sentinelById $at ([string]$snapshotWarning.id) ([string]$snapshotWarning.sentinelType) ([string]$snapshotWarning.status) ([string]$snapshotWarning.severity) ([string]$snapshotWarning.taskId) ([string]$snapshotWarning.mapId) ([string]$snapshotWarning.nodeId) ([string]$snapshotWarning.resultId) $snapshotWarning.traceEventIds ([string]$snapshotWarning.reason) ([string]$snapshotWarning.clearanceAction) ([string]$snapshotWarning.createdAtMs) ([string]$snapshotWarning.clearedAtMs) "" @() (Get-SnapshotSentinelClearAction $snapshotWarning $payload.snapshot))
            }
            Add-TimelineEvent $timeline $at $kind "snapshot updated: maps=$snapshotMapCount nodes=$snapshotNodeCount" $payload
        }
    }
}

foreach ($item in $rolloutItems) {
    if ($item.type -ne "response_item" -or -not $item.payload) {
        continue
    }
    $payload = $item.payload
    $at = [string]$item.timestamp
    if ($payload.type -eq "function_call") {
        $tool = [string]$payload.name
        if ($collabToolNames -notcontains $tool) {
            continue
        }
        $callId = [string]$payload.call_id
        $promptPreview = ""
        $receivers = @()
        try {
            $args = [string]$payload.arguments | ConvertFrom-Json
            if ($args.message) { $promptPreview = [string]$args.message }
            elseif ($args.prompt) { $promptPreview = [string]$args.prompt }
            if ($args.targets) { $receivers = @($args.targets | ForEach-Object { [string]$_ }) }
            elseif ($args.target) { $receivers = @([string]$args.target) }
        } catch {
            $promptPreview = [string]$payload.arguments
        }
        if ($promptPreview.Length -gt 600) {
            $promptPreview = $promptPreview.Substring(0, 600)
        }
        [void](Add-Or-Update-ToolCall $toolCalls $toolCallById $at $callId $tool "in_progress" "" $receivers $promptPreview "")
        Add-TimelineEvent $timeline $at "tool:$tool" "tool call: $tool (in_progress)" $payload
    }
    elseif ($payload.type -eq "function_call_output") {
        $callId = [string]$payload.call_id
        if (-not $toolCallById.ContainsKey($callId)) {
            continue
        }
        $existingTool = [string]$toolCallById[$callId].tool
        $toolOutput = [string]$payload.output
        $receivers = @()
        $status = "completed"
        $structuredSuccess = $false
        try {
            $parsedOutput = $toolOutput | ConvertFrom-Json
            if ($parsedOutput.agent_id) {
                $receivers = @([string]$parsedOutput.agent_id)
                $structuredSuccess = $true
            }
            elseif ($parsedOutput.task_name) {
                $receivers = @([string]$parsedOutput.task_name)
                $structuredSuccess = $true
            }
            elseif ($parsedOutput.status) {
                $receivers = @($parsedOutput.status.PSObject.Properties.Name | ForEach-Object { [string]$_ })
                $structuredSuccess = $true
            }
            if ($parsedOutput.timed_out -eq $true) {
                $status = "timed_out"
            }
        } catch {
            if ($toolOutput -match "(?i)\b(error|failed|not found|TaskSpace mode has multiple ready nodes|Call spawn_agent with|blocked this tool call)\b") {
                $status = "failed"
            }
        }
        if ($existingTool -eq "spawn_agent" -and -not $structuredSuccess) {
            $status = "failed"
        }
        $preview = $toolOutput
        if ($preview.Length -gt 600) {
            $preview = $preview.Substring(0, 600)
        }
        $updated = Add-Or-Update-ToolCall $toolCalls $toolCallById $at $callId "" $status "" $receivers "" $preview
        if ($updated) {
            Add-TimelineEvent $timeline $at "tool:$($updated.tool)" "tool call: $($updated.tool) ($status)" $payload
        }
    }
}

foreach ($item in $jsonlItems) {
    $eventItem = $item.item
    if (-not $eventItem -or $eventItem.type -ne "collab_tool_call") {
        continue
    }
    $tool = [string]$eventItem.tool
    $status = [string]$eventItem.status
    $receivers = @()
    if ($eventItem.receiver_thread_ids) {
        $receivers = @($eventItem.receiver_thread_ids | ForEach-Object { [string]$_ })
    }
    $toolCall = [ordered]@{
        at = ""
        id = [string]$eventItem.id
        tool = $tool
        status = $status
        senderThreadId = [string]$eventItem.sender_thread_id
        receiverThreadIds = $receivers
        promptPreview = if ($eventItem.prompt) { ([string]$eventItem.prompt).Substring(0, [Math]::Min(600, ([string]$eventItem.prompt).Length)) } else { "" }
        outputPreview = ""
    }
    if ($status -in @("completed", "in_progress") -and (Has-TimestampedToolCallWithStatus $toolCalls $tool $status)) {
        continue
    }
    if ($status -eq "in_progress" -and (Has-TimestampedToolCall $toolCalls $tool)) {
        continue
    }
    if (Has-TimestampedToolCallDuplicate $toolCalls $toolCall) {
        continue
    }
    $isNewToolCall = -not $toolCallById.ContainsKey([string]$eventItem.id)
    [void](Add-Or-Update-ToolCall $toolCalls $toolCallById "" ([string]$eventItem.id) $tool $status ([string]$eventItem.sender_thread_id) $receivers $toolCall.promptPreview "")
    if ($isNewToolCall) {
        Add-TimelineEvent $timeline "" "tool:$tool" "tool call: $tool ($status)" $toolCall
    }
}

$nodeList = @($nodes.Values | Sort-Object id)
$taskList = @($tasks.ToArray() | Sort-Object id)
$agentList = @($agents.Values | Sort-Object threadId)
$blockedToolActionCount = 0
$subagentPlanCount = 0
foreach ($map in @($maps.ToArray())) {
    $plans = $map["subagentPlans"]
    if ($null -ne $plans) {
        $subagentPlanCount += [int]$plans.Count
    }
}
foreach ($node in $nodeList) {
    $blockedActions = $node["blockedActions"]
    if ($null -ne $blockedActions) {
        $blockedToolActionCount += [int]$blockedActions.Count
    }
}
$cognitiveAudit = Get-CognitiveAuditSummary $taskList $nodeList @($sentinelWarnings.ToArray()) @($timeline.ToArray()) $ArtifactRoot
$summary = [ordered]@{
    tasks = $taskList.Count
    maps = $maps.Count
    subagentPlans = $subagentPlanCount
    nodes = $nodeList.Count
    edges = $edges.Count
    agents = $agentList.Count
    toolCalls = $toolCalls.Count
    blockedToolActions = $blockedToolActionCount
    activeMaintenanceBarriers = @($nodeList | ForEach-Object {
            @($_.maintenanceBarriers | Where-Object { $_.state -eq "active" })
        }).Count
    mapRuntimeEvents = @($timeline | Where-Object { $_.kind -notlike "tool:*" }).Count
    outputContracts = [int]$cognitiveAudit.metrics.outputContractCount
    factSources = [int]$cognitiveAudit.metrics.factSourceCount
    acceptedResults = [int]$cognitiveAudit.metrics.acceptedResultCount
    questionedOrInvalidResults = [int]$cognitiveAudit.metrics.questionedOrInvalidResultCount
    cognitiveStructuralGatePassed = [bool]$cognitiveAudit.structuralGatePassed
    cognitiveAuditHardGatePassed = [bool]$cognitiveAudit.hardGatePassed
    inputParseErrors = ([int]$rolloutReadStats.parseErrorCount + [int]$jsonlReadStats.parseErrorCount)
    finalArtifacts = [int]$cognitiveAudit.metrics.finalArtifactCount
}

$reduced = [ordered]@{
    generatedAt = (Get-Date).ToString("o")
    source = [ordered]@{
        rolloutPath = (Resolve-Path -LiteralPath $RolloutPath).Path
        jsonlPath = (Resolve-Path -LiteralPath $JsonlPath).Path
        rolloutReadStats = $rolloutReadStats
        jsonlReadStats = $jsonlReadStats
        artifactRoot = $ArtifactRoot
        exportPolicy = $exportPolicy
        exportPolicyPath = $exportPolicyPath
    }
    summary = $summary
    tasks = $taskList
    maps = @($maps.ToArray())
    nodes = $nodeList
    edges = @($edges.ToArray())
    sentinelWarnings = @($sentinelWarnings.ToArray())
    cognitiveAudit = $cognitiveAudit
    finalArtifacts = @($cognitiveAudit.finalArtifacts)
    agents = $agentList
    toolCalls = @($toolCalls.ToArray())
    timeline = @($timeline.ToArray())
}

$reportPaths = Write-ActionMapObservabilityReport -Reduced $reduced -OutputDir $output.FullName

Write-Host "ObservabilityJson: $($reportPaths.Json)"
Write-Host "ObservabilityMarkdown: $($reportPaths.Markdown)"
Write-Host "ObservabilityHtml: $($reportPaths.Html)"
