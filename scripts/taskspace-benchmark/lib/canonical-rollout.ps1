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

function Get-TaskspaceExecDeclaredCalls {
    param($Payload)
    if ($null -eq $Payload -or [string]$Payload.type -ne "function_call" -or
        [string]$Payload.name -ne "taskspace_exec") {
        return @()
    }
    $arguments = if ($Payload.arguments -is [string]) {
        ([string]$Payload.arguments) | ConvertFrom-Json
    } else {
        $Payload.arguments
    }
    $declared = New-Object System.Collections.Generic.List[object]
    $index = 0
    foreach ($call in @($arguments.calls)) {
        $map = $call.PSObject.Properties["map"]
        $client = $call.PSObject.Properties["client"]
        $kind = if ($null -ne $map -and $null -eq $client) {
            "map"
        } elseif ($null -ne $client -and $null -eq $map) {
            "client"
        } else {
            "invalid"
        }
        $declared.Add([pscustomobject]@{
                call_index = $index
                kind = $kind
                value = if ($kind -eq "map") { $map.Value } elseif ($kind -eq "client") { $client.Value } else { $call }
            })
        $index++
    }
    @($declared.ToArray())
}
