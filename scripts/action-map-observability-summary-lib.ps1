. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function Get-ActionMapObservabilityInt64Env {
    param([string]$Name, [int64]$Default, [int64]$Minimum)
    $raw = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($raw)) { return $Default }
    try {
        $value = [int64]$raw
        if ($value -lt $Minimum) { return $Minimum }
        $value
    }
    catch { $Default }
}

function Get-ActionMapObservabilityExportPolicy {
    param([Parameter(Mandatory = $true)][string]$RolloutPath)
    $rolloutBytes = if (Test-Path -LiteralPath $RolloutPath) { [int64](Get-Item -LiteralPath $RolloutPath).Length } else { 0 }
    $maxBytes = Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_ROLLOUT_MAX_BYTES" ([int64](32 * 1024 * 1024)) ([int64](1024 * 1024))
    [pscustomobject]@{
        schema_version = "taskspace-observability-export-policy-v1"
        rollout_export_mode = if ($rolloutBytes -gt $maxBytes) { "summary_only_large_rollout" } else { "full" }
        rollout_bytes = $rolloutBytes
        rollout_max_bytes = $maxBytes
        event_parse_max_bytes = Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES" ([int64](1024 * 1024)) ([int64](64 * 1024))
        timeline_sample_limit = [int](Get-ActionMapObservabilityInt64Env "TASKSPACE_OBSERVABILITY_TIMELINE_SAMPLE_LIMIT" 240 20)
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

function Get-ActionMapRolloutPayloadType {
    param($Payload)
    if ($null -eq $Payload) { return "" }
    if ([string]$Payload.type -eq "map_runtime") { return [string]$Payload.map_event_type }
    [string]$Payload.type
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
        $Timeline.Add([ordered]@{ at = $At; kind = $Kind; summary = $Summary; details = $Details })
        return
    }
    $State.dropped = [int]$State.dropped + 1
}

function Get-ActionMapPayloadTypeFromPrefix {
    param([string]$Line)
    $prefix = $Line.Substring(0, [Math]::Min(32768, $Line.Length))
    $mapMatch = [regex]::Match($prefix, '(?s)"payload"\s*:\s*\{.*?"map_event_type"\s*:\s*"([^"]+)"')
    if ($mapMatch.Success) { return [string]$mapMatch.Groups[1].Value }
    $match = [regex]::Match($prefix, '(?s)"payload"\s*:\s*\{.*?"type"\s*:\s*"([^"]+)"')
    if ($match.Success) { return [string]$match.Groups[1].Value }
    $top = [regex]::Match($prefix, '"type"\s*:\s*"([^"]+)"')
    if ($top.Success) { return [string]$top.Groups[1].Value }
    "unknown"
}

function Add-ActionMapSummaryReadField {
    param([object]$Stats, [string]$Name, [object]$Value)
    if ($Stats -is [System.Collections.IDictionary] -and -not $Stats.Contains($Name)) { $Stats[$Name] = $Value }
}

function Read-ActionMapSummaryLine {
    param([string]$Line, [object]$Stats, [int64]$MaxParseBytes)
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
        return [pscustomobject]@{ parsed = $false; large_line_skipped = $true; payload_type = Get-ActionMapPayloadTypeFromPrefix $Line; line_chars = [int64]$Line.Length; row = $null }
    }
    try {
        $row = $Line | ConvertFrom-Json
        if ($Stats) { $Stats.parsedLines = [int]$Stats.parsedLines + 1 }
        [pscustomobject]@{ parsed = $true; large_line_skipped = $false; payload_type = Get-ActionMapRolloutPayloadType $row.payload; line_chars = [int64]$Line.Length; row = $row }
    }
    catch {
        if ($Stats) {
            $Stats.parseErrorCount = [int]$Stats.parseErrorCount + 1
            $Stats.parseErrors.Add([ordered]@{ line = [int]$Stats.totalLines; message = $_.Exception.Message })
        }
        $null
    }
}

function New-ActionMapTimelineDetails {
    param([object]$Payload, [string]$Kind)
    $details = [ordered]@{ type = $Kind }
    foreach ($field in @("taskId", "mapId", "nodeId", "resultId", "leaseId", "sentinelId", "traceEventId", "callId", "updateKind", "recordId", "status", "clearAction", "clearanceAction", "clearedBy", "clearedAtMs")) {
        if ($Payload.PSObject.Properties.Name -contains $field -and $null -ne $Payload.$field) { $details[$field] = $Payload.$field }
    }
    if ($Kind -eq "snapshot_updated") {
        foreach ($field in @("checkpointId", "reason", "snapshotSha256")) {
            if ($Payload.PSObject.Properties.Name -contains $field) { $details[$field] = $Payload.$field }
        }
    }
    elseif ($Kind -eq "snapshot_delta") {
        foreach ($field in @("baseCheckpointId", "sequence", "baseSnapshotSha256", "previousSnapshotSha256", "snapshotSha256")) {
            if ($Payload.PSObject.Properties.Name -contains $field) { $details[$field] = $Payload.$field }
        }
    }
    $details
}

