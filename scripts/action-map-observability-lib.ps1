
. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-sentinel-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-cognitive-audit-lib.ps1")

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
        [object]$CreatedFrom = $null,
        [string]$TaskId = ""
    )

    if ([string]::IsNullOrWhiteSpace($MapId)) {
        return $null
    }
    if (-not $MapById.ContainsKey($MapId)) {
        $map = [ordered]@{
            id = $MapId
            taskId = $TaskId
            title = $Title
            ownerSessionId = $OwnerSessionId
            createdFrom = $CreatedFrom
            subagentPlans = New-Object System.Collections.Generic.List[object]
        }
        $MapById[$MapId] = $map
        $Maps.Add($map)
    }
    else {
        $map = $MapById[$MapId]
        if ($Title) { $map.title = $Title }
        if ($TaskId) { $map.taskId = $TaskId }
        if ($OwnerSessionId) { $map.ownerSessionId = $OwnerSessionId }
        if ($CreatedFrom) { $map.createdFrom = $CreatedFrom }
        if ($null -eq $map.subagentPlans) {
            $map.subagentPlans = New-Object System.Collections.Generic.List[object]
        }
    }
    return $map
}

function Add-Or-Update-SubagentPlan {
    param(
        [object]$Map,
        [object]$Plan
    )

    if (-not $Map -or -not $Plan) {
        return
    }
    if ($null -eq $Map.subagentPlans) {
        $Map.subagentPlans = New-Object System.Collections.Generic.List[object]
    }
    $planId = [string]$Plan.id
    if ([string]::IsNullOrWhiteSpace($planId)) {
        return
    }
    $existing = @($Map.subagentPlans | Where-Object { [string]$_.id -eq $planId } | Select-Object -First 1)
    $record = [ordered]@{
        id = $planId
        taskId = [string]$Plan.taskId
        mapId = [string]$Plan.mapId
        parentNodeId = [string]$Plan.parentNodeId
        whyParallelizable = [string]$Plan.whyParallelizable
        expectedArtifact = [string]$Plan.expectedArtifact
        acceptanceCheck = [string]$Plan.acceptanceCheck
        maxScope = [string]$Plan.maxScope
        supportsQuestions = @(Get-ObjectArray $Plan.supportsQuestions)
        testsHypotheses = @(Get-ObjectArray $Plan.testsHypotheses)
        dependsOnResults = @(Get-ObjectArray $Plan.dependsOnResults)
        status = [string]$Plan.status
        leaseId = [string]$Plan.leaseId
        childThreadId = [string]$Plan.childThreadId
        resultIds = @(Get-ObjectArray $Plan.resultIds)
        createdAtMs = [string]$Plan.createdAtMs
        updatedAtMs = [string]$Plan.updatedAtMs
    }
    if ($existing.Count -gt 0) {
        $existingPlan = $existing[0]
        foreach ($property in $record.Keys) {
            $existingPlan[$property] = $record[$property]
        }
        return
    }
    $Map.subagentPlans.Add($record)
}

function Ensure-Task {
    param(
        [System.Collections.Generic.List[object]]$Tasks,
        [hashtable]$TaskById,
        [string]$TaskId,
        [string]$Title = "",
        [string]$Objective = "",
        [string]$Status = "",
        [string]$OwnerSessionId = "",
        [string]$ActiveMapId = "",
        [object]$MapIds = @(),
        [object]$CognitiveState = $null
    )

    if ([string]::IsNullOrWhiteSpace($TaskId)) {
        return $null
    }
    if (-not $TaskById.ContainsKey($TaskId)) {
        $task = [ordered]@{
            id = $TaskId
            title = $Title
            objective = $Objective
            status = $Status
            ownerSessionId = $OwnerSessionId
            activeMapId = $ActiveMapId
            mapIds = @(Get-ObjectArray $MapIds)
            cognitiveState = Convert-CognitiveState $CognitiveState
            events = New-Object System.Collections.Generic.List[object]
        }
        $TaskById[$TaskId] = $task
        $Tasks.Add($task)
    }
    else {
        $task = $TaskById[$TaskId]
        if ($Title) { $task.title = $Title }
        if ($Objective) { $task.objective = $Objective }
        if ($Status) { $task.status = $Status }
        if ($OwnerSessionId) { $task.ownerSessionId = $OwnerSessionId }
        if ($ActiveMapId) { $task.activeMapId = $ActiveMapId }
        if ($null -ne $MapIds) { $task.mapIds = @(Get-ObjectArray $MapIds) }
        if ($null -ne $CognitiveState) { $task.cognitiveState = Convert-CognitiveState $CognitiveState }
    }
    return $task
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
        [string]$SourceEventRef = "",
        [object]$EvidencePackage = $null,
        [string]$MapId = "",
        [string]$TaskId = "",
        [string]$SubagentPlanId = "",
        [object]$ArtifactRefs = @(),
        [object]$ToolSuccess = $null
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
        if ($MapId) { $result.mapId = $MapId }
        if ($TaskId) { $result.taskId = $TaskId }
        if ($SubagentPlanId) { $result.subagentPlanId = $SubagentPlanId }
        if ($SourceEventRef) { $result.sourceEventRef = $SourceEventRef }
        if ($null -ne $ArtifactRefs) { $result.artifactRefs = @(Get-ObjectArray $ArtifactRefs) }
        if ($null -ne $ToolSuccess) { $result.success = [bool]$ToolSuccess }
        if ($null -ne $EvidencePackage) {
            $result["evidencePackage"] = Convert-EvidencePackage $EvidencePackage
            $result["validity"] = "unreviewed"
            $result["claimCount"] = 0
            $result["evidenceRefCount"] = 0
            $result["validatorRefCount"] = 0
            Update-ResultEvidenceDerivedFields $result
        }
        return
    }

    $result = [ordered]@{
        at = $At
        resultId = $ResultId
        mapId = $MapId
        taskId = $TaskId
        leaseId = $LeaseId
        sourceThreadId = $SourceThreadId
        kind = $Kind
        actionClass = $ActionClass
        sourceEventRef = $SourceEventRef
        artifactRefs = @(Get-ObjectArray $ArtifactRefs)
        success = $null
        subagentPlanId = $SubagentPlanId
    }
    if ($null -ne $ToolSuccess) { $result.success = [bool]$ToolSuccess }
    if ($null -ne $EvidencePackage) {
        $result["evidencePackage"] = Convert-EvidencePackage $EvidencePackage
        $result["validity"] = "unreviewed"
        $result["claimCount"] = 0
        $result["evidenceRefCount"] = 0
        $result["validatorRefCount"] = 0
        Update-ResultEvidenceDerivedFields $result
    }
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
