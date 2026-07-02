. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function Get-ActionMapObservabilityInt64Env {
    param(
        [string]$Name,
        [int64]$Default,
        [int64]$Minimum
    )
    $raw = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($raw)) { return $Default }
    try {
        $value = [int64]$raw
        if ($value -lt $Minimum) { return $Minimum }
        return $value
    } catch {
        return $Default
    }
}

function Get-ActionMapObservabilityExportPolicy {
    param([Parameter(Mandatory = $true)][string]$RolloutPath)
    $rolloutBytes = [int64]0
    if (Test-Path -LiteralPath $RolloutPath) {
        $rolloutBytes = [int64](Get-Item -LiteralPath $RolloutPath).Length
    }
    $maxBytes = Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_ROLLOUT_MAX_BYTES" ([int64](32 * 1024 * 1024)) ([int64](1024 * 1024))
    $eventMaxBytes = Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES" ([int64](1024 * 1024)) ([int64](64 * 1024))
    $timelineLimit = [int](Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_TIMELINE_SAMPLE_LIMIT" 240 20)
    [pscustomobject]@{
        schema_version = "taskspace-observability-export-policy-v1"
        rollout_export_mode = if ($rolloutBytes -gt $maxBytes) { "summary_only_large_rollout" } else { "full" }
        rollout_bytes = $rolloutBytes
        rollout_max_bytes = $maxBytes
        event_parse_max_bytes = $eventMaxBytes
        timeline_sample_limit = $timelineLimit
    }
}

function Add-ActionMapSummaryCount {
    param([hashtable]$Counts, [AllowEmptyString()][string]$Key)
    if ([string]::IsNullOrWhiteSpace($Key)) { $Key = "unknown" }
    if (-not $Counts.ContainsKey($Key)) { $Counts[$Key] = 0 }
    $Counts[$Key]++
}

function Convert-ActionMapSummaryCounts {
    param([hashtable]$Counts)
    $ordered = [ordered]@{}
    foreach ($key in @($Counts.Keys | Sort-Object)) { $ordered[$key] = [int]$Counts[$key] }
    [pscustomobject]$ordered
}

function Add-ActionMapSummaryTimeline {
    param(
        [System.Collections.Generic.List[object]]$Timeline,
        [hashtable]$State,
        [int]$Limit,
        [string]$At,
        [string]$Kind,
        [string]$Summary,
        [object]$Details
    )
    if ($Timeline.Count -lt $Limit) {
        $Timeline.Add([ordered]@{
            at = $At
            kind = $Kind
            summary = $Summary
            details = $Details
        })
        return
    }
    $State.dropped = [int]$State.dropped + 1
}

function Get-ActionMapPayloadTypeFromPrefix {
    param([string]$Line)
    $prefixLength = [Math]::Min(32768, $Line.Length)
    $prefix = $Line.Substring(0, $prefixLength)
    $match = [regex]::Match($prefix, '(?s)"payload"\s*:\s*\{.*?"type"\s*:\s*"([^"]+)"')
    if ($match.Success) { return [string]$match.Groups[1].Value }
    $top = [regex]::Match($prefix, '"type"\s*:\s*"([^"]+)"')
    if ($top.Success) { return [string]$top.Groups[1].Value }
    "unknown"
}

function Add-ActionMapSummaryReadField {
    param([object]$Stats, [string]$Name, [object]$Value)
    if ($Stats -is [System.Collections.IDictionary]) {
        if (-not $Stats.Contains($Name)) { $Stats[$Name] = $Value }
    }
}

function Read-ActionMapSummaryLine {
    param(
        [string]$Line,
        [object]$Stats,
        [int64]$MaxParseBytes
    )
    if ($Stats) {
        Add-ActionMapSummaryReadField $Stats "largeLineSkippedCount" 0
        $Stats.totalLines = [int]$Stats.totalLines + 1
    }
    if ([string]::IsNullOrWhiteSpace($Line)) {
        if ($Stats) { $Stats.skippedBlankLines = [int]$Stats.skippedBlankLines + 1 }
        return $null
    }
    if ([int64]$Line.Length -gt $MaxParseBytes) {
        if ($Stats -is [System.Collections.IDictionary]) { $Stats["largeLineSkippedCount"] = [int]$Stats["largeLineSkippedCount"] + 1 }
        return [pscustomobject]@{
            parsed = $false
            large_line_skipped = $true
            payload_type = Get-ActionMapPayloadTypeFromPrefix $Line
            line_chars = [int64]$Line.Length
            row = $null
        }
    }
    try {
        $row = $Line | ConvertFrom-Json
        if ($Stats) { $Stats.parsedLines = [int]$Stats.parsedLines + 1 }
        return [pscustomobject]@{ parsed = $true; large_line_skipped = $false; payload_type = [string]$row.payload.type; line_chars = [int64]$Line.Length; row = $row }
    } catch {
        if ($Stats) {
            $Stats.parseErrorCount = [int]$Stats.parseErrorCount + 1
            $Stats.parseErrors.Add([ordered]@{ line = [int]$Stats.totalLines; message = $_.Exception.Message })
        }
        return $null
    }
}

function New-ActionMapSummaryCognitiveAudit {
    [pscustomobject]@{
        auditSchemaVersion = "taskspace-cognitive-audit-v1"
        auditScope = "summary_only_large_rollout"
        promotionNotInMvp = $false
        hardGateFailures = @("summary_only_large_rollout")
        unsupportedMvpGateIds = @()
        gateRecords = @()
        metrics = [pscustomobject]@{
            outputContractCount = 0
            factSourceCount = 0
            acceptedResultCount = 0
            questionedOrInvalidResultCount = 0
            finalArtifactCount = 0
        }
        structuralGatePassed = $false
        hardGatePassed = $false
        fullMvpHardGateImplemented = $false
        finalArtifacts = @()
    }
}

function New-ActionMapLargeRolloutSummary {
    param(
        [Parameter(Mandatory = $true)][string]$RolloutPath,
        [Parameter(Mandatory = $true)][string]$JsonlPath,
        [Parameter(Mandatory = $true)][object]$Policy,
        [Parameter(Mandatory = $true)][object]$RolloutReadStats,
        [Parameter(Mandatory = $true)][object]$JsonlReadStats,
        [AllowEmptyString()][string]$ArtifactRoot = ""
    )
    $timeline = New-Object System.Collections.Generic.List[object]
    $timelineState = @{ dropped = 0 }
    $tasks = New-Object System.Collections.Generic.List[object]
    $taskById = @{}
    $maps = New-Object System.Collections.Generic.List[object]
    $mapById = @{}
    $nodes = @{}
    $agents = @{}
    $edges = New-Object System.Collections.Generic.List[object]
    $edgeKeys = @{}
    $toolCalls = New-Object System.Collections.Generic.List[object]
    $toolCallById = @{}
    $runtimeCounts = @{}
    $topCounts = @{}
    $largeEventCounts = @{}
    $limit = [int]$Policy.timeline_sample_limit
    $maxLine = [int64]$Policy.event_parse_max_bytes
    $collabToolNames = @("spawn_agent", "wait_agent", "close_agent", "resume_agent")

    foreach ($line in Get-Content -LiteralPath $RolloutPath -Encoding UTF8) {
        $read = Read-ActionMapSummaryLine $line $RolloutReadStats $maxLine
        if (-not $read) { continue }
        if ($read.large_line_skipped) {
            Add-ActionMapSummaryCount $largeEventCounts ([string]$read.payload_type)
            Add-ActionMapSummaryCount $runtimeCounts ([string]$read.payload_type)
            Add-ActionMapSummaryTimeline $timeline $timelineState $limit "" ([string]$read.payload_type) "large rollout event summarized without payload materialization" ([ordered]@{ large_line_skipped = $true; line_chars = $read.line_chars })
            continue
        }
        $item = $read.row
        Add-ActionMapSummaryCount $topCounts ([string]$item.type)
        $payload = $item.payload
        $at = [string]$item.timestamp
        if ($payload -and $payload.type) {
            $kind = [string]$payload.type
            Add-ActionMapSummaryCount $runtimeCounts $kind
            $traceKind = [string]$payload.kind
            if ($kind -eq "taskspace_trace_event_recorded" -and -not [string]::IsNullOrWhiteSpace($traceKind)) {
                Add-ActionMapSummaryCount $runtimeCounts $traceKind
            }
            $updateKind = [string]$payload.updateKind
            if ($updateKind -like "state_commit*") {
                Add-ActionMapSummaryCount $runtimeCounts $updateKind
            }
            switch ($kind) {
                "task_created" { [void](Ensure-Task $tasks $taskById ([string]$payload.taskId) ([string]$payload.title) ([string]$payload.objective) "active" ([string]$payload.ownerSessionId) ([string]$payload.activeMapId)) }
                "task_status_changed" { $task = Ensure-Task $tasks $taskById ([string]$payload.taskId); if ($task) { $task.status = [string]$payload.currentStatus } }
                "map_created" { [void](Ensure-Map $maps $mapById ([string]$payload.mapId) ([string]$payload.title) ([string]$payload.ownerSessionId) $payload.createdFrom ([string]$payload.taskId)) }
                "node_status_changed" { $node = Ensure-Node $nodes ([string]$payload.nodeId) ([string]$payload.nodeTitle); if ($node) { $node.status = [string]$payload.currentStatus; $node.events.Add([ordered]@{ at = $at; kind = $kind; from = [string]$payload.previousStatus; to = [string]$payload.currentStatus }) } }
                "lease_created" { $node = Ensure-Node $nodes ([string]$payload.nodeId); if ($node) { Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "created" } }
                "lease_attached" { $node = Ensure-Node $nodes ([string]$payload.nodeId); if ($node -and $payload.agentThreadId) { $agentId = [string]$payload.agentThreadId; Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "attached" "" $agentId; if (-not $node.agentThreads.Contains($agentId)) { $node.agentThreads.Add($agentId) }; $agents[$agentId] = [ordered]@{ threadId = $agentId; path = [string]$payload.agentPath; nodeId = [string]$payload.nodeId; leaseId = [string]$payload.leaseId } } }
                "node_result_recorded" { $node = Ensure-Node $nodes ([string]$payload.nodeId); Add-Or-Update-NodeResult $node $at ([string]$payload.resultId) ([string]$payload.leaseId) ([string]$payload.sourceThreadId) ([string]$payload.kind) ([string]$payload.actionClass) "" $null ([string]$payload.mapId) "" }
                "snapshot_updated" {
                    foreach ($snapshotTask in @($payload.snapshot.tasks)) { [void](Ensure-Task $tasks $taskById ([string]$snapshotTask.id) ([string]$snapshotTask.title) ([string]$snapshotTask.objective) ([string]$snapshotTask.status) ([string]$snapshotTask.ownerSessionId) ([string]$snapshotTask.activeMapId) $snapshotTask.mapIds $null) }
                    foreach ($snapshotMap in @($payload.snapshot.maps)) {
                        $map = Ensure-Map $maps $mapById ([string]$snapshotMap.id) ([string]$snapshotMap.title) ([string]$snapshotMap.ownerSessionId) $snapshotMap.createdFrom ([string]$snapshotMap.taskId)
                        foreach ($snapshotPlan in @($snapshotMap.subagentPlans)) { Add-Or-Update-SubagentPlan $map $snapshotPlan }
                        foreach ($snapshotEdge in @($snapshotMap.edges)) {
                            $from = [string]$snapshotEdge.from
                            $to = [string]$snapshotEdge.to
                            $mapId = [string]$snapshotMap.id
                            $edgeKey = "$mapId|$from|$to"
                            if ($from -and $to -and -not $edgeKeys.ContainsKey($edgeKey)) {
                                $edgeKeys[$edgeKey] = $true
                                $edges.Add([ordered]@{ mapId = $mapId; from = $from; to = $to })
                            }
                        }
                        foreach ($snapshotNode in @($snapshotMap.nodes)) { $node = Ensure-Node $nodes ([string]$snapshotNode.id) ([string]$snapshotNode.title) ([string]$snapshotNode.kind); if ($node -and $snapshotNode.status) { $node.status = [string]$snapshotNode.status } }
                        foreach ($snapshotResult in @($snapshotMap.results)) { $node = Ensure-Node $nodes ([string]$snapshotResult.nodeId); Add-Or-Update-NodeResult $node $at ([string]$snapshotResult.id) ([string]$snapshotResult.assignmentId) ([string]$snapshotResult.sourceThreadId) ([string]$snapshotResult.kind) ([string]$snapshotResult.actionClass) "" $null ([string]$snapshotMap.id) ([string]$snapshotMap.taskId) ([string]$snapshotResult.subagentPlanId) }
                    }
                }
            }
            Add-ActionMapSummaryTimeline $timeline $timelineState $limit $at $kind "$kind summarized" ([ordered]@{ type = $kind; taskId = [string]$payload.taskId; mapId = [string]$payload.mapId; nodeId = [string]$payload.nodeId; resultId = [string]$payload.resultId })
        }
        if ([string]$item.type -eq "response_item" -and $payload -and $payload.type -eq "function_call") {
            $tool = [string]$payload.name
            if ($collabToolNames -contains $tool) {
                [void](Add-Or-Update-ToolCall $toolCalls $toolCallById $at ([string]$payload.call_id) $tool "in_progress" "" @() "" "")
                Add-ActionMapSummaryTimeline $timeline $timelineState $limit $at "tool:$tool" "tool call: $tool (in_progress)" ([ordered]@{ type = "function_call"; name = $tool; call_id = [string]$payload.call_id })
            }
        }
    }

    if (Test-Path -LiteralPath $JsonlPath) {
        foreach ($line in Get-Content -LiteralPath $JsonlPath -Encoding UTF8) {
            $read = Read-ActionMapSummaryLine $line $JsonlReadStats $maxLine
            if (-not $read -or -not $read.parsed) { continue }
            $eventItem = $read.row.item
            if (-not $eventItem -or [string]$eventItem.type -ne "collab_tool_call") { continue }
            [void](Add-Or-Update-ToolCall $toolCalls $toolCallById "" ([string]$eventItem.id) ([string]$eventItem.tool) ([string]$eventItem.status) ([string]$eventItem.sender_thread_id) @($eventItem.receiver_thread_ids | ForEach-Object { [string]$_ }) "" "")
        }
    }

    $nodeList = @($nodes.Values | Sort-Object id)
    $taskList = @($tasks.ToArray() | Sort-Object id)
    $agentList = @($agents.Values | Sort-Object threadId)
    $subagentPlanCount = 0
    foreach ($map in @($maps.ToArray())) {
        if ($null -ne $map["subagentPlans"]) { $subagentPlanCount += [int]$map["subagentPlans"].Count }
    }
    $audit = New-ActionMapSummaryCognitiveAudit
    $summary = [ordered]@{
        tasks = $taskList.Count
        maps = $maps.Count
        subagentPlans = $subagentPlanCount
        nodes = $nodeList.Count
        edges = $edges.Count
        agents = $agentList.Count
        toolCalls = $toolCalls.Count
        blockedToolActions = 0
        activeMaintenanceBarriers = 0
        mapRuntimeEvents = [int](($runtimeCounts.Values | Measure-Object -Sum).Sum)
        runtimeEventCounts = Convert-ActionMapSummaryCounts $runtimeCounts
        topLevelEventCounts = Convert-ActionMapSummaryCounts $topCounts
        largeLineEventCounts = Convert-ActionMapSummaryCounts $largeEventCounts
        timelineEventsDropped = [int]$timelineState.dropped
        outputContracts = 0
        factSources = 0
        acceptedResults = 0
        questionedOrInvalidResults = 0
        cognitiveStructuralGatePassed = $false
        cognitiveAuditHardGatePassed = $false
        inputParseErrors = ([int]$RolloutReadStats.parseErrorCount + [int]$JsonlReadStats.parseErrorCount)
        finalArtifacts = 0
    }
    [ordered]@{
        generatedAt = (Get-Date).ToString("o")
        source = [ordered]@{
            rolloutPath = (Resolve-Path -LiteralPath $RolloutPath).Path
            jsonlPath = if (Test-Path -LiteralPath $JsonlPath) { (Resolve-Path -LiteralPath $JsonlPath).Path } else { $JsonlPath }
            rolloutReadStats = $RolloutReadStats
            jsonlReadStats = $JsonlReadStats
            artifactRoot = $ArtifactRoot
            exportPolicy = $Policy
        }
        summary = $summary
        tasks = $taskList
        maps = @($maps.ToArray())
        nodes = $nodeList
        edges = @($edges.ToArray())
        sentinelWarnings = @()
        cognitiveAudit = $audit
        finalArtifacts = @()
        agents = $agentList
        toolCalls = @($toolCalls.ToArray())
        timeline = @($timeline.ToArray())
    }
}
