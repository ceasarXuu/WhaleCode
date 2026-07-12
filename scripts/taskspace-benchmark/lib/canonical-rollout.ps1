function Get-TaskspaceCanonicalResponseItem {
    param($Row)
    if ($null -eq $Row) { return $null }
    $rowType = [string]$Row.type
    $payload = $Row.payload
    if ($rowType -eq "response_item") { return $payload }
    if ($rowType -eq "event_msg" -and
        $null -ne $payload -and
        [string]$payload.type -eq "task_context_event_recorded") {
        return $payload.rawPayload
    }
    return $null
}
