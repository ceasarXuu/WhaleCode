
function Read-JsonLines([string]$PathValue) {
    $items = New-Object System.Collections.Generic.List[object]
    if (-not (Test-Path $PathValue)) {
        return $items
    }

    foreach ($line in Get-Content -LiteralPath $PathValue -Encoding UTF8) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        try {
            $items.Add(($line | ConvertFrom-Json))
        }
        catch {
        }
    }
    return $items
}

function Add-TimelineEvent {
    param(
        [System.Collections.Generic.List[object]]$Timeline,
        [string]$At,
        [string]$Kind,
        [string]$Summary,
        [object]$Details
    )

    $Timeline.Add([ordered]@{
        at = $At
        kind = $Kind
        summary = $Summary
        details = $Details
    })
}

function Ensure-Node {
    param(
        [hashtable]$Nodes,
        [string]$NodeId,
        [string]$Title = "",
        [string]$Kind = ""
    )

    if ([string]::IsNullOrWhiteSpace($NodeId)) {
        return $null
    }
    if (-not $Nodes.ContainsKey($NodeId)) {
        $Nodes[$NodeId] = [ordered]@{
            id = $NodeId
            title = $Title
            kind = if ($Kind) { $Kind } else { "unknown" }
            status = "unknown"
            leases = New-Object System.Collections.Generic.List[object]
            results = New-Object System.Collections.Generic.List[object]
            blockedActions = New-Object System.Collections.Generic.List[object]
            maintenanceBarriers = New-Object System.Collections.Generic.List[object]
            agentThreads = New-Object System.Collections.Generic.List[string]
            events = New-Object System.Collections.Generic.List[object]
        }
    }
    elseif ($Title -and -not $Nodes[$NodeId].title) {
        $Nodes[$NodeId].title = $Title
    }
    if ($Kind -and $Kind -ne "unknown") {
        $Nodes[$NodeId].kind = $Kind
    }
    return $Nodes[$NodeId]
}

function Ensure-Map {
    param(
        [System.Collections.Generic.List[object]]$Maps,
        [hashtable]$MapById,
        [string]$MapId,
        [string]$Title = "",
        [string]$OwnerSessionId = "",
        [object]$CreatedFrom = $null
    )

    if ([string]::IsNullOrWhiteSpace($MapId)) {
        return $null
    }
    if (-not $MapById.ContainsKey($MapId)) {
        $map = [ordered]@{
            id = $MapId
            title = $Title
            ownerSessionId = $OwnerSessionId
            createdFrom = $CreatedFrom
        }
        $MapById[$MapId] = $map
        $Maps.Add($map)
    }
    else {
        $map = $MapById[$MapId]
        if ($Title) { $map.title = $Title }
        if ($OwnerSessionId) { $map.ownerSessionId = $OwnerSessionId }
        if ($CreatedFrom) { $map.createdFrom = $CreatedFrom }
    }
    return $map
}

function Get-ObjectPropertyNames($Value) {
    if ($null -eq $Value) {
        return @()
    }
    return @($Value.PSObject.Properties.Name)
}

function Parse-ResultBody([string]$Body) {
    $parsed = [ordered]@{
        toolName = ""
        callId = ""
        success = $null
        preview = ""
    }
    if ([string]::IsNullOrWhiteSpace($Body)) {
        return $parsed
    }
    if ($Body -match "(?m)^tool:\s*(.+)$") {
        $parsed.toolName = $Matches[1].Trim()
    }
    if ($Body -match "(?m)^call_id:\s*(.+)$") {
        $parsed.callId = $Matches[1].Trim()
    }
    if ($Body -match "(?m)^success:\s*(true|false)\s*$") {
        $parsed.success = [bool]::Parse($Matches[1])
    }
    if ($Body -match "(?ms)^preview:\s*(.*)$") {
        $parsed.preview = $Matches[1].Trim()
    }
    return $parsed
}

