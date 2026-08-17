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
    $Request | Add-Member -Force -NotePropertyName failed_call_count -NotePropertyValue (
        [int64]$failedCalls.Count
    )
    $Request | Add-Member -Force -NotePropertyName sibling_failure_copy_count -NotePropertyValue (
        Get-R7ExactInt64Sum @($copyGroups | ForEach-Object {
                $scope = [string]$_.Group[0].failure_provenance_scope
                if ($scope -eq "tool_sequence_skip") {
                    [int64]$_.Count
                } elseif ($scope -eq "provider_response") {
                    [int64][Math]::Max(0, $_.Count - 1)
                } else {
                    [int64]0
                }
            }) "sibling_failure_copy_count"
    )
    $invalidEvidence = @($Request.calls | Where-Object { -not [bool]$_.evidence_valid })
    $Request | Add-Member -Force -NotePropertyName invalid_evidence_count -NotePropertyValue (
        [int64]$invalidEvidence.Count
    )
    $Request | Add-Member -Force -NotePropertyName evidence_health -NotePropertyValue $(if (
            $invalidEvidence.Count
        ) { "invalid" } else { "valid" })
}

function Get-R7WireRequestInventory {
    param(
        [Parameter(Mandatory = $true)][string]$WireTracePath,
        $RequestFacts = $null
    )
    if ($null -eq $RequestFacts) {
        $RequestFacts = Invoke-TaskspaceRequestFactsGenerator -WireTracePath $WireTracePath
    }
    if ([string]$RequestFacts.availability.attempt -ne "measured" -or
        [string]$RequestFacts.availability.completion -ne "measured" -or
        [string]$RequestFacts.availability.usage -ne "measured") {
        $codes = @($RequestFacts.findings | ForEach-Object { [string]$_.code }) -join ","
        throw "Canonical request facts are unavailable for wire inventory: $codes"
    }

    $wireShapes = @{}
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $WireTracePath) {
        $event = $line | ConvertFrom-Json -Depth 100
        if ($null -eq (Get-R7JsonProperty $event "request_index")) { continue }
        $requestId = [string](Get-R7JsonProperty $event "request_id" "")
        if ([string]::IsNullOrWhiteSpace($requestId)) {
            throw "Provider wire shape event has no request identity"
        }
        if ($wireShapes.ContainsKey($requestId)) {
            throw "Provider wire trace has duplicate shape event: $requestId"
        }
        $resultIdentity = Get-R7JsonProperty $event "taskspace_final_control_result_identity"
        $wireShapes[$requestId] = [pscustomobject]@{
            trace_schema = [string](Get-R7JsonProperty $event "schema_version" "")
            transport = [string](Get-R7JsonProperty $event "transport" "")
            provider_wire_api = [string](Get-R7JsonProperty $event "provider_wire_api" "")
            lcp_message_count = Get-R7RequiredNonnegativeInt64Fact `
                $event "lcp_message_count" "provider wire shape event"
            message_shapes = @(Get-R7JsonProperty $event "message_shapes" @())
            final_control_result_identities = @(
                Get-R7JsonProperty $resultIdentity "results" @()
            )
        }
    }

    $inventory = @(
        $RequestFacts.rows |
            Where-Object { $null -ne $_.wire_attempt_line_number } |
            Sort-Object { [int64]$_.request_index } |
            ForEach-Object {
                $row = $_
                $requestId = [string]$row.request_id
                if (-not $wireShapes.ContainsKey($requestId)) {
                    throw "Canonical request fact has no Provider wire shape: $requestId"
                }
                $shape = $wireShapes[$requestId]
                if ([string]$shape.trace_schema -ne "provider-chat-wire-trace-v11" -or
                    [string]::IsNullOrWhiteSpace([string]$shape.transport)) {
                    throw "Provider wire shape metadata is incomplete: $requestId"
                }
                $usage = $row.usage
                [pscustomobject]@{
                    request_id = $requestId
                    logical_request_id = [string]$row.logical_request_id
                    attempt_seq = [int64]$row.attempt_seq
                    request_index = [int64]$row.request_index
                    trace_schema = [string]$shape.trace_schema
                    transport = [string]$shape.transport
                    provider_wire_api = [string]$shape.provider_wire_api
                    lcp_message_count = [int64]$shape.lcp_message_count
                    message_shapes = @($shape.message_shapes)
                    final_control_result_identities = @($shape.final_control_result_identities)
                    input_tokens = if ($null -ne $usage) { [int64]$usage.input_tokens } else { $null }
                    cached_input_tokens = if ($null -ne $usage) { [int64]$usage.cached_input_tokens } else { $null }
                    output_tokens = if ($null -ne $usage) { [int64]$usage.output_tokens } else { $null }
                    reasoning_output_tokens = if ($null -ne $usage) { [int64]$usage.reasoning_output_tokens } else { $null }
                    total_tokens = if ($null -ne $usage) { [int64]$usage.total_tokens } else { $null }
                    terminal_status = [string]$row.terminal_status
                }
            }
    )
    if ($inventory.Count -eq 0 -or $inventory.Count -ne $wireShapes.Count) {
        throw "Canonical request facts and Provider wire shapes do not form a complete inventory"
    }
    $inventory
}

function Get-R7WireRequestFacts {
    param([Parameter(Mandatory = $true)][string]$WireTracePath)
    @(
        Get-R7WireRequestInventory $WireTracePath |
            Where-Object terminal_status -eq "response_completed"
    )
}

function Get-R7WireAttemptSummary {
    param([Parameter(Mandatory = $true)][object[]]$Inventory)
    $statusCounts = [ordered]@{}
    foreach ($status in @("response_completed", "response_failed", "cancelled", "retry_unauthorized")) {
        $statusCounts[$status] = @($Inventory | Where-Object terminal_status -eq $status).Count
    }
    [pscustomobject]@{
        physical_attempt_count = [int64]$Inventory.Count
        logical_request_count = [int64]@($Inventory | Group-Object logical_request_id).Count
        completed_response_count = [int64]$statusCounts.response_completed
        failed_or_cancelled_attempt_count = [int64]@(
            $Inventory | Where-Object terminal_status -ne "response_completed"
        ).Count
        retried_logical_request_count = [int64]@(
            $Inventory | Group-Object logical_request_id | Where-Object Count -gt 1
        ).Count
        transports = @($Inventory | ForEach-Object transport | Sort-Object -Unique)
        terminal_status_counts = [pscustomobject]$statusCounts
    }
}

function Add-R7WireFactsToRequestPath {
    param(
        [Parameter(Mandatory = $true)][object[]]$RequestPath,
        [Parameter(Mandatory = $true)][string]$WireTracePath,
        [int64]$ExpectedProviderAttempts = 0,
        $RequestFacts = $null
    )
    $inventory = @(Get-R7WireRequestInventory $WireTracePath $RequestFacts)
    if ($ExpectedProviderAttempts -gt 0 -and $inventory.Count -ne $ExpectedProviderAttempts) {
        throw "Provider wire physical attempt count mismatch: wire=$($inventory.Count) expected=$ExpectedProviderAttempts"
    }
    $wire = @($inventory | Where-Object terminal_status -eq "response_completed")
    if ($wire.Count -ne $RequestPath.Count) {
        throw "Completed provider response/request path count mismatch: wire=$($wire.Count) request_path=$($RequestPath.Count)"
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
            [int64]$request.rollout_provider_attempt_seq -ne [int64]$fact.attempt_seq) {
            throw "Rollout/wire provider request identity mismatch: $rolloutRequestId"
        }
        $inputTokens = [int64]$fact.input_tokens
        $cachedTokens = [int64]$fact.cached_input_tokens
        $outputTokens = [int64]$fact.output_tokens
        $reasoningTokens = [int64]$fact.reasoning_output_tokens
        $totalTokens = [int64]$fact.total_tokens
        $cacheHitRate = if ($inputTokens -gt 0) {
            [Math]::Round($cachedTokens / $inputTokens, 6)
        } else {
            $null
        }
        $request | Add-Member -Force -NotePropertyName provider_wire_request_id -NotePropertyValue $fact.request_id
        $request | Add-Member -Force -NotePropertyName provider_wire_trace_schema -NotePropertyValue $fact.trace_schema
        $request | Add-Member -Force -NotePropertyName provider_wire_api -NotePropertyValue $fact.provider_wire_api
        $request | Add-Member -Force -NotePropertyName provider_transport -NotePropertyValue $fact.transport
        $logicalAttempts = @(
            $inventory |
                Where-Object logical_request_id -eq $fact.logical_request_id |
                Sort-Object { [int64]$_.attempt_seq }
        )
        $priorAttempts = @(
            $logicalAttempts | Where-Object {
                [int64]$_.attempt_seq -lt [int64]$fact.attempt_seq
            }
        )
        $request | Add-Member -Force -NotePropertyName provider_attempt_count -NotePropertyValue (
            [int64]$logicalAttempts.Count
        )
        $request | Add-Member -Force -NotePropertyName provider_prior_failed_attempt_count -NotePropertyValue (
            [int64]$priorAttempts.Count
        )
        $request | Add-Member -Force -NotePropertyName provider_prior_terminal_statuses -NotePropertyValue @(
            $priorAttempts | ForEach-Object terminal_status
        )
        $request | Add-Member -Force -NotePropertyName input_tokens -NotePropertyValue $inputTokens
        $request | Add-Member -Force -NotePropertyName cached_input_tokens -NotePropertyValue $cachedTokens
        $request | Add-Member -Force -NotePropertyName output_tokens -NotePropertyValue $outputTokens
        $request | Add-Member -Force -NotePropertyName reasoning_output_tokens -NotePropertyValue $reasoningTokens
        $request | Add-Member -Force -NotePropertyName total_tokens -NotePropertyValue $totalTokens
        $request | Add-Member -Force -NotePropertyName cache_hit_rate -NotePropertyValue $cacheHitRate
        $resultWireIdentity = $null
        $newResults = @(
            $fact.final_control_result_identities |
                Where-Object { [int]$_.message_index -ge [int]$fact.lcp_message_count } |
                Sort-Object { [int]$_.message_index }
        )
        if ([bool]$request.final_control_result_before) {
            if ([int]$request.final_control_result_count -ne 1 -or $newResults.Count -ne 1) {
                throw "Provider request must carry exactly one settled final control result: $rolloutRequestId"
            }
            $resultWireIdentity = $newResults[0]
            $itemKind = [string](Get-R7JsonProperty $resultWireIdentity "item_kind" "")
            $callHash = [string](Get-R7JsonProperty $resultWireIdentity "control_call_id_sha256" "")
            $canonicalRevision = Get-R7JsonProperty $resultWireIdentity "canonical_revision"
            $settled = [bool](Get-R7JsonProperty $resultWireIdentity "settled" $false)
            if ([string]::IsNullOrWhiteSpace($itemKind) -or
                $callHash -notmatch '^[a-fA-F0-9]{64}$' -or
                $null -eq $canonicalRevision -or -not $settled) {
                throw "Provider final control result identity is incomplete: $rolloutRequestId"
            }
        } elseif ($newResults.Count) {
            throw "Wire trace has an unpaired final control result: $rolloutRequestId"
        }
        $request | Add-Member -Force -NotePropertyName final_control_result_item_kind -NotePropertyValue (
            [string](Get-R7JsonProperty $resultWireIdentity "item_kind" "")
        )
        $request | Add-Member -Force -NotePropertyName final_control_result_message_index -NotePropertyValue (
            Get-R7JsonProperty $resultWireIdentity "message_index"
        )
        $request | Add-Member -Force -NotePropertyName final_control_result_call_id_sha256 -NotePropertyValue (
            [string](Get-R7JsonProperty $resultWireIdentity "control_call_id_sha256" "")
        )
        $request | Add-Member -Force -NotePropertyName final_control_result_canonical_revision -NotePropertyValue (
            Get-R7JsonProperty $resultWireIdentity "canonical_revision"
        )
        $request | Add-Member -Force -NotePropertyName final_control_result_settled -NotePropertyValue (
            Get-R7JsonProperty $resultWireIdentity "settled"
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
        $classes[$name] = [int64]@(
            $RequestPath | Where-Object primary_failure_class -eq $name
        ).Count
    }
    $knownCount = Get-R7ExactInt64Sum @($classes.Values) "primary_failure_count"
    $unknown = @($RequestPath | Where-Object {
            [string]$_.primary_failure_class -notin @($classes.Keys)
        })
    $invalidEvidence = @($RequestPath | Where-Object evidence_health -ne "valid")
    $finalControlResult = @($RequestPath | Where-Object final_control_result_before -eq $true)
    $withoutFinalControlResult = @($RequestPath | Where-Object final_control_result_before -ne $true)
    $finalControlResultInput = Get-R7ExactInt64Sum @(
        $finalControlResult | ForEach-Object { $_.input_tokens }
    ) "input_tokens"
    $finalControlResultCached = Get-R7ExactInt64Sum @(
        $finalControlResult | ForEach-Object { $_.cached_input_tokens }
    ) "cached_input_tokens"
    $otherInput = Get-R7ExactInt64Sum @(
        $withoutFinalControlResult | ForEach-Object { $_.input_tokens }
    ) "input_tokens"
    $otherCached = Get-R7ExactInt64Sum @(
        $withoutFinalControlResult | ForEach-Object { $_.cached_input_tokens }
    ) "cached_input_tokens"
    [pscustomobject]@{
        provider_requests = [int64]$RequestPath.Count
        primary_failure_counts = [pscustomobject]$classes
        unknown_primary_failure_count = [int64]$unknown.Count
        evidence_health = if ($invalidEvidence.Count) { "invalid" } else { "valid" }
        invalid_evidence_request_count = [int64]$invalidEvidence.Count
        invalid_evidence_call_count = Get-R7ExactPropertyInt64Sum `
            $RequestPath "invalid_evidence_count" "request observability"
        classification_reconciled = (
            $unknown.Count -eq 0 -and
            $knownCount -eq $RequestPath.Count -and
            $invalidEvidence.Count -eq 0
        )
        sibling_failure_copy_count = Get-R7ExactPropertyInt64Sum `
            $RequestPath "sibling_failure_copy_count" "request observability"
        output_tokens = Get-R7ExactInt64Sum @($RequestPath.output_tokens) "output_tokens"
        reasoning_output_tokens = Get-R7ExactInt64Sum `
            @($RequestPath.reasoning_output_tokens) `
            "reasoning_output_tokens"
        total_tokens = Get-R7ExactInt64Sum @($RequestPath.total_tokens) "total_tokens"
        final_control_result_before_requests = [int64]$finalControlResult.Count
        final_control_result_before_input_tokens = $finalControlResultInput
        final_control_result_before_cached_input_tokens = $finalControlResultCached
        final_control_result_before_cache_hit_rate = if ($finalControlResultInput -gt 0) {
            [Math]::Round($finalControlResultCached / $finalControlResultInput, 6)
        } else {
            $null
        }
        no_final_control_result_before_requests = [int64]$withoutFinalControlResult.Count
        no_final_control_result_before_input_tokens = $otherInput
        no_final_control_result_before_cached_input_tokens = $otherCached
        no_final_control_result_before_cache_hit_rate = if ($otherInput -gt 0) {
            [Math]::Round($otherCached / $otherInput, 6)
        } else {
            $null
        }
        final_control_result_item_kinds = @(
            $finalControlResult | ForEach-Object final_control_result_item_kind |
                Where-Object { $_ } | Sort-Object -Unique
        )
        final_control_result_item_kind_unresolved_count = [int64]@(
            $finalControlResult | Where-Object {
                [string]::IsNullOrWhiteSpace([string]$_.final_control_result_item_kind)
            }
        ).Count
    }
}
