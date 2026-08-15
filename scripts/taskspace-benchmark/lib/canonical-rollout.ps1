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
    $sequence = [string]$arguments.type
    $shapes = @{
        initialize_and_work = @('initialize_map', 'tools')
        work = @('tools')
        update_map = @('update_map')
        update_and_work = @('update_map', 'tools')
        update_and_finish = @('update_map', 'finish_map')
        read_map = @('read_map')
        reopen_update_and_work = @('reopen_map', 'update_map', 'tools')
        finish_map = @('finish_map')
    }
    if (-not $shapes.ContainsKey($sequence)) { throw "unknown TaskSpace Exec sequence: $sequence" }
    $declared = New-Object System.Collections.Generic.List[object]
    foreach ($slot in @($shapes[$sequence])) {
        $property = $arguments.PSObject.Properties[$slot]
        if ($null -eq $property) { throw "TaskSpace Exec sequence $sequence is missing $slot" }
        if ($slot -ne 'tools') {
            $declared.Add([pscustomobject]@{
                    call_index = $declared.Count
                    kind = 'map'
                    value = [pscustomobject]@{ operation = $slot; input = $property.Value }
                })
            continue
        }
        $toolIndex = 0
        foreach ($tool in @($property.Value)) {
            $nodeId = $tool.PSObject.Properties['node_id']
            $input = $tool.PSObject.Properties['input']
            $kind = if ($null -ne $nodeId -and $null -ne $input) {
                'client'
            } else {
                'invalid'
            }
            $value = if ($kind -eq 'client') {
                $namespace = $tool.PSObject.Properties['namespace']
                [pscustomobject]@{
                    name = [string]$tool.tool
                    namespace = if ($null -eq $namespace) { $null } else { $namespace.Value }
                    node_id = [string]$nodeId.Value
                    input = $input.Value
                }
            } else { $tool }
            $declared.Add([pscustomobject]@{
                    call_index = $toolIndex
                    kind = $kind
                    value = $value
                })
            $toolIndex++
        }
    }
    @($declared.ToArray())
}