function Update-ResultDerivedFields {
    param(
        [object]$Result,
        [string]$Body
    )

    $parsed = Parse-ResultBody $Body
    $Result.toolName = [string]$parsed.toolName
    $Result.callId = [string]$parsed.callId
    $Result.success = $parsed.success
    $Result.preview = [string]$parsed.preview
}

function Add-Or-Update-NodeResult {
    param(
        [object]$Node,
        [string]$At,
        [string]$ResultId,
        [string]$LeaseId,
        [string]$SourceThreadId,
        [string]$Kind,
        [string]$ActionClass = "",
        [string]$Body = ""
    )

    if (-not $Node -or [string]::IsNullOrWhiteSpace($ResultId)) {
        return
    }

    $existing = @($Node.results | Where-Object { $_.resultId -eq $ResultId } | Select-Object -First 1)
    if ($existing.Count -gt 0) {
        $result = $existing[0]
        if ($At) {
            if ([string]::IsNullOrWhiteSpace([string]$result.at)) {
                $result.at = $At
            }
            else {
                try {
                    if ([datetime]::Parse($At) -lt [datetime]::Parse([string]$result.at)) { $result.at = $At }
                }
                catch {
                }
            }
        }
        if ($LeaseId) { $result.leaseId = $LeaseId }
        if ($SourceThreadId) { $result.sourceThreadId = $SourceThreadId }
        if ($Kind) { $result.kind = $Kind }
        if ($ActionClass) { $result.actionClass = $ActionClass }
        if ($Body) {
            $result.body = $Body
            Update-ResultDerivedFields $result $Body
        }
        return
    }

    $result = [ordered]@{
        at = $At
        resultId = $ResultId
        leaseId = $LeaseId
        sourceThreadId = $SourceThreadId
        kind = $Kind
        actionClass = $ActionClass
        body = $Body
        toolName = ""
        callId = ""
        success = $null
        preview = ""
    }
    Update-ResultDerivedFields $result $Body
    $Node.results.Add($result)
}

function Add-Or-Update-Lease {
    param(
        [object]$Node,
        [string]$At,
        [string]$LeaseId,
        [string]$State,
        [string]$Reason = "",
        [string]$AgentThreadId = ""
    )

    if (-not $Node -or [string]::IsNullOrWhiteSpace($LeaseId)) {
        return
    }
    $existing = @($Node.leases | Where-Object { $_.leaseId -eq $LeaseId } | Select-Object -First 1)
    if ($existing.Count -gt 0) {
        $lease = $existing[0]
        if ($State) { $lease.state = $State }
        if ($At) {
            if ($State -eq "released") { $lease.releasedAt = $At } else { $lease.at = $At }
        }
        if ($Reason) { $lease.reason = $Reason }
        if ($AgentThreadId) { $lease.agentThreadId = $AgentThreadId }
        return
    }
    $Node.leases.Add([ordered]@{
        at = $At
        leaseId = $LeaseId
        state = $State
        reason = $Reason
        agentThreadId = $AgentThreadId
        releasedAt = ""
    })
}

function Add-Or-Update-MaintenanceBarrier {
    param(
        [object]$Node,
        [string]$At,
        [string]$MapId,
        [string]$Reason,
        [int]$ResultCount,
        [int]$Budget,
        [string]$State
    )

    if (-not $Node) {
        return
    }
    if ($State -eq "cleared") {
        $existing = @($Node.maintenanceBarriers | Where-Object {
                $_.mapId -eq $MapId -and $_.state -eq "active"
            } | Select-Object -First 1)
    }
    else {
        $existing = @($Node.maintenanceBarriers | Where-Object {
                $_.mapId -eq $MapId -and $_.reason -eq $Reason -and $_.state -ne "cleared"
            } | Select-Object -First 1)
    }
    if ($existing.Count -gt 0) {
        $barrier = $existing[0]
        if ($State) { $barrier.state = $State }
        if ($At) {
            if ($State -eq "cleared") { $barrier.clearedAt = $At } else { $barrier.at = $At }
        }
        if ($State -eq "cleared" -and $Reason) { $barrier.clearReason = $Reason }
        if ($ResultCount -ge 0) { $barrier.resultCount = $ResultCount }
        if ($Budget -ge 0) { $barrier.budget = $Budget }
        return
    }
    $Node.maintenanceBarriers.Add([ordered]@{
        at = $At
        mapId = $MapId
        reason = $Reason
        resultCount = $ResultCount
        budget = $Budget
        state = $State
        clearedAt = ""
        clearReason = ""
    })
}

