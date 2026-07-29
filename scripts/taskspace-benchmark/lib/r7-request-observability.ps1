function Get-R7PrimaryFailureClass {
    param([object[]]$Calls)
    $classes = @(
        $Calls |
            Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.failure_class) } |
            ForEach-Object { [string]$_.failure_class } |
            Sort-Object -Unique
    )
    foreach ($candidate in @(
            "tool_sequence_protocol",
            "taskspace_state_machine",
            "taskspace_protocol",
            "taskspace_resource",
            "taskspace",
            "ordinary_tool"
        )) {
        if ($candidate -in $classes) { return $candidate }
    }
    if ($classes.Count) { return $classes[0] }
    "none"
}

function Add-R7RequestFailureFacts {
    param([Parameter(Mandatory = $true)]$Request)
    $failedCalls = @($Request.calls | Where-Object { $_.success -eq $false })
    $primary = Get-R7PrimaryFailureClass $failedCalls
    $signatures = @(
        $failedCalls |
            ForEach-Object {
                "{0}|{1}|{2}" -f $_.failure_class, $_.failure_code, (@($_.violation_codes) -join ",")
            } |
            Sort-Object -Unique
    )
    $secondaryTags = @(
        $failedCalls |
            ForEach-Object {
                $call = $_
                if ($call.failure_class) { "class:$($call.failure_class)" }
                if ($call.failure_code) { "code:$($call.failure_code)" }
                @($call.violation_codes) | ForEach-Object { "violation:$_" }
            } |
            Sort-Object -Unique
    )
    $Request | Add-Member -Force -NotePropertyName primary_failure_class -NotePropertyValue $primary
    $Request | Add-Member -Force -NotePropertyName secondary_failure_tags -NotePropertyValue $secondaryTags
    $Request | Add-Member -Force -NotePropertyName failed_call_count -NotePropertyValue $failedCalls.Count
    $Request | Add-Member -Force -NotePropertyName sibling_failure_copy_count -NotePropertyValue (
        [Math]::Max(0, $failedCalls.Count - $signatures.Count)
    )
}

function Get-R7WireRequestFacts {
    param([Parameter(Mandatory = $true)][string]$WireTracePath)
    $facts = @{}
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $WireTracePath) {
        $event = $line | ConvertFrom-Json -Depth 100
        $requestId = [string](Get-R7JsonProperty $event "request_id" "")
        if ([string]::IsNullOrWhiteSpace($requestId)) { continue }
        if (-not $facts.ContainsKey($requestId)) {
            $facts[$requestId] = [ordered]@{
                request_id = $requestId
                request_index = 0
                provider_wire_api = ""
                lcp_message_count = 0
                message_shapes = @()
                input_tokens = $null
                cached_input_tokens = $null
            }
        }
        $fact = $facts[$requestId]
        if ($null -ne (Get-R7JsonProperty $event "request_index")) {
            $fact.request_index = [int]$event.request_index
            $fact.provider_wire_api = [string](Get-R7JsonProperty $event "provider_wire_api" "")
            $fact.lcp_message_count = [int](Get-R7JsonProperty $event "lcp_message_count" 0)
            $fact.message_shapes = @(Get-R7JsonProperty $event "message_shapes" @())
        }
        if ([string](Get-R7JsonProperty $event "event_name" "") -eq "provider.chat_wire_request_terminal") {
            $fact.input_tokens = Get-R7JsonProperty $event "input_tokens"
            $fact.cached_input_tokens = Get-R7JsonProperty $event "cached_input_tokens"
        }
    }
    $ordered = @(
        $facts.Values |
            Where-Object { [int]$_.request_index -gt 0 } |
            Sort-Object { [int]$_.request_index } |
            ForEach-Object { [pscustomobject]$_ }
    )
    $incomplete = @($ordered | Where-Object {
            $null -eq $_.input_tokens -or $null -eq $_.cached_input_tokens
        })
    if ($incomplete.Count) {
        throw "Provider wire trace has $($incomplete.Count) request rows without terminal cache facts"
    }
    $ordered
}

