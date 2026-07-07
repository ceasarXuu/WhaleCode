. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function Get-SentinelClearAction {
    param([object]$Details)
    foreach ($field in @("clearAction", "clear_action")) {
        $value = [string](Get-ObjectField $Details $field)
        if (-not [string]::IsNullOrWhiteSpace($value)) { return $value }
    }
    $legacyValue = [string](Get-ObjectField $Details "clearanceAction")
    if (Test-AllowedSentinelClearAction $legacyValue) { return $legacyValue }
    return ""
}

function Get-SentinelDetailsField {
    param([object]$Details, [string[]]$Names)
    foreach ($name in $Names) {
        $value = [string](Get-ObjectField $Details $name)
        if (-not [string]::IsNullOrWhiteSpace($value)) { return $value }
    }
    return ""
}

function Test-AllowedSentinelClearAction {
    param([string]$Action)
    return $Action -in @("FixApplied", "RiskAcceptedByMainAgent", "ContractRevised")
}

function Add-Or-Update-SentinelWarning {
    param(
        [System.Collections.Generic.List[object]]$Warnings,
        [hashtable]$WarningById,
        [string]$At,
        [string]$SentinelId,
        [string]$SentinelType = "",
        [string]$Status = "",
        [string]$Severity = "",
        [string]$TaskId = "",
        [string]$MapId = "",
        [string]$NodeId = "",
        [string]$ResultId = "",
        [object]$TraceEventIds = @(),
        [string]$Reason = "",
        [string]$ClearanceAction = "",
        [string]$CreatedAtMs = "",
        [string]$ClearedAtMs = "",
        [string]$ClearedBy = "",
        [object]$ClearEventIds = @(),
        [string]$ClearAction = ""
    )

    if ([string]::IsNullOrWhiteSpace($SentinelId)) { return $null }
    if (-not $WarningById.ContainsKey($SentinelId)) {
        $clearEventCount = @(Get-ObjectArray $ClearEventIds).Count
        $clearanceSource = if ($Status -eq "cleared" -and ($clearEventCount -gt 0 -or $ClearedBy)) { "event" } elseif ($Status -eq "cleared") { "snapshot" } else { "" }
        $warning = [ordered]@{
            at = $At; id = $SentinelId; sentinelType = $SentinelType; status = $Status; severity = $Severity
            taskId = $TaskId; mapId = $MapId; nodeId = $NodeId; resultId = $ResultId
            traceEventIds = @(Get-ObjectArray $TraceEventIds); reason = $Reason; clearanceAction = $ClearanceAction
            clearAction = $ClearAction
            createdAtMs = $CreatedAtMs; clearedAtMs = $ClearedAtMs; clearedBy = $ClearedBy; clearEventIds = @(Get-ObjectArray $ClearEventIds)
            clearanceSource = $clearanceSource
        }
        $WarningById[$SentinelId] = $warning
        $Warnings.Add($warning)
        return $warning
    }

    $warning = $WarningById[$SentinelId]
    if ($Status -and $Status -ne "cleared" -and $At) { $warning.at = $At }
    elseif ($At -and -not $warning.at) { $warning.at = $At }
    if ($SentinelType) { $warning.sentinelType = $SentinelType }
    if ($Status) { $warning.status = $Status }
    if ($Severity) { $warning.severity = $Severity }
    if ($TaskId) { $warning.taskId = $TaskId }
    if ($MapId) { $warning.mapId = $MapId }
    if ($NodeId) { $warning.nodeId = $NodeId }
    if ($ResultId) { $warning.resultId = $ResultId }
    if (@(Get-ObjectArray $TraceEventIds).Count -gt 0) { $warning.traceEventIds = @(Get-ObjectArray $TraceEventIds) }
    if ($Reason) { $warning.reason = $Reason }
    if ($ClearanceAction) { $warning.clearanceAction = $ClearanceAction }
    if ($ClearAction) { $warning.clearAction = $ClearAction }
    if ($CreatedAtMs) { $warning.createdAtMs = $CreatedAtMs }
    if ($ClearedAtMs) { $warning.clearedAtMs = $ClearedAtMs }
    if ($ClearedBy) { $warning.clearedBy = $ClearedBy }
    if (@(Get-ObjectArray $ClearEventIds).Count -gt 0) { $warning.clearEventIds = @(Get-ObjectArray $ClearEventIds) }
    if ($Status -eq "cleared" -and (@(Get-ObjectArray $ClearEventIds).Count -gt 0 -or $ClearedBy)) { $warning.clearanceSource = "event" }
    elseif ($Status -eq "cleared" -and -not $warning.clearanceSource) { $warning.clearanceSource = "snapshot" }
    return $warning
}

