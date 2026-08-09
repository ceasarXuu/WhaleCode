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

function Test-R7NonnegativeJsonInteger {
    param($Value)
    $null -ne (ConvertTo-R7NonnegativeInt64Fact $Value)
}

function Get-R7WireRequestInventory {
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
                terminal_logical_request_id = ""
                attempt_seq = [int64]0
                terminal_attempt_seq = [int64]0
                request_index = [int64]0
                trace_schema = ""
                transport = ""
                terminal_transport = ""
                provider_wire_api = ""
                lcp_message_count = [int64]0
                message_shapes = @()
                final_control_result_identities = @()
                input_tokens = $null
                cached_input_tokens = $null
                output_tokens = $null
                reasoning_output_tokens = $null
                total_tokens = $null
                terminal_status = ""
                shape_event_count = [int64]0
                terminal_event_count = [int64]0
            }
        }
        $fact = $facts[$requestId]
        if ($null -ne (Get-R7JsonProperty $event "request_index")) {
            $fact.shape_event_count = [int64]$fact.shape_event_count + 1
            $fact.request_index = Get-R7RequiredNonnegativeInt64Fact `
                $event "request_index" "provider wire shape event"
            $fact.trace_schema = [string](Get-R7JsonProperty $event "schema_version" "")
            $fact.logical_request_id =
                [string](Get-R7JsonProperty $event "logical_request_id" "")
            $fact.attempt_seq = Get-R7RequiredNonnegativeInt64Fact `
                $event "attempt_seq" "provider wire shape event"
            $fact.transport = [string](Get-R7JsonProperty $event "transport" "")
            $fact.provider_wire_api = [string](Get-R7JsonProperty $event "provider_wire_api" "")
            $fact.lcp_message_count = Get-R7RequiredNonnegativeInt64Fact `
                $event "lcp_message_count" "provider wire shape event"
            $fact.message_shapes = @(Get-R7JsonProperty $event "message_shapes" @())
            $resultIdentity = Get-R7JsonProperty $event "taskspace_final_control_result_identity"
            $fact.final_control_result_identities = @(
                Get-R7JsonProperty $resultIdentity "results" @()
            )
        }
        if ([string](Get-R7JsonProperty $event "event_name" "") -eq "provider.chat_wire_request_terminal") {
            $fact.terminal_event_count = [int64]$fact.terminal_event_count + 1
            $fact.terminal_status = [string](Get-R7JsonProperty $event "status" "")
            $fact.terminal_logical_request_id =
                [string](Get-R7JsonProperty $event "logical_request_id" "")
            $fact.terminal_attempt_seq = Get-R7RequiredNonnegativeInt64Fact `
                $event "attempt_seq" "provider wire terminal event"
            $fact.terminal_transport = [string](Get-R7JsonProperty $event "transport" "")
            $fact.input_tokens = Get-R7JsonProperty $event "input_tokens"
            $fact.cached_input_tokens = Get-R7JsonProperty $event "cached_input_tokens"
            $fact.output_tokens = Get-R7JsonProperty $event "output_tokens"
            $fact.reasoning_output_tokens =
                Get-R7JsonProperty $event "reasoning_output_tokens"
            $fact.total_tokens = Get-R7JsonProperty $event "total_tokens"
        }
    }
    $unordered = @($facts.Values | ForEach-Object { [pscustomobject]$_ })
    $incomplete = @($unordered | Where-Object {
            [string]$_.trace_schema -ne "provider-chat-wire-trace-v11" -or
            [string]::IsNullOrWhiteSpace([string]$_.logical_request_id) -or
            [string]$_.logical_request_id -ne [string]$_.terminal_logical_request_id -or
            [string]::IsNullOrWhiteSpace([string]$_.transport) -or
            [string]$_.transport -ne [string]$_.terminal_transport -or
            -not (Test-R7NonnegativeJsonInteger $_.request_index) -or
            [int64]$_.request_index -lt 1 -or
            -not (Test-R7NonnegativeJsonInteger $_.attempt_seq) -or
            [int64]$_.attempt_seq -lt 1 -or
            -not (Test-R7NonnegativeJsonInteger $_.terminal_attempt_seq) -or
            [int64]$_.attempt_seq -ne [int64]$_.terminal_attempt_seq -or
            [int64]$_.shape_event_count -ne 1 -or
            [int64]$_.terminal_event_count -ne 1 -or
            [string]$_.terminal_status -notin @(
                "response_completed", "response_failed", "cancelled", "retry_unauthorized"
            ) -or
            (
                [string]$_.terminal_status -eq "response_completed" -and
                (
                    -not (Test-R7NonnegativeJsonInteger $_.input_tokens) -or
                    -not (Test-R7NonnegativeJsonInteger $_.cached_input_tokens) -or
                    -not (Test-R7NonnegativeJsonInteger $_.output_tokens) -or
                    -not (Test-R7NonnegativeJsonInteger $_.reasoning_output_tokens) -or
                    -not (Test-R7NonnegativeJsonInteger $_.total_tokens) -or
                    [int64]$_.cached_input_tokens -gt [int64]$_.input_tokens -or
                    [int64]$_.reasoning_output_tokens -gt [int64]$_.output_tokens -or
                    [bigint]$_.total_tokens -ne
                        ([bigint]$_.input_tokens + [bigint]$_.output_tokens)
                )
            )
        })
    if ($incomplete.Count) {
        throw "Provider wire trace has $($incomplete.Count) incomplete physical request rows"
    }
    $ordered = @($unordered | Sort-Object { [int64]$_.request_index })
    $expectedIndexes = @(1..$ordered.Count)
    $actualIndexes = @($ordered | ForEach-Object { [int64]$_.request_index })
    if ($ordered.Count -eq 0 -or
        (Compare-Object $expectedIndexes $actualIndexes -SyncWindow 0)) {
        throw "Provider wire physical request indexes are missing, duplicated, or reordered"
    }
    foreach ($group in @($ordered | Group-Object logical_request_id)) {
        $attempts = @($group.Group | Sort-Object { [int64]$_.request_index })
        $expectedAttempts = @(1..$attempts.Count)
        $actualAttempts = @($attempts | ForEach-Object { [int64]$_.attempt_seq })
        if (Compare-Object $expectedAttempts $actualAttempts -SyncWindow 0) {
            throw "Provider wire logical request attempts are missing, duplicated, or reordered: $($group.Name)"
        }
        $completed = @($attempts | Where-Object terminal_status -eq "response_completed")
        if ($completed.Count -gt 1) {
            throw "Provider wire logical request has multiple completed attempts: $($group.Name)"
        }
        if ($completed.Count -eq 1 -and
            [int64]$completed[0].attempt_seq -ne [int64]$attempts[-1].attempt_seq) {
            throw "Provider wire logical request completed before its final attempt: $($group.Name)"
        }
    }
    $ordered
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
        [int64]$ExpectedProviderAttempts = 0
    )
    $inventory = @(Get-R7WireRequestInventory $WireTracePath)
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