function Add-R7WireFactsToRequestPath {
    param(
        [Parameter(Mandatory = $true)][object[]]$RequestPath,
        [Parameter(Mandatory = $true)][string]$WireTracePath
    )
    $wire = @(Get-R7WireRequestFacts $WireTracePath)
    if ($wire.Count -ne $RequestPath.Count) {
        throw "Provider wire/request path count mismatch: wire=$($wire.Count) request_path=$($RequestPath.Count)"
    }
    for ($index = 0; $index -lt $RequestPath.Count; $index++) {
        $request = $RequestPath[$index]
        $fact = $wire[$index]
        if ([int]$fact.request_index -ne ($index + 1)) {
            throw "Provider wire request index mismatch at $($index + 1)"
        }
        $inputTokens = [double]$fact.input_tokens
        $cachedTokens = [double]$fact.cached_input_tokens
        $cacheHitRate = if ($inputTokens -gt 0) {
            [Math]::Round($cachedTokens / $inputTokens, 6)
        } else {
            $null
        }
        $request | Add-Member -Force -NotePropertyName provider_wire_request_id -NotePropertyValue $fact.request_id
        $request | Add-Member -Force -NotePropertyName provider_wire_api -NotePropertyValue $fact.provider_wire_api
        $request | Add-Member -Force -NotePropertyName input_tokens -NotePropertyValue $inputTokens
        $request | Add-Member -Force -NotePropertyName cached_input_tokens -NotePropertyValue $cachedTokens
        $request | Add-Member -Force -NotePropertyName cache_hit_rate -NotePropertyValue $cacheHitRate
        $wireRole = ""
        if ([bool]$request.receipt_before) {
            $candidateRoles = @(
                $fact.message_shapes |
                    Where-Object {
                        [int]$_.index -ge [int]$fact.lcp_message_count -and
                        [string]$_.role -in @("developer", "system")
                    } |
                    Sort-Object { [int]$_.index } |
                    ForEach-Object { [string]$_.role }
            )
            if ($candidateRoles.Count) { $wireRole = $candidateRoles[-1] }
        }
        $request | Add-Member -Force -NotePropertyName receipt_wire_role -NotePropertyValue $wireRole
    }
    $RequestPath
}

function Get-R7RequestObservabilitySummary {
    param([Parameter(Mandatory = $true)][object[]]$RequestPath)
    $classes = [ordered]@{}
    foreach ($name in @(
            "none",
            "tool_sequence_protocol",
            "taskspace_state_machine",
            "taskspace_protocol",
            "taskspace_resource",
            "taskspace",
            "ordinary_tool"
        )) {
        $classes[$name] = @($RequestPath | Where-Object primary_failure_class -eq $name).Count
    }
    $knownCount = ($classes.Values | Measure-Object -Sum).Sum
    $unknown = @($RequestPath | Where-Object {
            [string]$_.primary_failure_class -notin @($classes.Keys)
        })
    $receipt = @($RequestPath | Where-Object receipt_before -eq $true)
    $withoutReceipt = @($RequestPath | Where-Object receipt_before -ne $true)
    $receiptInput = ($receipt | Measure-Object -Property input_tokens -Sum).Sum
    $receiptCached = ($receipt | Measure-Object -Property cached_input_tokens -Sum).Sum
    $otherInput = ($withoutReceipt | Measure-Object -Property input_tokens -Sum).Sum
    $otherCached = ($withoutReceipt | Measure-Object -Property cached_input_tokens -Sum).Sum
    [pscustomobject]@{
        provider_requests = $RequestPath.Count
        primary_failure_counts = [pscustomobject]$classes
        unknown_primary_failure_count = $unknown.Count
        classification_reconciled = ($unknown.Count -eq 0 -and $knownCount -eq $RequestPath.Count)
        sibling_failure_copy_count = (
            $RequestPath | Measure-Object -Property sibling_failure_copy_count -Sum
        ).Sum
        receipt_before_requests = $receipt.Count
        receipt_before_input_tokens = $receiptInput
        receipt_before_cached_input_tokens = $receiptCached
        receipt_before_cache_hit_rate = if ($receiptInput -gt 0) {
            [Math]::Round($receiptCached / $receiptInput, 6)
        } else {
            $null
        }
        no_receipt_before_requests = $withoutReceipt.Count
        no_receipt_before_input_tokens = $otherInput
        no_receipt_before_cached_input_tokens = $otherCached
        no_receipt_before_cache_hit_rate = if ($otherInput -gt 0) {
            [Math]::Round($otherCached / $otherInput, 6)
        } else {
            $null
        }
        receipt_original_roles = @(
            $receipt | ForEach-Object receipt_original_role | Where-Object { $_ } | Sort-Object -Unique
        )
        receipt_wire_roles = @(
            $receipt | ForEach-Object receipt_wire_role | Where-Object { $_ } | Sort-Object -Unique
        )
        receipt_wire_role_unresolved_count = @(
            $receipt | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.receipt_wire_role) }
        ).Count
    }
}