function New-SentinelClearanceRecord {
    param([object]$Details, [string]$At = "")
    [ordered]@{
        at = $At
        sentinelId = Get-SentinelDetailsField $Details @("sentinelId", "sentinel_id")
        clearAction = Get-SentinelClearAction $Details
        taskId = Get-SentinelDetailsField $Details @("taskId", "task_id")
        mapId = Get-SentinelDetailsField $Details @("mapId", "map_id")
        nodeId = Get-SentinelDetailsField $Details @("nodeId", "node_id")
        resultId = Get-SentinelDetailsField $Details @("resultId", "result_id")
        clearedBy = Get-SentinelDetailsField $Details @("clearedBy", "cleared_by")
        clearedAtMs = Get-SentinelDetailsField $Details @("clearedAtMs", "cleared_at_ms")
    }
}

function Add-SentinelClearanceRecord {
    param([hashtable]$ClearanceById, [object]$Record)
    $sentinelId = [string](Get-ObjectField $Record "sentinelId")
    if (-not $sentinelId) { return }
    if (-not $ClearanceById.ContainsKey($sentinelId)) {
        $ClearanceById[$sentinelId] = New-Object System.Collections.Generic.List[object]
    }
    $ClearanceById[$sentinelId].Add($Record)
}

function Get-TimelineSentinelClearances {
    param([object]$Timeline)
    $clearanceById = @{}
    foreach ($event in @(Get-ObjectArray $Timeline)) {
        if ([string](Get-ObjectField $event "kind") -ne "sentinel_warning_cleared") { continue }
        $record = New-SentinelClearanceRecord (Get-ObjectField $event "details") ([string](Get-ObjectField $event "at"))
        if (Test-AllowedSentinelClearAction ([string](Get-ObjectField $record "clearAction"))) {
            Add-SentinelClearanceRecord $clearanceById $record
        }
    }
    return $clearanceById
}

function Test-SentinelClearanceContextMatches {
    param([object]$Warning, [object]$Clearance)
    $matched = 0
    foreach ($field in @("taskId", "mapId", "nodeId", "resultId")) {
        $clearValue = [string](Get-ObjectField $Clearance $field)
        if ([string]::IsNullOrWhiteSpace($clearValue)) { continue }
        $warningValue = [string](Get-ObjectField $Warning $field)
        if ([string]::IsNullOrWhiteSpace($warningValue)) { continue }
        if ($clearValue -ne $warningValue) { return $false }
        $matched++
    }
    return $matched -gt 0
}

function Test-SentinelClearanceNotBeforeWarning {
    param([object]$Warning, [object]$Clearance)
    $warningAt = [string](Get-ObjectField $Warning "at")
    $clearanceAt = [string](Get-ObjectField $Clearance "at")
    if ([string]::IsNullOrWhiteSpace($warningAt) -or [string]::IsNullOrWhiteSpace($clearanceAt)) { return $true }
    try { return [datetime]::Parse($clearanceAt) -ge [datetime]::Parse($warningAt) }
    catch { return $true }
}

function Test-SentinelWarningCleared {
    param([object]$Warning, [hashtable]$TimelineClearanceById)
    $sentinelId = [string](Get-ObjectField $Warning "id")
    if ($sentinelId -and $TimelineClearanceById.ContainsKey($sentinelId)) {
        foreach ($clearance in @(Get-ObjectArray $TimelineClearanceById[$sentinelId])) {
            if ((Test-SentinelClearanceContextMatches $Warning $clearance) -and (Test-SentinelClearanceNotBeforeWarning $Warning $clearance)) { return $true }
        }
    }
    if ([string](Get-ObjectField $Warning "status") -ne "cleared") { return $false }
    if ([string](Get-ObjectField $Warning "clearanceSource") -eq "event") { return $false }
    return Test-AllowedSentinelClearAction (Get-SentinelClearAction $Warning)
}