function Add-ActionMapRolloutToolItem {
    param(
        [object]$Item,
        [System.Collections.Generic.List[object]]$ToolCalls,
        [hashtable]$ToolCallById,
        [System.Collections.Generic.List[object]]$Timeline,
        [hashtable]$TimelineState,
        [int]$TimelineLimit
    )
    if ([string]$Item.type -ne "response_item" -or -not $Item.payload) { return }
    $payload = $Item.payload
    $at = [string]$Item.timestamp
    $collabTools = @("spawn_agent", "wait_agent", "close_agent", "resume_agent")
    if ([string]$payload.type -eq "function_call" -and $collabTools -contains [string]$payload.name) {
        $tool = [string]$payload.name
        [void](Add-Or-Update-ToolCall $ToolCalls $ToolCallById $at ([string]$payload.call_id) $tool "in_progress" "" @() "" "")
        Add-ActionMapSummaryTimeline $Timeline $TimelineState $TimelineLimit $at "tool:$tool" "tool call: $tool (in_progress)" ([ordered]@{ callId = [string]$payload.call_id })
        return
    }
    if ([string]$payload.type -ne "function_call_output") { return }
    $callId = [string]$payload.call_id
    if (-not $ToolCallById.ContainsKey($callId)) { return }
    $output = [string]$payload.output
    $status = if ($output -match "(?i)\b(error|failed|not found|blocked this tool call)\b") { "failed" } else { "completed" }
    $updated = Add-Or-Update-ToolCall $ToolCalls $ToolCallById $at $callId "" $status "" @() "" ($output.Substring(0, [Math]::Min(600, $output.Length)))
    Add-ActionMapSummaryTimeline $Timeline $TimelineState $TimelineLimit $at "tool:$($updated.tool)" "tool call: $($updated.tool) ($status)" ([ordered]@{ callId = $callId })
}

function New-ActionMapObservabilityEventScan {
    param(
        [Parameter(Mandatory = $true)][string]$RolloutPath,
        [Parameter(Mandatory = $true)][string]$JsonlPath,
        [Parameter(Mandatory = $true)][object]$Policy,
        [Parameter(Mandatory = $true)][object]$RolloutReadStats,
        [Parameter(Mandatory = $true)][object]$JsonlReadStats
    )
    $isLarge = [string]$Policy.rollout_export_mode -eq "summary_only_large_rollout"
    $maxLine = if ($isLarge) { [int64]$Policy.event_parse_max_bytes } else { [int64]::MaxValue }
    $limit = if ($isLarge) { [int]$Policy.timeline_sample_limit } else { [int]::MaxValue }
    $timeline = New-Object System.Collections.Generic.List[object]
    $timelineState = @{ dropped = 0 }
    $toolCalls = New-Object System.Collections.Generic.List[object]
    $toolCallById = @{}
    $runtimeCounts = @{}
    $topCounts = @{}
    $largeEventCounts = @{}

    foreach ($line in Get-Content -LiteralPath $RolloutPath -Encoding UTF8) {
        $read = Read-ActionMapSummaryLine $line $RolloutReadStats $maxLine
        if (-not $read) { continue }
        if ($read.large_line_skipped) {
            Add-ActionMapSummaryCount $largeEventCounts ([string]$read.payload_type)
            Add-ActionMapSummaryCount $runtimeCounts ([string]$read.payload_type)
            Add-ActionMapSummaryTimeline $timeline $timelineState $limit "" ([string]$read.payload_type) "large event omitted from timeline details" ([ordered]@{ large_line_skipped = $true; line_chars = $read.line_chars })
            continue
        }
        $item = $read.row
        Add-ActionMapSummaryCount $topCounts ([string]$item.type)
        $payload = $item.payload
        if ($payload -and $payload.type) {
            $kind = Get-ActionMapRolloutPayloadType $payload
            Add-ActionMapSummaryCount $runtimeCounts $kind
            if ($kind -eq "taskspace_trace_event_recorded" -and [string]$payload.kind) { Add-ActionMapSummaryCount $runtimeCounts ([string]$payload.kind) }
            if ([string]$payload.updateKind -like "state_commit*") { Add-ActionMapSummaryCount $runtimeCounts ([string]$payload.updateKind) }
            Add-ActionMapSummaryTimeline $timeline $timelineState $limit ([string]$item.timestamp) $kind "$kind observed" (New-ActionMapTimelineDetails $payload $kind)
        }
        Add-ActionMapRolloutToolItem $item $toolCalls $toolCallById $timeline $timelineState $limit
    }
    if (Test-Path -LiteralPath $JsonlPath -PathType Leaf) {
        foreach ($line in Get-Content -LiteralPath $JsonlPath -Encoding UTF8) {
            $read = Read-ActionMapSummaryLine $line $JsonlReadStats $maxLine
            if (-not $read -or -not $read.parsed) { continue }
            $eventItem = $read.row.item
            if (-not $eventItem -or [string]$eventItem.type -ne "collab_tool_call") { continue }
            [void](Add-Or-Update-ToolCall $toolCalls $toolCallById "" ([string]$eventItem.id) ([string]$eventItem.tool) ([string]$eventItem.status) ([string]$eventItem.sender_thread_id) @($eventItem.receiver_thread_ids | ForEach-Object { [string]$_ }) "" "")
        }
    }
    [pscustomobject]@{
        timeline = @($timeline.ToArray())
        toolCalls = @($toolCalls.ToArray())
        runtimeEventCounts = Convert-ActionMapSummaryCounts $runtimeCounts
        topLevelEventCounts = Convert-ActionMapSummaryCounts $topCounts
        largeLineEventCounts = Convert-ActionMapSummaryCounts $largeEventCounts
        mapRuntimeEventCount = [int](($runtimeCounts.Values | Measure-Object -Sum).Sum)
        timelineEventsDropped = [int]$timelineState.dropped
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
        metrics = [pscustomobject]@{ outputContractCount = 0; factSourceCount = 0; acceptedResultCount = 0; questionedOrInvalidResultCount = 0; finalArtifactCount = 0 }
        structuralGatePassed = $false
        hardGatePassed = $false
        fullMvpHardGateImplemented = $false
        finalArtifacts = @()
    }
}