function Add-Or-Update-ToolCall {
    param(
        [System.Collections.Generic.List[object]]$ToolCalls,
        [hashtable]$ToolCallById,
        [string]$At,
        [string]$CallId,
        [string]$Tool,
        [string]$Status,
        [string]$SenderThreadId = "",
        [string[]]$ReceiverThreadIds = @(),
        [string]$PromptPreview = "",
        [string]$OutputPreview = ""
    )

    if ([string]::IsNullOrWhiteSpace($CallId)) {
        return $null
    }
    if ($ToolCallById.ContainsKey($CallId)) {
        $toolCall = $ToolCallById[$CallId]
        if ($At) { $toolCall.at = $At }
        if ($Tool) { $toolCall.tool = $Tool }
        if ($Status) { $toolCall.status = $Status }
        if ($SenderThreadId) { $toolCall.senderThreadId = $SenderThreadId }
        if ($ReceiverThreadIds.Count -gt 0) { $toolCall.receiverThreadIds = @($ReceiverThreadIds) }
        if ($PromptPreview) { $toolCall.promptPreview = $PromptPreview }
        if ($OutputPreview) { $toolCall.outputPreview = $OutputPreview }
        return $toolCall
    }
    $toolCall = [ordered]@{
        at = $At
        id = $CallId
        tool = $Tool
        status = $Status
        senderThreadId = $SenderThreadId
        receiverThreadIds = @($ReceiverThreadIds)
        promptPreview = $PromptPreview
        outputPreview = $OutputPreview
    }
    $ToolCallById[$CallId] = $toolCall
    $ToolCalls.Add($toolCall)
    return $toolCall
}

function Get-ToolCallSemanticKey {
    param([object]$ToolCall)
    $receivers = @($ToolCall.receiverThreadIds) -join ","
    return "$($ToolCall.tool)|$($ToolCall.status)|$receivers|$($ToolCall.promptPreview)|$($ToolCall.outputPreview)"
}

function Has-TimestampedToolCallDuplicate {
    param(
        [System.Collections.Generic.List[object]]$ToolCalls,
        [object]$Candidate
    )
    $candidateKey = Get-ToolCallSemanticKey $Candidate
    foreach ($existing in @($ToolCalls)) {
        if ([string]::IsNullOrWhiteSpace([string]$existing.at)) {
            continue
        }
        if ((Get-ToolCallSemanticKey $existing) -eq $candidateKey) {
            return $true
        }
    }
    return $false
}

function Has-TimestampedToolCallWithStatus {
    param(
        [System.Collections.Generic.List[object]]$ToolCalls,
        [string]$Tool,
        [string]$Status
    )
    foreach ($existing in @($ToolCalls)) {
        if ([string]::IsNullOrWhiteSpace([string]$existing.at)) {
            continue
        }
        if ([string]$existing.tool -eq $Tool -and [string]$existing.status -eq $Status) {
            return $true
        }
    }
    return $false
}

function Has-TimestampedToolCall {
    param(
        [System.Collections.Generic.List[object]]$ToolCalls,
        [string]$Tool
    )
    foreach ($existing in @($ToolCalls)) {
        if ([string]::IsNullOrWhiteSpace([string]$existing.at)) {
            continue
        }
        if ([string]$existing.tool -eq $Tool) {
            return $true
        }
    }
    return $false
}

function Escape-Html([string]$Text) {
    return [System.Net.WebUtility]::HtmlEncode($Text)
}

