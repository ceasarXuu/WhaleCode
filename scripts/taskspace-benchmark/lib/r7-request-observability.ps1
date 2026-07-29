function Get-R7PrimaryFailureClass {
    param([object[]]$Calls)
    $classes = @(
        $Calls |
            Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.failure_class) } |
            ForEach-Object { [string]$_.failure_class } |
            Sort-Object -Unique
    )
    foreach ($candidate in @(
            "evidence_unclassified",
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
    $copyGroups = @(
        $failedCalls |
            Where-Object {
                [bool]$_.zero_dispatch -and
                -not [string]::IsNullOrWhiteSpace([string]$_.failure_copy_group_id)
            } |
            Group-Object failure_copy_group_id
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
        ($copyGroups | ForEach-Object { [Math]::Max(0, $_.Count - 1) } | Measure-Object -Sum).Sum
    )
    $invalidEvidence = @($Request.calls | Where-Object { -not [bool]$_.evidence_valid })
    $Request | Add-Member -Force -NotePropertyName invalid_evidence_count -NotePropertyValue (
        $invalidEvidence.Count
    )
    $Request | Add-Member -Force -NotePropertyName evidence_health -NotePropertyValue $(if (
            $invalidEvidence.Count
        ) { "invalid" } else { "valid" })
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
                logical_request_id = ""
                attempt_seq = 0
                request_index = 0
                trace_schema = ""
                provider_wire_api = ""
                lcp_message_count = 0
                message_shapes = @()
                receipt_identities = @()
                input_tokens = $null
                cached_input_tokens = $null
                terminal_status = ""
                shape_event_count = 0
                terminal_event_count = 0
            }
        }
        $fact = $facts[$requestId]
        if ($null -ne (Get-R7JsonProperty $event "request_index")) {
            $fact.shape_event_count = [int]$fact.shape_event_count + 1
            $fact.request_index = [int]$event.request_index
            $fact.trace_schema = [string](Get-R7JsonProperty $event "schema_version" "")
            $fact.logical_request_id =
                [string](Get-R7JsonProperty $event "logical_request_id" "")
            $fact.attempt_seq = [int](Get-R7JsonProperty $event "attempt_seq" 0)
            $fact.provider_wire_api = [string](Get-R7JsonProperty $event "provider_wire_api" "")
            $fact.lcp_message_count = [int](Get-R7JsonProperty $event "lcp_message_count" 0)
            $fact.message_shapes = @(Get-R7JsonProperty $event "message_shapes" @())
            $receiptIdentity = Get-R7JsonProperty $event "taskspace_final_receipt_identity"
            $fact.receipt_identities = @(
                Get-R7JsonProperty $receiptIdentity "receipts" @()
            )
        }
        if ([string](Get-R7JsonProperty $event "event_name" "") -eq "provider.chat_wire_request_terminal") {
            $fact.terminal_event_count = [int]$fact.terminal_event_count + 1
            $fact.terminal_status = [string](Get-R7JsonProperty $event "status" "")
            if ([string](Get-R7JsonProperty $event "logical_request_id" "") -ne
                [string]$fact.logical_request_id -or
                [int](Get-R7JsonProperty $event "attempt_seq" 0) -ne [int]$fact.attempt_seq) {
                throw "Provider wire shape/terminal identity mismatch: $requestId"
            }
            $fact.input_tokens = Get-R7JsonProperty $event "input_tokens"
            $fact.cached_input_tokens = Get-R7JsonProperty $event "cached_input_tokens"
        }
    }
    $ordered = @(
        $facts.Values |
            Where-Object {
                [int]$_.request_index -gt 0 -and
                [string]$_.terminal_status -eq "response_completed"
            } |
            Sort-Object { [int]$_.request_index } |
            ForEach-Object { [pscustomobject]$_ }
    )
    $incomplete = @($ordered | Where-Object {
            [string]$_.trace_schema -ne "provider-chat-wire-trace-v9" -or
            [string]::IsNullOrWhiteSpace([string]$_.logical_request_id) -or
            [int]$_.attempt_seq -lt 1 -or
            [int]$_.shape_event_count -ne 1 -or
            [int]$_.terminal_event_count -ne 1 -or
            $null -eq $_.input_tokens -or
            $null -eq $_.cached_input_tokens
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
    $wireById = @{}
    foreach ($fact in $wire) {
        if ($wireById.ContainsKey([string]$fact.request_id)) {
            throw "Duplicate completed provider wire request id: $($fact.request_id)"
        }
        $wireById[[string]$fact.request_id] = $fact
    }
    for ($index = 0; $index -lt $RequestPath.Count; $index++) {
        $request = $RequestPath[$index]
        $rolloutRequestId = [string]$request.rollout_provider_request_id
        if (-not $wireById.ContainsKey($rolloutRequestId)) {
            throw "Rollout provider request identity is absent from wire trace: $rolloutRequestId"
        }
        $fact = $wireById[$rolloutRequestId]
        if ([string]$request.rollout_provider_logical_request_id -ne
            [string]$fact.logical_request_id -or
            [int]$request.rollout_provider_attempt_seq -ne [int]$fact.attempt_seq) {
            throw "Rollout/wire provider request identity mismatch: $rolloutRequestId"
        }
        $inputTokens = [double]$fact.input_tokens
        $cachedTokens = [double]$fact.cached_input_tokens
        $cacheHitRate = if ($inputTokens -gt 0) {
            [Math]::Round($cachedTokens / $inputTokens, 6)
        } else {
            $null
        }
        $request | Add-Member -Force -NotePropertyName provider_wire_request_id -NotePropertyValue $fact.request_id
        $request | Add-Member -Force -NotePropertyName provider_wire_trace_schema -NotePropertyValue $fact.trace_schema
        $request | Add-Member -Force -NotePropertyName provider_wire_api -NotePropertyValue $fact.provider_wire_api
        $request | Add-Member -Force -NotePropertyName input_tokens -NotePropertyValue $inputTokens
        $request | Add-Member -Force -NotePropertyName cached_input_tokens -NotePropertyValue $cachedTokens
        $request | Add-Member -Force -NotePropertyName cache_hit_rate -NotePropertyValue $cacheHitRate
        $receiptWireIdentity = $null
        $newReceipts = @(
            $fact.receipt_identities |
                Where-Object { [int]$_.message_index -ge [int]$fact.lcp_message_count } |
                Sort-Object { [int]$_.message_index }
        )
        if ([bool]$request.receipt_before) {
            if ([int]$request.receipt_count -ne 1 -or $newReceipts.Count -ne 1) {
                throw "Provider request must carry exactly one complete response-final receipt: $rolloutRequestId"
            }
            $receiptWireIdentity = $newReceipts[0]
            $wireRole = [string](Get-R7JsonProperty $receiptWireIdentity "wire_role" "")
            $callHash = [string](Get-R7JsonProperty $receiptWireIdentity "control_call_id_sha256" "")
            $reservationRevision =
                Get-R7JsonProperty $receiptWireIdentity "reservation_revision_after"
            $canonicalRevision = Get-R7JsonProperty $receiptWireIdentity "canonical_revision"
            $revisionDelta = Get-R7JsonProperty $receiptWireIdentity "revision_delta"
            $receiptComplete = [bool](Get-R7JsonProperty $receiptWireIdentity "complete" $false)
            if ([string]::IsNullOrWhiteSpace($wireRole) -or
                $callHash -notmatch '^[a-fA-F0-9]{64}$' -or
                $null -eq $reservationRevision -or $null -eq $canonicalRevision -or
                $null -eq $revisionDelta -or -not $receiptComplete -or
                [int64]$revisionDelta -ne
                    ([int64]$canonicalRevision - [int64]$reservationRevision)) {
                throw "Provider response-final receipt identity is incomplete: $rolloutRequestId"
            }
        } elseif ($newReceipts.Count) {
            throw "Wire trace has an unpaired response-final receipt: $rolloutRequestId"
        }
        $request | Add-Member -Force -NotePropertyName receipt_wire_role -NotePropertyValue (
            [string](Get-R7JsonProperty $receiptWireIdentity "wire_role" "")
        )
        $request | Add-Member -Force -NotePropertyName receipt_message_index -NotePropertyValue (
            Get-R7JsonProperty $receiptWireIdentity "message_index"
        )
        $request | Add-Member -Force -NotePropertyName receipt_control_call_id_sha256 -NotePropertyValue (
            [string](Get-R7JsonProperty $receiptWireIdentity "control_call_id_sha256" "")
        )
        $request | Add-Member -Force -NotePropertyName receipt_reservation_revision -NotePropertyValue (
            Get-R7JsonProperty $receiptWireIdentity "reservation_revision_after"
        )
        $request | Add-Member -Force -NotePropertyName receipt_canonical_revision -NotePropertyValue (
            Get-R7JsonProperty $receiptWireIdentity "canonical_revision"
        )
        $request | Add-Member -Force -NotePropertyName receipt_revision_delta -NotePropertyValue (
            Get-R7JsonProperty $receiptWireIdentity "revision_delta"
        )
        $request | Add-Member -Force -NotePropertyName receipt_complete -NotePropertyValue (
            Get-R7JsonProperty $receiptWireIdentity "complete"
        )
    }
    $RequestPath
}

function Get-R7RequestObservabilitySummary {
    param([Parameter(Mandatory = $true)][object[]]$RequestPath)
    $classes = [ordered]@{}
    foreach ($name in @(
            "none",
            "evidence_unclassified",
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
    $invalidEvidence = @($RequestPath | Where-Object evidence_health -ne "valid")
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
        evidence_health = if ($invalidEvidence.Count) { "invalid" } else { "valid" }
        invalid_evidence_request_count = $invalidEvidence.Count
        invalid_evidence_call_count = (
            $RequestPath | Measure-Object -Property invalid_evidence_count -Sum
        ).Sum
        classification_reconciled = (
            $unknown.Count -eq 0 -and
            $knownCount -eq $RequestPath.Count -and
            $invalidEvidence.Count -eq 0
        )
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
