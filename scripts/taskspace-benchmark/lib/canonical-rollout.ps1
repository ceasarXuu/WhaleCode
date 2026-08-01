function Get-TaskspaceRolloutPayloadType {
    param($Payload)
    if ($null -eq $Payload) { return "" }
    if ([string]$Payload.type -eq "map_runtime") {
        return [string]$Payload.map_event_type
    }
    return [string]$Payload.type
}

function Get-TaskspaceCanonicalResponseItem {
    param($Row)
    if ($null -eq $Row) { return $null }
    $rowType = [string]$Row.type
    $payload = $Row.payload
    if ($rowType -eq "response_item") { return $payload }
    if ($rowType -eq "event_msg" -and
        $null -ne $payload -and
        (Get-TaskspaceRolloutPayloadType $payload) -eq "task_context_event_recorded") {
        return $payload.rawPayload
    }
    return $null
}
